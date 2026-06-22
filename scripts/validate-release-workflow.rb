#!/usr/bin/env ruby
# frozen_string_literal: true

require "yaml"

ROOT = File.expand_path("..", __dir__)
SIGNED_RELEASE = File.join(ROOT, ".github/workflows/signed-release.yml")
DESKTOP_BUILD = File.join(ROOT, ".github/workflows/desktop-build.yml")

def fail_with(message)
  warn "release workflow validation failed: #{message}"
  exit 1
end

def load_workflow(path)
  YAML.load_file(path)
rescue Psych::SyntaxError => e
  fail_with("#{path} is not valid YAML: #{e.message}")
end

def workflow_on(workflow)
  workflow["on"] || workflow[true] || {}
end

def step_named(job, name)
  step = job.fetch("steps", []).find { |candidate| candidate["name"] == name }
  fail_with("missing step #{name.inspect}") unless step
  step
end

def assert_equal(actual, expected, message)
  return if actual == expected

  fail_with("#{message}; expected #{expected.inspect}, got #{actual.inspect}")
end

def assert_includes(collection, value, message)
  fail_with("#{message}; missing #{value.inspect}") unless collection&.include?(value)
end

def assert_run_contains(step, text, message)
  run = step["run"].to_s
  return if run.include?(text)

  fail_with("#{message}; #{step["name"].inspect} does not contain #{text.inspect}")
end

signed_release = load_workflow(SIGNED_RELEASE)
desktop_build = load_workflow(DESKTOP_BUILD)

assert_equal(signed_release["name"], "Signed Release", "signed release workflow name changed")
on = workflow_on(signed_release)
assert_includes(on.dig("push", "tags"), "v*", "signed release must run for v* tags")
dispatch = on["workflow_dispatch"] || {}
skip_stapling = dispatch.dig("inputs", "skip_stapling") || {}
assert_equal(
  skip_stapling["default"],
  "true",
  "manual signed release should default to skip stapling"
)
assert_equal(skip_stapling["type"], "choice", "skip_stapling input should stay constrained")
assert_equal(skip_stapling["options"], %w[true false], "skip_stapling options changed")

job = signed_release.dig("jobs", "signed-macos-release")
fail_with("missing signed-macos-release job") unless job
assert_equal(job["runs-on"], "macos-latest", "signed release must run on macOS")
assert_equal(
  job.dig("permissions", "contents"),
  "read",
  "signed release should keep read-only contents permission"
)

required_secrets = %w[
  APPLE_CERTIFICATE
  APPLE_CERTIFICATE_PASSWORD
  APPLE_ID
  APPLE_PASSWORD
  APPLE_TEAM_ID
  KEYCHAIN_PASSWORD
]

env = job["env"] || {}
required_secrets.each do |name|
  assert_equal(env[name], "${{ secrets.#{name} }}", "#{name} must come from a GitHub secret")
end
assert_equal(
  env["SKIP_STAPLING"],
  "${{ github.event.inputs.skip_stapling || 'true' }}",
  "SKIP_STAPLING expression changed"
)

validate_secrets = step_named(job, "Validate signing secrets")
required_secrets.each do |name|
  assert_run_contains(validate_secrets, name, "#{name} should be validated before signing")
end
assert_run_contains(validate_secrets, "Missing signing secrets", "missing-secret failures should be explicit")

import_certificate = step_named(job, "Import Apple signing certificate")
[
  "base64 --decode",
  "security create-keychain",
  "security import certificate.p12",
  "security set-key-partition-list",
  "security find-identity -v -p codesigning",
  "APPLE_SIGNING_IDENTITY=$identity"
].each do |text|
  assert_run_contains(import_certificate, text, "certificate import contract changed")
end

build_release = step_named(job, "Build signed macOS release")
[
  'args=(--bundles "app,dmg" --ci)',
  '--skip-stapling',
  'npm run tauri build -- "${args[@]}"'
].each do |text|
  assert_run_contains(build_release, text, "signed Tauri build command changed")
end

verify_release = step_named(job, "Verify signed macOS artifacts")
[
  'report_path="$validation_dir/signed-artifacts.md"',
  'app_path="src-tauri/target/release/bundle/macos/Keyplane.app"',
  'codesign --verify --deep --strict --verbose=2 "$app_path"',
  'codesign -dv --verbose=4 "$app_path"',
  'spctl --assess --type execute --verbose=4 "$app_path"',
  'hdiutil verify "$dmg_path"',
  'shasum -a 256 "$dmg_path"'
].each do |text|
  assert_run_contains(verify_release, text, "signed artifact verification contract changed")
end

assert_equal(
  step_named(job, "Upload signed macOS app bundle").dig("with", "path"),
  "src-tauri/target/release/bundle/macos/Keyplane.app",
  "signed app artifact path changed"
)
assert_equal(
  step_named(job, "Upload signed macOS dmg").dig("with", "path"),
  "src-tauri/target/release/bundle/dmg/*.dmg",
  "signed dmg artifact path changed"
)
verification_upload = step_named(job, "Upload signed release verification report")
assert_equal(
  verification_upload.dig("with", "name"),
  "keyplane-macos-release-evidence",
  "signed release verification artifact name changed"
)
assert_equal(
  verification_upload.dig("with", "path"),
  "target/keyplane-validation/signed-artifacts.md",
  "signed release verification artifact path changed"
)

cleanup = step_named(job, "Clean up signing keychain")
assert_equal(cleanup["if"], "${{ always() }}", "signing keychain cleanup must run even on failure")
assert_run_contains(cleanup, "rm -f certificate.p12", "temporary certificate should be removed")
assert_run_contains(cleanup, "security delete-keychain build.keychain", "temporary keychain should be removed")

desktop_job = desktop_build.dig("jobs", "release-workflow-static-validation")
fail_with("Desktop Build must validate release workflow on PRs") unless desktop_job
validation_step = step_named(desktop_job, "Validate release workflow, Tauri capabilities, and evidence scripts")
assert_run_contains(
  validation_step,
  "ruby scripts/validate-release-workflow.rb",
  "Desktop Build must validate the signed release workflow on PRs"
)
assert_run_contains(
  validation_step,
  "ruby scripts/validate-tauri-capabilities.rb",
  "Desktop Build must validate Tauri capabilities on PRs"
)
assert_run_contains(
  validation_step,
  "ruby scripts/validate-evidence-scripts.rb",
  "Desktop Build must validate evidence report shape and fixture paths on PRs"
)

puts "release workflow validation passed"
