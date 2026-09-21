#!/usr/bin/env ruby

require "digest"
require "json"
require "optparse"
require "pathname"

def fail_with(message)
  warn "release manifest: #{message}"
  exit 1
end

def options_for(argv)
  options = {}
  parser = OptionParser.new do |opts|
    opts.on("--package-root PATH", String) { |value| options[:package_root] = value }
    opts.on("--target TARGET", String) { |value| options[:target] = value }
    opts.on("--package NAME", String) { |value| options[:package] = value }
    opts.on("--services NAMES", String) { |value| options[:services] = value }
    opts.on("--tauri-app PATH", String) { |value| options[:tauri_app] = value }
    opts.on("--manifest PATH", String) { |value| options[:manifest] = value }
    opts.on("--output PATH", String) { |value| options[:output] = value }
  end
  parser.parse!(argv)
  options
rescue OptionParser::ParseError => error
  fail_with(error.message)
end

def selected_service_catalog(catalog, raw)
  return catalog if raw.nil?

  names = raw.split(",").map(&:strip)
  fail_with("--services must contain at least one service") if names.empty? || names.any?(&:empty?)
  fail_with("--services contains duplicate service") unless names.uniq.length == names.length
  unknown = names - catalog.map { |entry| entry.fetch("name") }
  fail_with("--services contains unknown service: #{unknown.join(", ")}") unless unknown.empty?
  names.map do |name|
    catalog.find { |entry| entry.fetch("name") == name }
  end
end

def selected_manifest_service_catalog(catalog, manifest)
  raw = manifest["services"]
  return catalog if raw.nil? || (raw.is_a?(Array) && raw.empty?)
  fail_with("manifest services must be an array") unless raw.is_a?(Array)
  fail_with("manifest services contain duplicate service") unless raw.uniq.length == raw.length
  selected_service_catalog(catalog, raw.join(","))
end

def sha256(path)
  Digest::SHA256.file(path).hexdigest
end

def cargo_version(root, crate)
  fail_with("invalid Cargo package: #{crate}") unless crate.match?(/\A[a-zA-Z0-9_-]+\z/)
  paths = Dir.glob(root.join("crates", "*", crate, "Cargo.toml").to_s)
  fail_with("expected one Cargo.toml for #{crate}, found #{paths.length}") unless paths.length == 1
  content = File.read(paths.fetch(0))
  match = content.match(/^version\s*=\s*"([^"]+)"/)
  fail_with("missing version for #{crate}") unless match
  match[1]
rescue Errno::ENOENT
  fail_with("missing Cargo.toml for #{crate}")
end

def protocol_version(root)
  content = File.read(root.join("crates", "process", "a4n-service-protocol", "src", "registry.rs"))
  match = content.match(/PROTOCOL_VERSION:\s*u32\s*=\s*(\d+)/)
  fail_with("missing protocol version") unless match
  Integer(match[1])
end

def release_manifest_schema_version(root)
  content = File.read(root.join("crates", "process", "a4n-process-catalog", "src", "release.rs"))
  match = content.match(/RELEASE_MANIFEST_SCHEMA_VERSION:\s*u32\s*=\s*(\d+)/)
  fail_with("missing release manifest schema version") unless match
  Integer(match[1])
end

def sidecar_health_path(root)
  content = File.read(root.join("crates", "process", "a4n-service-protocol", "src", "lib.rs"))
  match = content.match(/SIDECAR_HEALTH_PATH:\s*&str\s*=\s*"([^"]+)"/)
  fail_with("missing sidecar health path") unless match
  match[1]
end

def required_option(options, key)
  value = options[key]
  flag = "--#{key.to_s.tr("_", "-")}"
  fail_with("#{flag} is required") unless value.is_a?(String) && !value.strip.empty?
  value
end

def required_identifier_option(options, key)
  value = required_option(options, key)
  fail_with("--#{key.to_s.tr("_", "-")} is invalid") unless value.match?(/\A[a-z0-9][a-z0-9_-]*\z/)
  value
end

def load_inventory(path, label, required_fields:, identifier_fields:, unique_fields:)
  entries = JSON.parse(File.read(path))
  fail_with("#{label} must be a non-empty array") unless entries.is_a?(Array) && !entries.empty?
  entries.each do |entry|
    valid = entry.is_a?(Hash) && required_fields.all? do |field|
      value = entry[field]
      value.is_a?(String) && !value.strip.empty?
    end
    fail_with("#{label} entry is incomplete") unless valid
    identifier_fields.each do |field|
      fail_with("#{label} #{field} is invalid") unless entry.fetch(field).match?(/\A[a-z0-9][a-z0-9_-]*\z/)
    end
  end
  unique_fields.each do |field|
    values = entries.map { |entry| entry.fetch(field) }
    fail_with("#{label} #{field} values are duplicated") unless values.uniq.length == values.length
  end
  entries
rescue JSON::ParserError => error
  fail_with("#{label} JSON is invalid: #{error.message}")
rescue Errno::ENOENT => error
  fail_with("#{label} is unavailable: #{error.message}")
end

def load_service_catalog(root)
  catalog = load_inventory(
    root.join("crates/process/a4n-process-catalog/service-catalog.json"),
    "service catalog",
    required_fields: %w[name binary capability_version addr_env url_env default_addr],
    identifier_fields: %w[name binary],
    unique_fields: %w[name binary addr_env url_env]
  )
  catalog.each do |entry|
    fail_with("service catalog environment is invalid") unless %w[addr_env url_env].all? do |field|
      entry.fetch(field).match?(/\AA4N_[A-Z0-9_]+\z/)
    end
  end
  catalog
end

def load_tool_catalog(root)
  load_inventory(
    root.join("crates/process/a4n-process-catalog/release-tools.json"),
    "release tool catalog",
    required_fields: %w[package binary],
    identifier_fields: %w[package binary],
    unique_fields: %w[package binary]
  )
end

def tool_catalog_for_package(tool_catalog, package)
  tool_catalog.reject { |entry| entry.fetch("package") == package }
end

def load_release_inventories(root)
  service_catalog = load_service_catalog(root)
  tool_catalog = load_tool_catalog(root)
  overlap = service_catalog.map { |entry| entry.fetch("binary") } & tool_catalog.map { |entry| entry.fetch("binary") }
  fail_with("service and release tool binaries overlap: #{overlap.join(", ")}") unless overlap.empty?
  [service_catalog, tool_catalog]
end

def print_service_rows(root)
  health_path = sidecar_health_path(root)
  load_service_catalog(root).each do |entry|
    puts (%w[name binary addr_env url_env].map { |field| entry.fetch(field) } + [health_path]).join("\t")
  end
end

def print_tool_rows(root)
  load_tool_catalog(root).each do |entry|
    puts %w[package binary].map { |field| entry.fetch(field) }.join("\t")
  end
end

def binary_extension(target)
  target.include?("windows") ? ".exe" : ""
end

def runtime_file_name(target)
  return "onnxruntime.dll" if target.include?("windows")
  return "libonnxruntime.dylib" if target.include?("darwin") || target.include?("macos")

  "libonnxruntime.so.1"
end

def safe_package_path(root, relative)
  relative = validate_relative_manifest_path(relative)
  path = root.join(relative).cleanpath
  root_path = root.cleanpath
  fail_with("manifest path escapes package root") unless path.to_s.start_with?(root_path.to_s + File::SEPARATOR)
  if path.exist? || File.symlink?(path.to_s)
    begin
      resolved_root = root.realpath
      resolved_path = path.realpath
      fail_with("manifest path resolves outside package root") unless resolved_path.to_s.start_with?(resolved_root.to_s + File::SEPARATOR)
    rescue SystemCallError
      fail_with("manifest path cannot be resolved")
    end
  end
  path
end

def validate_relative_manifest_path(relative)
  fail_with("manifest path must be a string") unless relative.is_a?(String)
  value = relative
  fail_with("manifest path is empty") if value.empty?
  fail_with("manifest path contains NUL") if value.include?("\0")
  fail_with("manifest path must be relative") if Pathname.new(value).absolute? || value.match?(/\A[A-Za-z]:[\\\/]/)
  fail_with("manifest path must use forward slashes") if value.include?("\\")
  fail_with("manifest path contains an empty or current component") if value.split("/").any? { |part| part.empty? || part == "." }
  fail_with("manifest path contains traversal") if value.split("/").include?("..")
  value
end

def sha256_value?(value)
  value.is_a?(String) && value.match?(/\A[0-9a-fA-F]{64}\z/)
end

def manifest_file_name(path)
  path.to_s.split(/[\\\/]/).last
end

def manifest_checksum_path(package_root, output)
  relative = output.relative_path_from(package_root).to_s
  relative = output.basename.to_s if relative.split(/[\\\/]/).include?("..")
  validate_relative_manifest_path(relative)
end

def tauri_app_layout(options, package_root, target)
  return unless options[:tauri_app]
  fail_with("--tauri-app requires a macOS target") unless target.include?("darwin") || target.include?("macos")

  app = Pathname(options[:tauri_app]).expand_path
  relative = app.relative_path_from(package_root).to_s
  validate_relative_manifest_path(relative)
  fail_with("Tauri app must be a direct child of package root") if relative.include?(File::SEPARATOR)
  fail_with("Tauri app must have an .app suffix") unless relative.end_with?(".app")
  fail_with("missing Tauri app: #{relative}") unless app.directory?

  { root: relative }
rescue ArgumentError
  fail_with("Tauri app must be inside package root")
end

def manifest_layout(binaries, target)
  host = binaries.find { |entry| entry.fetch("service") == "host" }
  fail_with("manifest must contain one host") unless host && binaries.count { |entry| entry.fetch("service") == "host" } == 1
  match = host.fetch("path").match(%r{\A([^/]+\.app)/Contents/MacOS/[^/]+\z})
  return { type: :portable } unless match

  fail_with("Tauri app layout requires a macOS target") unless target.include?("darwin") || target.include?("macos")
  app_root = match[1]
  { type: :tauri_macos, root: app_root }
end

def generate(options)
  root = Pathname(__dir__).parent
  package_root = Pathname(required_option(options, :package_root)).expand_path
  target = required_option(options, :target)
  package = required_identifier_option(options, :package)
  output = Pathname(required_option(options, :output)).expand_path
  catalog, tool_catalog = load_release_inventories(root)
  tool_catalog = tool_catalog_for_package(tool_catalog, package)
  selected_catalog = selected_service_catalog(catalog, options[:services])
  extension = binary_extension(target)
  protocol = protocol_version(root)
  schema_version = release_manifest_schema_version(root)
  binaries = []
  tauri_app = tauri_app_layout(options, package_root, target)

  host_name = "#{package}#{extension}"
  host_relative = if tauri_app
                    "#{tauri_app.fetch(:root)}/Contents/MacOS/#{host_name}"
                  else
                    "bin/#{host_name}"
                  end
  host_path = safe_package_path(package_root, host_relative)
  fail_with("missing #{host_relative}") unless host_path.file?
  binaries << {
    "name" => host_name,
    "service" => "host",
    "path" => host_relative,
    "version" => cargo_version(root, package),
    "target" => target,
    "sha256" => sha256(host_path),
    "protocol_version" => protocol,
    "capability_version" => nil
  }

  selected_catalog.each do |entry|
    binary = "#{entry.fetch("binary")}#{extension}"
    relative = if tauri_app
                 "#{tauri_app.fetch(:root)}/Contents/MacOS/#{binary}"
               else
                 "bin/services/#{binary}"
               end
    path = safe_package_path(package_root, relative)
    fail_with("missing #{relative}") unless path.file?
    binaries << {
      "name" => binary,
      "service" => entry.fetch("name"),
      "path" => relative,
      "version" => cargo_version(root, entry.fetch("binary")),
      "target" => target,
      "sha256" => sha256(path),
      "protocol_version" => protocol,
      "capability_version" => entry.fetch("capability_version")
    }
  end

  tools = tool_catalog.map do |entry|
    crate = entry.fetch("package")
    tool = entry.fetch("binary")
    name = "#{tool}#{extension}"
    relative = if tauri_app
                 "#{tauri_app.fetch(:root)}/Contents/MacOS/#{name}"
               else
                 "bin/#{name}"
               end
    path = safe_package_path(package_root, relative)
    fail_with("missing #{relative}") unless path.file?
    {
      "name" => name,
      "path" => relative,
      "version" => cargo_version(root, crate),
      "target" => target,
      "sha256" => sha256(path)
    }
  end

  runtime_name = runtime_file_name(target)
  runtime_relative = if tauri_app
                       "#{tauri_app.fetch(:root)}/Contents/Resources/#{runtime_name}"
                     else
                       "bin/services/#{runtime_name}"
                     end
  runtime_path = safe_package_path(package_root, runtime_relative)
  fail_with("missing #{runtime_relative}") unless runtime_path.file?
  runtime_files = [{
    "name" => runtime_name,
    "path" => runtime_relative,
    "sha256" => sha256(runtime_path)
  }]

  manifest = {
    "schema_version" => schema_version,
    "package" => package,
    "version" => binaries.first.fetch("version"),
    "target" => target,
    "protocol_version" => protocol,
    "services" => selected_catalog.map { |entry| entry.fetch("name") },
    "binaries" => binaries,
    "tools" => tools,
    "runtime_files" => runtime_files
  }
  manifest_relative = manifest_checksum_path(package_root, output)
  File.write(output, JSON.pretty_generate(manifest) + "\n")
  checksum = output.sub_ext(".sha256")
  files = binaries + tools + runtime_files
  checksum_entries = files.map { |entry| "#{entry.fetch("sha256")}  #{entry.fetch("path")}" }
  checksum_entries << "#{sha256(output)}  #{manifest_relative}"
  File.write(checksum, checksum_entries.join("\n") + "\n")
  puts "generated #{output} (#{binaries.length} binaries, #{tools.length} tools, #{runtime_files.length} runtime file)"
end

def verify(options)
  root = Pathname(__dir__).parent
  package_root = Pathname(required_option(options, :package_root)).expand_path
  path = Pathname(required_option(options, :manifest)).expand_path
  catalog, tool_catalog = load_release_inventories(root)
  schema_version = release_manifest_schema_version(root)
  manifest = JSON.parse(File.read(path))
  fail_with("manifest schema version is unsupported") unless manifest.fetch("schema_version") == schema_version
  package = manifest.fetch("package")
  tool_catalog = tool_catalog_for_package(tool_catalog, package)
  version = manifest.fetch("version")
  target = manifest.fetch("target")
  protocol = manifest.fetch("protocol_version")
  fail_with("manifest package is invalid") unless package.is_a?(String) && package.match?(/\A[a-z0-9][a-z0-9_-]*\z/)
  fail_with("manifest version is empty") unless version.is_a?(String) && !version.strip.empty?
  fail_with("manifest target is empty") unless target.is_a?(String) && !target.strip.empty?
  fail_with("manifest protocol version is invalid") unless protocol.is_a?(Integer)
  fail_with("manifest protocol version does not match release") unless protocol == protocol_version(root)
  binaries = manifest.fetch("binaries")
  fail_with("manifest binaries must be an array") unless binaries.is_a?(Array)
  selected_catalog = selected_manifest_service_catalog(catalog, manifest)
  services = binaries.map { |entry| entry.fetch("service") }
  expected_services = ["host"] + selected_catalog.map { |entry| entry.fetch("name") }
  services_match = services.length == expected_services.length &&
                   (services - expected_services).empty? &&
                   (expected_services - services).empty?
  fail_with("manifest services do not match service catalog") unless services_match
  names = binaries.map { |entry| entry.fetch("name") }
  fail_with("manifest binary names are duplicated") unless names.uniq.length == names.length
  binaries.each do |entry|
    name = entry.fetch("name")
    entry_target = entry.fetch("target")
    entry_protocol = entry.fetch("protocol_version")
    fail_with("manifest binary entry is incomplete") unless entry_target == target && entry_protocol == protocol
    fail_with("manifest binary version is empty") unless entry.fetch("version").is_a?(String) && !entry.fetch("version").strip.empty?
    fail_with("manifest binary SHA-256 is invalid") unless sha256_value?(entry.fetch("sha256"))
    fail_with("manifest binary name does not match path") unless manifest_file_name(entry.fetch("path")) == name
    actual_path = safe_package_path(package_root, entry.fetch("path"))
    fail_with("missing #{entry.fetch("path")}") unless actual_path.file?
    actual = sha256(actual_path)
    fail_with("SHA-256 mismatch for #{entry.fetch("service")}") unless actual.casecmp?(entry.fetch("sha256"))
  end
  extension = binary_extension(target)
  layout = manifest_layout(binaries, target)
  host = binaries.find { |entry| entry.fetch("service") == "host" }
  expected_host_name = "#{package}#{extension}"
  fail_with("manifest Host name does not match package") unless host.fetch("name") == expected_host_name
  fail_with("manifest version does not match package") unless version == cargo_version(root, package)
  fail_with("manifest version does not match host") unless host.fetch("version") == version
  expected_host_path = if layout.fetch(:type) == :tauri_macos
                         "#{layout.fetch(:root)}/Contents/MacOS/#{expected_host_name}"
                       else
                         "bin/#{expected_host_name}"
                       end
  fail_with("manifest host path is invalid") unless host.fetch("path") == expected_host_path
  tools = manifest.fetch("tools")
  fail_with("manifest tools must be an array") unless tools.is_a?(Array)
  expected_tool_names = tool_catalog.map { |entry| "#{entry.fetch("binary")}#{extension}" }
  tool_names = tools.map { |entry| entry.fetch("name") }
  tools_match = tool_names.length == expected_tool_names.length &&
                (tool_names - expected_tool_names).empty? &&
                (expected_tool_names - tool_names).empty?
  fail_with("manifest tool names do not match release tool catalog") unless tools_match
  tool_catalog.each do |tool_spec|
    crate = tool_spec.fetch("package")
    name = "#{tool_spec.fetch("binary")}#{extension}"
    entry = tools.find { |candidate| candidate.fetch("name") == name }
    expected_path = if layout.fetch(:type) == :tauri_macos
                      "#{layout.fetch(:root)}/Contents/MacOS/#{name}"
                    else
                      "bin/#{name}"
                    end
    fail_with("manifest tool target is invalid") unless entry.fetch("target") == target
    fail_with("manifest tool version is invalid") unless entry.fetch("version") == cargo_version(root, crate)
    fail_with("manifest tool SHA-256 is invalid") unless sha256_value?(entry.fetch("sha256"))
    fail_with("manifest tool name does not match path") unless manifest_file_name(entry.fetch("path")) == name
    fail_with("manifest tool path does not match layout") unless entry.fetch("path") == expected_path
    actual_path = safe_package_path(package_root, entry.fetch("path"))
    fail_with("missing #{entry.fetch("path")}") unless actual_path.file?
    fail_with("SHA-256 mismatch for #{name}") unless sha256(actual_path).casecmp?(entry.fetch("sha256"))
  end
  runtime_files = manifest.fetch("runtime_files")
  fail_with("manifest must contain one ONNX Runtime file") unless runtime_files.is_a?(Array) && runtime_files.length == 1
  expected_runtime_name = runtime_file_name(target)
  runtime_files.each do |entry|
    fail_with("manifest runtime file name is invalid") unless entry.fetch("name") == expected_runtime_name
    fail_with("manifest runtime file SHA-256 is invalid") unless sha256_value?(entry.fetch("sha256"))
    fail_with("manifest runtime file name does not match path") unless manifest_file_name(entry.fetch("path")) == entry.fetch("name")
    expected_runtime_path = if layout.fetch(:type) == :tauri_macos
                              "#{layout.fetch(:root)}/Contents/Resources/#{expected_runtime_name}"
                            else
                              "bin/services/#{expected_runtime_name}"
                            end
    fail_with("manifest runtime file path is invalid") unless entry.fetch("path") == expected_runtime_path
    actual_path = safe_package_path(package_root, entry.fetch("path"))
    fail_with("missing #{entry.fetch("path")}") unless actual_path.file?
    actual = sha256(actual_path)
    fail_with("SHA-256 mismatch for #{entry.fetch("name")}") unless actual.casecmp?(entry.fetch("sha256"))
  end
  selected_catalog.each do |catalog_entry|
    manifest_entry = binaries.find { |entry| entry.fetch("service") == catalog_entry.fetch("name") }
    fail_with("manifest capability version does not match service catalog") unless manifest_entry.fetch("capability_version") == catalog_entry.fetch("capability_version")
    fail_with("manifest service version does not match package") unless manifest_entry.fetch("version") == cargo_version(root, catalog_entry.fetch("binary"))
    binary_name = "#{catalog_entry.fetch("binary")}#{binary_extension(target)}"
    expected_path = if layout.fetch(:type) == :tauri_macos
                      "#{layout.fetch(:root)}/Contents/MacOS/#{binary_name}"
                    else
                      "bin/services/#{binary_name}"
                    end
    fail_with("manifest service path does not match layout") unless manifest_entry.fetch("name") == binary_name && manifest_entry.fetch("path") == expected_path
  end
  paths = (binaries + tools + runtime_files).map { |entry| entry.fetch("path") }
  fail_with("manifest file paths are duplicated") unless paths.uniq.length == paths.length
  puts "verified #{path} (#{binaries.length} binaries, #{tools.length} tools, #{runtime_files.length} runtime file)"
rescue JSON::ParserError, KeyError, TypeError => error
  fail_with("manifest JSON is invalid: #{error.message}")
end

command = ARGV.shift
root = Pathname(__dir__).parent
case command
when "generate"
  generate(options_for(ARGV))
when "verify"
  verify(options_for(ARGV))
when "service-rows"
  fail_with("service-rows does not accept options") unless ARGV.empty?
  print_service_rows(root)
when "tool-rows"
  fail_with("tool-rows does not accept options") unless ARGV.empty?
  print_tool_rows(root)
when "schema-version"
  fail_with("schema-version does not accept options") unless ARGV.empty?
  puts release_manifest_schema_version(root)
else
  fail_with("usage: release_manifest.rb generate|verify [options] | service-rows | tool-rows | schema-version")
end
