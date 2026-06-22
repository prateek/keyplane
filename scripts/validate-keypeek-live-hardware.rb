#!/usr/bin/env ruby
# frozen_string_literal: true

require "fileutils"
require "json"
require "open3"
require "optparse"
require "shellwords"
require "time"

ROOT = File.expand_path("..", __dir__)
SRC_TAURI = File.join(ROOT, "src-tauri")
DEFAULT_REPORT_DIR = File.join(ROOT, "target/keyplane-validation")
REPORT_NAME = "keypeek-live-hardware.md"
REQUIRED_ENV = %w[
  KEYPLANE_KEYPEEK_LIVE_VID
  KEYPLANE_KEYPEEK_LIVE_PID
].freeze

Canary = Struct.new(:label, :test_name, keyword_init: true)
Result = Struct.new(:label, :test_name, :status, :exit_status, :output, :elapsed_seconds, keyword_init: true)

CANARIES = [
  Canary.new(
    label: "Raw HID subscription canary",
    test_name: "local_keypeek_live_device_accepts_subscription_when_env_is_set"
  ),
  Canary.new(
    label: "Layer-change Runtime Event canary",
    test_name: "local_keypeek_live_device_emits_layer_change_when_env_is_set"
  )
].freeze

def fail_with(message)
  warn "KeyPeek live hardware validation failed: #{message}"
  exit 1
end

def parse_options
  options = {
    dry_run: ENV["KEYPLANE_KEYPEEK_LIVE_DRY_RUN"] == "1"
  }

  OptionParser.new do |parser|
    parser.banner = "Usage: validate-keypeek-live-hardware.rb [options]"
    parser.on("--dry-run", "Write a sample report without opening hardware") do
      options[:dry_run] = true
    end
    parser.on("--devices-json PATH", "Read discovered VIA Raw HID device JSON from a file") do |path|
      options[:devices_json_path] = path
    end
  end.parse!

  options
end

def load_json_file(path)
  JSON.parse(File.read(path))
rescue JSON::ParserError => e
  fail_with("#{path} is not valid JSON: #{e.message}")
end

def discover_devices_json
  stdout, stderr, status = Open3.capture3(
    "cargo",
    "run",
    "--quiet",
    "--manifest-path",
    File.join(SRC_TAURI, "Cargo.toml"),
    "--example",
    "discover_keypeek_devices",
    chdir: ROOT
  )
  return JSON.parse(stdout) if status.success?

  fail_with("could not discover VIA Raw HID devices: #{stderr.strip}")
rescue JSON::ParserError => e
  fail_with("VIA Raw HID discovery returned invalid JSON: #{e.message}")
end

def discovery_devices(options)
  return load_json_file(options[:devices_json_path]) if options[:devices_json_path]

  discover_devices_json
end

def env_has_usb_ids?
  REQUIRED_ENV.all? { |name| ENV[name].to_s.strip != "" }
end

def ensure_or_discover_env!(options)
  return if env_has_usb_ids?

  devices = discovery_devices(options)
  fail_with("VIA Raw HID discovery expected a JSON array") unless devices.is_a?(Array)

  case devices.length
  when 0
    fail_with(
      "missing #{REQUIRED_ENV.join(", ")} and no VIA Raw HID devices were discovered; " \
      "connect a KeyPeek-compatible keyboard or set VID/PID explicitly"
    )
  when 1
    device = devices.first
    vid = device["vid"].to_s
    pid = device["pid"].to_s
    fail_with("discovered VIA Raw HID device was missing vid or pid") if vid.empty? || pid.empty?

    ENV["KEYPLANE_KEYPEEK_LIVE_VID"] = vid
    ENV["KEYPLANE_KEYPEEK_LIVE_PID"] = pid
    if ENV["KEYPLANE_KEYPEEK_LIVE_DEVICE_LABEL"].to_s.strip == "" && device["label"]
      ENV["KEYPLANE_KEYPEEK_LIVE_DEVICE_LABEL"] = device["label"].to_s
    end
  else
    labels = devices.map { |device| "#{device["label"]} #{device["vid"]}:#{device["pid"]}" }
    fail_with(
      "discovered multiple VIA Raw HID devices; set KEYPLANE_KEYPEEK_LIVE_VID and " \
      "KEYPLANE_KEYPEEK_LIVE_PID for the intended KeyPeek-compatible keyboard: #{labels.join("; ")}"
    )
  end
end

def redaction_tokens
  REQUIRED_ENV
    .filter_map { |name| ENV[name].to_s.strip }
    .flat_map { |value| [value, value.delete_prefix("0x").delete_prefix("0X")] }
    .reject(&:empty?)
    .uniq
end

def redact(text)
  redaction_tokens.reduce(text.to_s) do |redacted, token|
    redacted.gsub(/#{Regexp.escape(token)}/i, "[redacted-usb-id]")
  end
end

def command_for(test_name)
  ["cargo", "test", test_name, "--", "--ignored"]
end

def run_canary(canary)
  started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  stdout, stderr, status = Open3.capture3(
    ENV.to_h,
    *command_for(canary.test_name),
    chdir: SRC_TAURI
  )
  elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started
  output = [stdout, stderr].reject(&:empty?).join("\n")

  Result.new(
    label: canary.label,
    test_name: canary.test_name,
    status: status.success? ? "passed" : "failed",
    exit_status: status.exitstatus,
    output: redact(output),
    elapsed_seconds: elapsed.round(2)
  )
end

def dry_result(canary)
  Result.new(
    label: canary.label,
    test_name: canary.test_name,
    status: "dry-run",
    exit_status: 0,
    output: "Dry run only. No HID device was opened and no Runtime Event was observed.",
    elapsed_seconds: 0.0
  )
end

def report_dir
  File.expand_path(ENV.fetch("KEYPLANE_KEYPEEK_LIVE_REPORT_DIR", DEFAULT_REPORT_DIR), ROOT)
end

def report_path
  File.join(report_dir, REPORT_NAME)
end

def append_output(report, output)
  trimmed = output.to_s.lines.last(120).join.rstrip
  return report << "_No command output captured._\n\n" if trimmed.empty?

  report << "```text\n"
  report << trimmed
  report << "\n```\n\n"
end

def build_report(results, dry:)
  report = +"# KeyPeek Live Hardware Validation Report\n\n"
  report << "- Generated: #{Time.now.utc.iso8601}\n"
  report << "- Mode: #{dry ? "dry run" : "hardware canary"}\n"
  report << "- VID/PID: #{dry ? "not required for dry run" : "provided and redacted"}\n"
  if ENV["KEYPLANE_KEYPEEK_LIVE_WAIT_MS"].to_s.strip != ""
    report << "- Layer-change wait: #{ENV["KEYPLANE_KEYPEEK_LIVE_WAIT_MS"]} ms\n"
  end
  if ENV["KEYPLANE_KEYPEEK_LIVE_DEVICE_LABEL"].to_s.strip != ""
    report << "- Device label: #{redact(ENV["KEYPLANE_KEYPEEK_LIVE_DEVICE_LABEL"])}\n"
  end
  if ENV["KEYPLANE_KEYPEEK_LIVE_FIRMWARE_REF"].to_s.strip != ""
    report << "- Firmware reference: #{redact(ENV["KEYPLANE_KEYPEEK_LIVE_FIRMWARE_REF"])}\n"
  end

  report << "\n## Result\n\n"
  if dry
    report << "Dry run only. This report does not validate a KeyPeek-compatible device.\n\n"
  elsif results.all? { |result| result.status == "passed" }
    report << "Passed. Subscription and layer-change Runtime Event canaries completed against the configured Raw HID device.\n\n"
  else
    report << "Failed. At least one canary did not complete; see command output below.\n\n"
  end

  results.each do |result|
    report << "## #{result.label}\n\n"
    report << "- Test: `#{result.test_name}`\n"
    report << "- Status: #{result.status}\n"
    report << "- Exit status: #{result.exit_status}\n"
    report << "- Elapsed: #{result.elapsed_seconds}s\n"
    report << "- Command: `#{command_for(result.test_name).shelljoin}`\n\n"
    append_output(report, result.output)
  end

  report << "## Acceptance Notes\n\n"
  report << "- The layer-change canary is the authoritative hardware proof because it observes a real KeyPeek Layer Stack Runtime Event.\n"
  report << "- Run the manual Overlay Window check from `docs/validation/keypeek-live-hardware.md` after these canaries pass.\n"
  report << "- This report intentionally redacts USB IDs so generated evidence can be shared without exposing local device identifiers.\n"
  report
end

options = parse_options
dry = options[:dry_run]
ensure_or_discover_env!(options) if !dry || options[:devices_json_path]

results = dry ? CANARIES.map { |canary| dry_result(canary) } : CANARIES.map { |canary| run_canary(canary) }

FileUtils.mkdir_p(report_dir)
File.write(report_path, build_report(results, dry: dry))
puts "KeyPeek live hardware validation report written to #{report_path}"

exit(results.all? { |result| %w[passed dry-run].include?(result.status) } ? 0 : 1)
