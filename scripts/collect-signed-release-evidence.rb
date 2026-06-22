#!/usr/bin/env ruby
# frozen_string_literal: true

require "fileutils"
require "json"
require "open3"
require "optparse"
require "shellwords"
require "time"

ROOT = File.expand_path("..", __dir__)
DEFAULT_REPORT_DIR = File.join(ROOT, "target/keyplane-validation")
REPORT_NAME = "signed-release.md"
DEFAULT_REPO = "prateek/keyplane"
WORKFLOW_NAME = "Signed Release"
JOB_NAME = "Signed macOS release"
REQUIRED_ARTIFACTS = %w[
  keyplane-macos-signed-app
  keyplane-macos-signed-dmg
].freeze

def fail_with(message)
  warn "signed release evidence collection failed: #{message}"
  exit 1
end

def parse_options
  options = {
    dry_run: false,
    repo: ENV.fetch("KEYPLANE_GITHUB_REPO", DEFAULT_REPO),
    report_dir: ENV.fetch("KEYPLANE_SIGNED_RELEASE_REPORT_DIR", DEFAULT_REPORT_DIR),
    run_id: ENV["KEYPLANE_SIGNED_RELEASE_RUN_ID"]
  }

  OptionParser.new do |parser|
    parser.banner = "Usage: collect-signed-release-evidence.rb [options]"
    parser.on("--dry-run", "Write a sample report without querying GitHub") do
      options[:dry_run] = true
    end
    parser.on("--repo REPO", "GitHub repository, default #{DEFAULT_REPO}") do |repo|
      options[:repo] = repo
    end
    parser.on("--run-id RUN_ID", "GitHub Actions run id for the signed release") do |run_id|
      options[:run_id] = run_id
    end
    parser.on("--runs-json PATH", "Read gh run list JSON from a file for latest-run discovery") do |path|
      options[:runs_json_path] = path
    end
    parser.on("--run-json PATH", "Read gh run JSON from a file instead of gh") do |path|
      options[:run_json_path] = path
    end
    parser.on("--artifacts-json PATH", "Read run artifacts JSON from a file instead of gh api") do |path|
      options[:artifacts_json_path] = path
    end
    parser.on("--report-dir DIR", "Directory for the generated evidence report") do |dir|
      options[:report_dir] = dir
    end
  end.parse!

  options
end

def run_command(*command)
  stdout, stderr, status = Open3.capture3(*command)
  return stdout if status.success?

  fail_with("#{command.shelljoin} failed with #{status.exitstatus}: #{stderr.strip}")
end

def load_json_file(path)
  JSON.parse(File.read(path))
rescue JSON::ParserError => e
  fail_with("#{path} is not valid JSON: #{e.message}")
end

def github_run_json(repo, run_id)
  JSON.parse(
    run_command(
      "gh",
      "run",
      "view",
      run_id,
      "--repo",
      repo,
      "--json",
      "attempt,conclusion,databaseId,event,headSha,jobs,name,status,updatedAt,url,workflowName"
    )
  )
end

def github_runs_json(repo)
  stdout, stderr, status = Open3.capture3(
    "gh",
    "run",
    "list",
    "--repo",
    repo,
    "--workflow",
    WORKFLOW_NAME,
    "--all",
    "--limit",
    "20",
    "--json",
    "conclusion,createdAt,databaseId,event,headSha,status,url,workflowName"
  )
  return JSON.parse(stdout) if status.success?

  fail_with(
    "could not discover the latest #{WORKFLOW_NAME.inspect} run: #{stderr.strip}. " \
    "Pass --run-id or KEYPLANE_SIGNED_RELEASE_RUN_ID after the workflow exists on the repository default branch."
  )
rescue JSON::ParserError => e
  fail_with("gh run list returned invalid JSON: #{e.message}")
end

def github_artifacts_json(repo, run_id)
  JSON.parse(run_command("gh", "api", "repos/#{repo}/actions/runs/#{run_id}/artifacts"))
end

def latest_signed_release_run_id(repo, runs_json_path)
  runs = runs_json_path ? load_json_file(runs_json_path) : github_runs_json(repo)
  fail_with("latest-run discovery expected a JSON array") unless runs.is_a?(Array)

  completed_run = runs.find { |run| run["status"] == "completed" }
  unless completed_run
    fail_with(
      "no completed #{WORKFLOW_NAME.inspect} run found. " \
      "Run the signed-release workflow with real Apple credentials, or pass --run-id for the run to inspect."
    )
  end

  run_id = completed_run["databaseId"].to_s
  fail_with("latest #{WORKFLOW_NAME.inspect} run did not include a databaseId") if run_id.strip.empty?

  run_id
end

def sample_run_json
  {
    "attempt" => 1,
    "conclusion" => "success",
    "databaseId" => 0,
    "event" => "workflow_dispatch",
    "headSha" => "dry-run",
    "jobs" => [
      {
        "name" => JOB_NAME,
        "status" => "completed",
        "conclusion" => "success",
        "url" => "https://github.com/#{DEFAULT_REPO}/actions/runs/0/job/0"
      }
    ],
    "name" => WORKFLOW_NAME,
    "status" => "completed",
    "updatedAt" => Time.now.utc.iso8601,
    "url" => "https://github.com/#{DEFAULT_REPO}/actions/runs/0",
    "workflowName" => WORKFLOW_NAME
  }
end

def sample_artifacts_json
  {
    "total_count" => REQUIRED_ARTIFACTS.length,
    "artifacts" => REQUIRED_ARTIFACTS.map do |name|
      {
        "name" => name,
        "expired" => false,
        "size_in_bytes" => 1,
        "archive_download_url" => "https://api.github.com/repos/#{DEFAULT_REPO}/actions/artifacts/0/zip"
      }
    end
  }
end

def artifacts_list(artifacts_json)
  artifacts_json.fetch("artifacts", [])
end

def validation_findings(run_json, artifacts_json)
  findings = []
  workflow_name = run_json["workflowName"] || run_json["name"]
  findings << "workflow was #{workflow_name.inspect}, expected #{WORKFLOW_NAME.inspect}" unless workflow_name == WORKFLOW_NAME
  findings << "run status was #{run_json["status"].inspect}, expected \"completed\"" unless run_json["status"] == "completed"
  findings << "run conclusion was #{run_json["conclusion"].inspect}, expected \"success\"" unless run_json["conclusion"] == "success"

  job = run_json.fetch("jobs", []).find { |candidate| candidate["name"] == JOB_NAME }
  if job.nil?
    findings << "missing job #{JOB_NAME.inspect}"
  else
    findings << "#{JOB_NAME.inspect} status was #{job["status"].inspect}, expected \"completed\"" unless job["status"] == "completed"
    findings << "#{JOB_NAME.inspect} conclusion was #{job["conclusion"].inspect}, expected \"success\"" unless job["conclusion"] == "success"
  end

  artifacts = artifacts_list(artifacts_json)
  artifact_names = artifacts.map { |artifact| artifact["name"] }
  REQUIRED_ARTIFACTS.each do |name|
    artifact = artifacts.find { |candidate| candidate["name"] == name }
    if artifact.nil?
      findings << "missing signed artifact #{name.inspect}"
    elsif artifact["expired"]
      findings << "signed artifact #{name.inspect} is expired"
    elsif artifact["size_in_bytes"].to_i <= 0
      findings << "signed artifact #{name.inspect} has empty size"
    end
  end

  unexpected_keyplane_artifacts = artifact_names.grep(/keyplane.*signed/i) - REQUIRED_ARTIFACTS
  unless unexpected_keyplane_artifacts.empty?
    findings << "unexpected signed Keyplane artifact names: #{unexpected_keyplane_artifacts.join(", ")}"
  end

  findings
end

def report_path(report_dir)
  File.join(File.expand_path(report_dir, ROOT), REPORT_NAME)
end

def build_report(run_json, artifacts_json, findings, dry:)
  artifacts = artifacts_list(artifacts_json)
  job = run_json.fetch("jobs", []).find { |candidate| candidate["name"] == JOB_NAME }

  report = +"# Signed Release Evidence Report\n\n"
  report << "- Generated: #{Time.now.utc.iso8601}\n"
  report << "- Mode: #{dry ? "dry run" : "GitHub Actions run evidence"}\n"
  report << "- Workflow: #{run_json["workflowName"] || run_json["name"]}\n"
  report << "- Run id: #{run_json["databaseId"]}\n"
  report << "- Run URL: #{run_json["url"]}\n"
  report << "- Event: #{run_json["event"]}\n"
  report << "- Head SHA: #{run_json["headSha"]}\n"
  report << "- Run status: #{run_json["status"]}\n"
  report << "- Run conclusion: #{run_json["conclusion"]}\n"
  report << "- Attempt: #{run_json["attempt"]}\n"
  report << "- Updated: #{run_json["updatedAt"]}\n"

  report << "\n## Result\n\n"
  if dry
    report << "Dry run only. This report does not prove a signed release was executed.\n\n"
  elsif findings.empty?
    report << "Passed. The signed release workflow completed successfully and uploaded the required signed macOS artifacts.\n\n"
  else
    report << "Failed. The run is not acceptable signed-release evidence yet.\n\n"
  end

  report << "## Job\n\n"
  if job
    report << "- Name: #{job["name"]}\n"
    report << "- Status: #{job["status"]}\n"
    report << "- Conclusion: #{job["conclusion"]}\n"
    report << "- URL: #{job["url"]}\n\n"
  else
    report << "- Missing #{JOB_NAME.inspect}\n\n"
  end

  report << "## Artifacts\n\n"
  if artifacts.empty?
    report << "_No artifacts returned for this run._\n\n"
  else
    artifacts.each do |artifact|
      report << "- #{artifact["name"]}: size=#{artifact["size_in_bytes"]}, expired=#{artifact["expired"]}\n"
    end
    report << "\n"
  end

  report << "## Findings\n\n"
  if dry && findings.empty?
    report << "- Sample evidence shape had no gaps. This is still not real release evidence.\n\n"
  elsif findings.empty?
    report << "- No evidence gaps found.\n\n"
  else
    findings.each { |finding| report << "- #{finding}\n" }
    report << "\n"
  end

  report << "## Acceptance Notes\n\n"
  report << "- This report checks workflow success, the signed macOS release job, and the signed `.app` and `.dmg` artifact records.\n"
  report << "- It does not download artifacts or inspect Apple notarization logs.\n"
  report << "- Keep Apple credentials and signing identities out of PR comments and committed files.\n"
  report
end

options = parse_options
dry = options[:dry_run]

if dry
  run_json = options[:run_json_path] ? load_json_file(options[:run_json_path]) : sample_run_json
  artifacts_json = options[:artifacts_json_path] ? load_json_file(options[:artifacts_json_path]) : sample_artifacts_json
elsif options[:run_json_path] && options[:artifacts_json_path]
  run_id = options[:run_id].to_s.strip
  run_id = latest_signed_release_run_id(options[:repo], options[:runs_json_path]) if run_id.empty? && options[:runs_json_path]
  run_json = load_json_file(options[:run_json_path])
  artifacts_json = load_json_file(options[:artifacts_json_path])
  if !run_id.empty? && run_json["databaseId"].to_s != run_id
    fail_with("run JSON databaseId #{run_json["databaseId"].inspect} did not match discovered run id #{run_id}")
  end
elsif options[:run_json_path] || options[:artifacts_json_path]
  fail_with("--run-json and --artifacts-json must be provided together")
else
  run_id = options[:run_id].to_s.strip
  run_id = latest_signed_release_run_id(options[:repo], options[:runs_json_path]) if run_id.empty?
  fail_with("run id must be numeric") unless run_id.match?(/\A\d+\z/)

  run_json = github_run_json(options[:repo], run_id)
  artifacts_json = github_artifacts_json(options[:repo], run_id)
end

findings = validation_findings(run_json, artifacts_json)
path = report_path(options[:report_dir])
FileUtils.mkdir_p(File.dirname(path))
File.write(path, build_report(run_json, artifacts_json, findings, dry: dry))

puts "signed release evidence report written to #{path}"
warn "signed release evidence has #{findings.length} gap(s); see #{path}" if !dry && !findings.empty?
exit(dry || findings.empty? ? 0 : 1)
