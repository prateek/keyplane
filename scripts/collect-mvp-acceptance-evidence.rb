#!/usr/bin/env ruby
# frozen_string_literal: true

require "fileutils"
require "open3"
require "optparse"
require "shellwords"
require "time"

ROOT = File.expand_path("..", __dir__)
DEFAULT_REPORT_DIR = File.join(ROOT, "target/keyplane-validation")
REPORT_NAME = "mvp-acceptance.md"
KEYPEEK_REPORT = File.join(DEFAULT_REPORT_DIR, "keypeek-live-hardware.md")
SIGNED_RELEASE_REPORT = File.join(DEFAULT_REPORT_DIR, "signed-release.md")

Result = Struct.new(
  :label,
  :status,
  :details,
  :command,
  :exit_status,
  :elapsed_seconds,
  :output,
  keyword_init: true
)

def fail_with(message)
  warn "MVP acceptance evidence collection failed: #{message}"
  exit 1
end

def parse_options
  options = {
    dry_run: false,
    run_local_commands: true,
    report_dir: ENV.fetch("KEYPLANE_MVP_ACCEPTANCE_REPORT_DIR", DEFAULT_REPORT_DIR),
    keypeek_report: ENV.fetch("KEYPLANE_KEYPEEK_LIVE_REPORT_PATH", KEYPEEK_REPORT),
    signed_release_report: ENV.fetch("KEYPLANE_SIGNED_RELEASE_REPORT_PATH", SIGNED_RELEASE_REPORT)
  }

  OptionParser.new do |parser|
    parser.banner = "Usage: collect-mvp-acceptance-evidence.rb [options]"
    parser.on("--dry-run", "Write a sample report without running local checks or requiring external evidence") do
      options[:dry_run] = true
    end
    parser.on("--skip-local-commands", "Inspect existing external reports without rerunning local verification") do
      options[:run_local_commands] = false
    end
    parser.on("--keypeek-report PATH", "Path to KeyPeek Live hardware report") do |path|
      options[:keypeek_report] = path
    end
    parser.on("--signed-release-report PATH", "Path to signed-release evidence report") do |path|
      options[:signed_release_report] = path
    end
    parser.on("--report-dir DIR", "Directory for the generated MVP acceptance report") do |dir|
      options[:report_dir] = dir
    end
  end.parse!

  options
end

def report_path(report_dir)
  File.join(File.expand_path(report_dir, ROOT), REPORT_NAME)
end

def redaction_tokens
  %w[
    KEYPLANE_LOCAL_VIL_CANDIDATE
    KEYPLANE_KEYPEEK_LIVE_VID
    KEYPLANE_KEYPEEK_LIVE_PID
    KEYPLANE_SIGNED_RELEASE_RUN_ID
  ].filter_map { |name| ENV[name].to_s.strip }
   .flat_map { |value| [value, value.delete_prefix("0x").delete_prefix("0X")] }
   .reject(&:empty?)
   .uniq
end

def redact(text)
  redaction_tokens.reduce(text.to_s) do |redacted, token|
    redacted.gsub(/#{Regexp.escape(token)}/i, "[redacted]")
  end
end

def dry_result(label, details)
  Result.new(
    label: label,
    status: "dry-run",
    details: details,
    command: nil,
    exit_status: 0,
    elapsed_seconds: 0.0,
    output: "Dry run only. This is sample evidence shape, not acceptance proof."
  )
end

def gap_result(label, details)
  Result.new(
    label: label,
    status: "gap",
    details: details,
    command: nil,
    exit_status: nil,
    elapsed_seconds: 0.0,
    output: nil
  )
end

def run_command_result(label, command, details:, chdir: ROOT, env: ENV.to_h)
  started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  stdout, stderr, status = Open3.capture3(env, *command, chdir: chdir)
  elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started
  output = [stdout, stderr].reject(&:empty?).join("\n")

  Result.new(
    label: label,
    status: status.success? ? "passed" : "failed",
    details: details,
    command: command,
    exit_status: status.exitstatus,
    elapsed_seconds: elapsed.round(2),
    output: redact(output)
  )
end

def report_file_result(label, path, details:, required_patterns:)
  expanded = File.expand_path(path, ROOT)
  return gap_result(label, "#{details}; missing #{expanded}") unless File.exist?(expanded)

  contents = File.read(expanded)
  missing = required_patterns.filter_map do |description, pattern|
    description unless contents.match?(pattern)
  end

  Result.new(
    label: label,
    status: missing.empty? ? "passed" : "gap",
    details: missing.empty? ? "#{details}; accepted #{expanded}" : "#{details}; missing #{missing.join(", ")} in #{expanded}",
    command: nil,
    exit_status: nil,
    elapsed_seconds: 0.0,
    output: missing.empty? ? "Evidence report accepted." : redact(contents.lines.last(80).join.rstrip)
  )
end

def local_results(dry:, run_local_commands:)
  if dry
    return [
      dry_result("Static release/capability validators", "Validates release workflow and Tauri capability drift checks"),
      dry_result("Rust domain and backend tests", "Covers Keyboard Snapshot composition, Runtime Events, imports, EDN, Backend Health, and overlay plans"),
      dry_result("Frontend contract tests", "Covers Overlay Surface, Import Review, Source Inspector, Settings, and Runtime Event rendering"),
      dry_result("Tauri debug no-bundle build", "Exercises the desktop build path without bundling installers"),
      dry_result("NocFree/Vial private import canary", "Checks one local .vil import when KEYPLANE_LOCAL_VIL_CANDIDATE is supplied")
    ]
  end

  unless run_local_commands
    return [
      gap_result("Local automated verification", "Skipped by --skip-local-commands; local PRD proof is not refreshed in this report")
    ]
  end

  results = []
  results << run_command_result(
    "Static release/capability validators",
    %w[npm run check:workflows],
    details: "Validates release workflow and Tauri capability drift checks"
  )
  results << run_command_result(
    "Rust domain and backend tests",
    %w[cargo test --manifest-path src-tauri/Cargo.toml],
    details: "Covers Keyboard Snapshot composition, Runtime Events, imports, EDN, Backend Health, and overlay plans"
  )
  results << run_command_result(
    "Frontend contract tests",
    %w[npm test],
    details: "Covers Overlay Surface, Import Review, Source Inspector, Settings, and Runtime Event rendering"
  )
  results << run_command_result(
    "Tauri debug no-bundle build",
    %w[npm run tauri build -- --debug --no-bundle],
    details: "Exercises the desktop build path without bundling installers"
  )

  if ENV["KEYPLANE_LOCAL_VIL_CANDIDATE"].to_s.strip.empty?
    results << gap_result(
      "NocFree/Vial private import canary",
      "Set KEYPLANE_LOCAL_VIL_CANDIDATE to a local sanitized or private .vil export to prove the NocFree/Vial acceptance path"
    )
  else
    results << run_command_result(
      "NocFree/Vial private import canary",
      %w[cargo test --manifest-path src-tauri/Cargo.toml local_vil_candidate_file_imports_when_env_is_set -- --ignored --nocapture],
      details: "Checks one local .vil import without committing the private export"
    )
  end

  results
end

def external_results(options)
  if options[:dry_run]
    return [
      dry_result("KeyPeek-backed live layer-change evidence", "Requires the KeyPeek Live hardware report from a real layer-change canary"),
      dry_result("Signed macOS release evidence", "Requires a Signed Release GitHub Actions run with Apple credentials")
    ]
  end

  [
    report_file_result(
      "KeyPeek-backed live layer-change evidence",
      options[:keypeek_report],
      details: "Requires a real KeyPeek-compatible Raw HID subscription and observed Layer Stack Runtime Event",
      required_patterns: [
        ["hardware-canary mode", /^- Mode: hardware canary$/],
        ["passed result", /^Passed\. Subscription and layer-change Runtime Event canaries completed/m],
        ["layer-change section", /^## Layer-change Runtime Event canary$/],
        ["passed layer-change canary status", /^## Layer-change Runtime Event canary$[\s\S]*?^- Status: passed$/]
      ]
    ),
    report_file_result(
      "Signed macOS release evidence",
      options[:signed_release_report],
      details: "Requires a real Signed Release workflow run with Apple credentials and signed artifacts",
      required_patterns: [
        ["GitHub Actions evidence mode", /^- Mode: GitHub Actions run evidence$/],
        ["passed result", /^Passed\. The signed release workflow completed successfully/m],
        ["no evidence gaps", /^- No evidence gaps found\.$/]
      ]
    )
  ]
end

def findings(results)
  results.reject { |result| result.status == "passed" }.map do |result|
    "#{result.label}: #{result.status} - #{result.details}"
  end
end

def append_output(report, result)
  output = result.output.to_s.lines.last(80).join.rstrip
  return if output.empty?

  report << "\n````text\n"
  report << output
  report << "\n````\n"
end

def build_report(results, dry:)
  gaps = dry ? [] : findings(results)

  report = +"# Keyplane MVP Acceptance Evidence Report\n\n"
  report << "- Generated: #{Time.now.utc.iso8601}\n"
  report << "- Mode: #{dry ? "dry run" : "acceptance evidence"}\n"
  report << "- Result: "
  if dry
    report << "dry run only; no acceptance gate is proven\n"
  elsif gaps.empty?
    report << "passed\n"
  else
    report << "failed with #{gaps.length} evidence gap(s)\n"
  end

  report << "\n## Findings\n\n"
  if dry
    report << "- Dry run only. This report validates the evidence shape but is not acceptance proof.\n"
  elsif gaps.empty?
    report << "- No evidence gaps found.\n"
  else
    gaps.each { |gap| report << "- #{gap}\n" }
  end

  report << "\n## Evidence\n"
  results.each do |result|
    report << "\n### #{result.label}\n\n"
    report << "- Status: #{result.status}\n"
    report << "- Details: #{result.details}\n"
    report << "- Exit status: #{result.exit_status}\n" unless result.exit_status.nil?
    report << "- Elapsed: #{result.elapsed_seconds}s\n" if result.elapsed_seconds&.positive?
    report << "- Command: `#{result.command.shelljoin}`\n" if result.command
    append_output(report, result)
    report << "\n"
  end

  report << "## Acceptance Notes\n\n"
  report << "- The KeyPeek live gate requires a hardware canary report with an observed Layer Stack Runtime Event from a real KeyPeek-compatible device.\n"
  report << "- The signed-release gate requires a real `Signed Release` GitHub Actions run with Apple credentials and signed `.app` and `.dmg` artifacts.\n"
  report << "- Private `.vil` paths, USB IDs, and release run IDs are redacted from command output.\n"
  report
end

options = parse_options
results = local_results(dry: options[:dry_run], run_local_commands: options[:run_local_commands]) +
          external_results(options)

path = report_path(options[:report_dir])
FileUtils.mkdir_p(File.dirname(path))
File.write(path, build_report(results, dry: options[:dry_run]))

gaps = findings(results)
puts "MVP acceptance evidence report written to #{path}"
warn "MVP acceptance has #{gaps.length} evidence gap(s); see #{path}" if !options[:dry_run] && !gaps.empty?
exit(options[:dry_run] || gaps.empty? ? 0 : 1)
