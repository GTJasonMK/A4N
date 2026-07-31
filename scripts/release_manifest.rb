#!/usr/bin/env ruby

require "digest"
require "json"
require "optparse"
require "pathname"

TOOL_BINARIES = [
  ["a4n-cli", "a4n"],
  ["a4n-server", "a4n-server"],
  ["a4n-migration", "a4n-migration"]
].freeze

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
    opts.on("--host NAME", String) { |value| options[:host] = value }
    opts.on("--catalog PATH", String) { |value| options[:catalog] = value }
    opts.on("--tauri-app PATH", String) { |value| options[:tauri_app] = value }
    opts.on("--manifest PATH", String) { |value| options[:manifest] = value }
    opts.on("--output PATH", String) { |value| options[:output] = value }
  end
  parser.parse!(argv)
  options
rescue OptionParser::ParseError => error
  fail_with(error.message)
end

def sha256(path)
  Digest::SHA256.file(path).hexdigest
end

def cargo_version(root, crate)
  path = root.join("crates", crate, "Cargo.toml")
  content = File.read(path)
  match = content.match(/^version\s*=\s*"([^"]+)"/)
  fail_with("missing version for #{crate}") unless match
  match[1]
rescue Errno::ENOENT
  fail_with("missing Cargo.toml for #{crate}")
end

def protocol_version(root)
  content = File.read(root.join("crates", "a4n-service-protocol", "src", "registry.rs"))
  match = content.match(/PROTOCOL_VERSION:\s*u32\s*=\s*(\d+)/)
  fail_with("missing protocol version") unless match
  Integer(match[1])
end

def load_catalog(path)
  entries = JSON.parse(File.read(path))
  fail_with("catalog must be an array") unless entries.is_a?(Array)
  entries
rescue JSON::ParserError => error
  fail_with("catalog JSON is invalid: #{error.message}")
end

def binary_extension(target)
  target.include?("windows") ? ".exe" : ""
end

def runtime_file_name(target)
  return "onnxruntime.dll" if target.include?("windows")
  return "libonnxruntime.dylib" if target.include?("darwin") || target.include?("macos")

  "libonnxruntime.so.1"
end

def manifest_path(options)
  options[:manifest] || options[:output] || fail_with("--manifest or --output is required")
end

def safe_package_path(root, relative)
  relative = validate_relative_manifest_path(relative)
  path = root.join(relative).cleanpath
  root_path = root.cleanpath
  fail_with("manifest path escapes package root") unless path.to_s.start_with?(root_path.to_s + File::SEPARATOR)
  path
end

def validate_relative_manifest_path(relative)
  value = relative.to_s
  fail_with("manifest path is empty") if value.empty?
  fail_with("manifest path contains NUL") if value.include?("\0")
  fail_with("manifest path must be relative") if Pathname.new(value).absolute? || value.match?(/\A[A-Za-z]:[\\\/]/)
  fail_with("manifest path contains traversal") if value.split(/[\\\/]+/).include?("..")
  value
end

def sha256_value?(value)
  value.is_a?(String) && value.match?(/\A[0-9a-fA-F]{64}\z/)
end

def manifest_file_name(path)
  path.to_s.split(/[\\\/]/).last
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
  match = host.fetch("path").match(%r{\A([^/]+\.app)/Contents/MacOS/([^/]+)\z})
  return { type: :portable } unless match

  fail_with("Tauri app layout requires a macOS target") unless target.include?("darwin") || target.include?("macos")
  app_root = match[1]
  host_name = match[2]
  fail_with("Tauri Host name does not match manifest path") unless host.fetch("name") == host_name
  { type: :tauri_macos, root: app_root, host: host_name }
end

def generate(options)
  root = Pathname(__dir__).parent
  package_root = Pathname(options[:package_root] || fail_with("--package-root is required")).expand_path
  target = options[:target] || fail_with("--target is required")
  package = options[:package] || fail_with("--package is required")
  host = options[:host] || package
  catalog_path = Pathname(options[:catalog] || root.join("crates/a4n-service-protocol/service-catalog.json")).expand_path
  output = Pathname(options[:output] || package_root.join("release-manifest.json")).expand_path
  catalog = load_catalog(catalog_path)
  extension = binary_extension(target)
  protocol = protocol_version(root)
  binaries = []
  tauri_app = tauri_app_layout(options, package_root, target)

  host_name = "#{host}#{extension}"
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
    "version" => cargo_version(root, host),
    "target" => target,
    "sha256" => sha256(host_path),
    "protocol_version" => protocol,
    "capability_version" => nil
  }

  catalog.each do |entry|
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

  tools = TOOL_BINARIES.map do |crate, tool|
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
    "schema_version" => 2,
    "package" => package,
    "version" => binaries.first.fetch("version"),
    "target" => target,
    "protocol_version" => protocol,
    "binaries" => binaries,
    "tools" => tools,
    "runtime_files" => runtime_files
  }
  File.write(output, JSON.pretty_generate(manifest) + "\n")
  checksum = output.sub_ext(".sha256")
  files = binaries + tools + runtime_files
  File.write(checksum, files.map { |entry| "#{entry.fetch("sha256")}  #{entry.fetch("path")}" }.join("\n") + "\n")
  puts "generated #{output} (#{binaries.length} binaries, #{tools.length} tools, #{runtime_files.length} runtime file)"
end

def verify(options)
  package_root = Pathname(options[:package_root] || fail_with("--package-root is required")).expand_path
  path = Pathname(manifest_path(options)).expand_path
  manifest = JSON.parse(File.read(path))
  fail_with("manifest schema version is unsupported") unless manifest.fetch("schema_version") == 2
  package = manifest.fetch("package")
  version = manifest.fetch("version")
  target = manifest.fetch("target")
  protocol = manifest.fetch("protocol_version")
  fail_with("manifest package is empty") unless package.is_a?(String) && !package.strip.empty?
  fail_with("manifest version is empty") unless version.is_a?(String) && !version.strip.empty?
  fail_with("manifest target is empty") unless target.is_a?(String) && !target.strip.empty?
  fail_with("manifest protocol version is invalid") unless protocol.is_a?(Integer)
  binaries = manifest.fetch("binaries")
  fail_with("manifest must contain host and eight sidecars") unless binaries.is_a?(Array) && binaries.length == 9
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
  layout = manifest_layout(binaries, target)
  host = binaries.find { |entry| entry.fetch("service") == "host" }
  fail_with("manifest version does not match host") unless host.fetch("version") == version
  expected_host_path = if layout.fetch(:type) == :tauri_macos
                         "#{layout.fetch(:root)}/Contents/MacOS/#{layout.fetch(:host)}"
                       else
                         "bin/#{host.fetch("name")}"
                       end
  fail_with("manifest host path is invalid") unless host.fetch("path") == expected_host_path
  tools = manifest.fetch("tools")
  extension = binary_extension(target)
  expected_tool_names = TOOL_BINARIES.map { |_, tool| "#{tool}#{extension}" }
  fail_with("manifest must contain a4n CLI, Server, and migration tools") unless tools.is_a?(Array) && tools.length == TOOL_BINARIES.length
  fail_with("manifest tool names do not match release tools") unless tools.map { |entry| entry.fetch("name") } == expected_tool_names
  tools.zip(TOOL_BINARIES).each do |entry, (crate, _tool)|
    name = entry.fetch("name")
    expected_path = if layout.fetch(:type) == :tauri_macos
                      "#{layout.fetch(:root)}/Contents/MacOS/#{name}"
                    else
                      "bin/#{name}"
                    end
    fail_with("manifest tool target is invalid") unless entry.fetch("target") == target
    fail_with("manifest tool version is invalid") unless entry.fetch("version") == cargo_version(Pathname(__dir__).parent, crate)
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
  if options[:catalog]
    catalog = load_catalog(Pathname(options[:catalog]).expand_path)
    services = binaries.map { |entry| entry.fetch("service") }
    expected = ["host"] + catalog.map { |entry| entry.fetch("name") }
    fail_with("manifest services do not match service catalog") unless services == expected
    catalog.each do |catalog_entry|
      manifest_entry = binaries.find { |entry| entry.fetch("service") == catalog_entry.fetch("name") }
      fail_with("manifest capability version does not match service catalog") unless manifest_entry.fetch("capability_version") == catalog_entry.fetch("capability_version")
      binary_name = "#{catalog_entry.fetch("binary")}#{binary_extension(target)}"
      expected_path = if layout.fetch(:type) == :tauri_macos
                        "#{layout.fetch(:root)}/Contents/MacOS/#{binary_name}"
                      else
                        "bin/services/#{binary_name}"
                      end
      fail_with("manifest service path does not match layout") unless manifest_entry.fetch("name") == binary_name && manifest_entry.fetch("path") == expected_path
    end
  end
  paths = (binaries + tools + runtime_files).map { |entry| entry.fetch("path") }
  fail_with("manifest file paths are duplicated") unless paths.uniq.length == paths.length
  puts "verified #{path} (#{binaries.length} binaries, #{tools.length} tools, #{runtime_files.length} runtime file)"
rescue JSON::ParserError, KeyError, TypeError => error
  fail_with("manifest JSON is invalid: #{error.message}")
end

command = ARGV.shift
options = options_for(ARGV)
case command
when "generate"
  generate(options)
when "verify"
  verify(options)
else
  fail_with("usage: release_manifest.rb generate|verify [options]")
end
