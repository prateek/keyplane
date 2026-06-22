#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"

ROOT = File.expand_path("..", __dir__)
CAPABILITY = File.join(ROOT, "src-tauri/capabilities/default.json")
PACKAGE_JSON = File.join(ROOT, "package.json")
CARGO_TOML = File.join(ROOT, "src-tauri/Cargo.toml")
LIB_RS = File.join(ROOT, "src-tauri/src/lib.rs")

EXPECTED_WINDOWS = %w[main overlay].freeze
EXPECTED_PERMISSIONS = %w[
  autostart:allow-disable
  autostart:allow-enable
  autostart:allow-is-enabled
  core:default
].freeze

def fail_with(message)
  warn "Tauri capability validation failed: #{message}"
  exit 1
end

def load_json(path)
  JSON.parse(File.read(path))
rescue JSON::ParserError => e
  fail_with("#{path} is not valid JSON: #{e.message}")
end

capability = load_json(CAPABILITY)
windows = capability.fetch("windows") { fail_with("default capability is missing windows") }
permissions = capability.fetch("permissions") { fail_with("default capability is missing permissions") }

unless windows.sort == EXPECTED_WINDOWS
  fail_with("default capability windows changed; expected #{EXPECTED_WINDOWS.inspect}, got #{windows.inspect}")
end

unless permissions.sort == EXPECTED_PERMISSIONS
  fail_with("default permissions changed; expected #{EXPECTED_PERMISSIONS.inspect}, got #{permissions.inspect}")
end

opener_permissions = permissions.select { |permission| permission.start_with?("opener:") }
fail_with("unused opener permissions remain: #{opener_permissions.join(", ")}") unless opener_permissions.empty?

package_json = load_json(PACKAGE_JSON)
dependencies = package_json.fetch("dependencies", {})
if dependencies.key?("@tauri-apps/plugin-opener")
  fail_with("package.json still depends on unused @tauri-apps/plugin-opener")
end

cargo_toml = File.read(CARGO_TOML)
if cargo_toml.include?("tauri-plugin-opener")
  fail_with("Cargo.toml still depends on unused tauri-plugin-opener")
end

lib_rs = File.read(LIB_RS)
if lib_rs.include?("tauri_plugin_opener")
  fail_with("Rust app still initializes unused tauri_plugin_opener")
end

puts "Tauri capability validation passed"
