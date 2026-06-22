#!/usr/bin/env ruby
# frozen_string_literal: true

require "fileutils"
require "json"
require "open3"
require "tempfile"
require "tmpdir"

ROOT = File.expand_path("..", __dir__)

def fail_with(message)
  warn "evidence script validation failed: #{message}"
  exit 1
end

def run_command(*command)
  stdout, stderr, status = Open3.capture3(*command, chdir: ROOT)
  output = [stdout, stderr].reject(&:empty?).join("\n")
  return output if status.success?

  fail_with("#{command.join(" ")} failed with #{status.exitstatus}:\n#{output}")
end

def assert_includes(text, expected, message)
  return if text.include?(expected)

  fail_with("#{message}; missing #{expected.inspect}")
end

def refute_match(text, pattern, message)
  return unless text.match?(pattern)

  fail_with(message)
end

def with_json_file(value)
  file = Tempfile.new(["keyplane-evidence-fixture", ".json"])
  file.write(JSON.pretty_generate(value))
  file.close
  yield file.path
ensure
  file&.unlink
end

run_command("ruby", "scripts/validate-keypeek-live-hardware.rb", "--dry-run")
run_command("ruby", "scripts/collect-signed-release-evidence.rb", "--dry-run")
run_command("ruby", "scripts/collect-mvp-acceptance-evidence.rb", "--dry-run")

with_json_file([{ "vid" => "feed", "pid" => "cafe", "label" => "Fixture VIA Raw HID" }]) do |devices_path|
  run_command(
    "ruby",
    "scripts/validate-keypeek-live-hardware.rb",
    "--dry-run",
    "--devices-json",
    devices_path
  )
end
keypeek_report = File.read(File.join(ROOT, "target/keyplane-validation/keypeek-live-hardware.md"))
assert_includes(
  keypeek_report,
  "Fixture VIA Raw HID",
  "KeyPeek fixture device label should appear in the generated report"
)
refute_match(
  keypeek_report,
  /feed|cafe/i,
  "KeyPeek fixture USB IDs should be redacted from the generated report"
)

signed_report_dir = Dir.mktmpdir("keyplane-signed-release-evidence")
begin
  run_id = 123_456_789
  with_json_file([
    {
      "databaseId" => 987_654_321,
      "status" => "in_progress",
      "conclusion" => nil,
      "workflowName" => "Signed Release"
    },
    {
      "databaseId" => run_id,
      "status" => "completed",
      "conclusion" => "success",
      "workflowName" => "Signed Release"
    }
  ]) do |runs_path|
    with_json_file(
      {
        "attempt" => 1,
        "conclusion" => "success",
        "databaseId" => run_id,
        "event" => "workflow_dispatch",
        "headSha" => "abc123",
        "jobs" => [
          {
            "name" => "Signed macOS release",
            "status" => "completed",
            "conclusion" => "success",
            "url" => "https://example.test/job"
          }
        ],
        "status" => "completed",
        "updatedAt" => "2026-06-22T00:00:00Z",
        "url" => "https://example.test/run",
        "workflowName" => "Signed Release"
      }
    ) do |run_path|
      with_json_file(
        {
          "total_count" => 2,
          "artifacts" => [
            {
              "name" => "keyplane-macos-signed-app",
              "expired" => false,
              "size_in_bytes" => 42
            },
            {
              "name" => "keyplane-macos-signed-dmg",
              "expired" => false,
              "size_in_bytes" => 43
            }
          ]
        }
      ) do |artifacts_path|
        run_command(
          "ruby",
          "scripts/collect-signed-release-evidence.rb",
          "--runs-json",
          runs_path,
          "--run-json",
          run_path,
          "--artifacts-json",
          artifacts_path,
          "--report-dir",
          signed_report_dir
        )
      end
    end
  end

  signed_report = File.read(File.join(signed_report_dir, "signed-release.md"))
  assert_includes(signed_report, "- Run id: #{run_id}", "signed-release fixture run id should be selected")
  assert_includes(
    signed_report,
    "- No evidence gaps found.",
    "signed-release fixture should satisfy report evidence shape"
  )
ensure
  FileUtils.remove_entry(signed_report_dir) if signed_report_dir && Dir.exist?(signed_report_dir)
end

puts "evidence script validation passed"
