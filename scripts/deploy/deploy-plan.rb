#!/usr/bin/env ruby

require "digest"
require "json"
require "open3"
require "optparse"
require "pathname"

SCHEMA_VERSION = 1
IGNORED_DEPLOY_PATHS = [
  %r{\A\.github/},
  %r{\A\.codex/},
  %r{\A\.gitignore\z},
  %r{\A\.deploy\.env\z},
  %r{\Ascripts/deploy/deploy-cloud\.env\z},
  %r{\Ascripts/deploy/},
  %r{\Ascripts/lib/},
  %r{\Atarget/},
  %r{\Afrontend/(?:node_modules|dist|\.vite|\.tauri)/},
].freeze
FINGERPRINT_PATHS = [
  %r{\ACargo\.toml\z},
  %r{\ACargo\.lock\z},
  %r{\Acrates/},
  %r{\Afrontend/src/},
  %r{\Afrontend/package(?:-lock)?\.json\z},
  %r{\Aprompts/},
  %r{\Atools/onnxruntime/},
].freeze
FULL_CHANGE_PACKAGES = %w[
  a4n-migration
  a4n-process-catalog
  a4n-server
].freeze
FULL_CHANGE_CRATES = %w[
  a4n-service-protocol
  a4n-worker-contract
].freeze

def fail_with(message)
  warn "deploy-plan: #{message}"
  exit 1
end

def options_for(argv)
  options = {}
  parser = OptionParser.new do |opts|
    opts.on("--root PATH", String) { |value| options[:root] = value }
    opts.on("--metadata PATH", String) { |value| options[:metadata] = value }
    opts.on("--state PATH", String) { |value| options[:state] = value }
    opts.on("--plan PATH", String) { |value| options[:plan] = value }
    opts.on("--manifest PATH", String) { |value| options[:manifest] = value }
    opts.on("--release-id VALUE", String) { |value| options[:release_id] = value }
    opts.on("--output PATH", String) { |value| options[:output] = value }
    opts.on("--mode MODE", String) { |value| options[:mode] = value }
    opts.on("--target TARGET", String) { |value| options[:target] = value }
    opts.on("--builder IMAGE", String) { |value| options[:builder] = value }
    opts.on("--cargo-offline VALUE", String) { |value| options[:cargo_offline] = value }
    opts.on("--status STATUS", String) { |value| options[:status] = value }
    opts.on("--exit-status VALUE", Integer) { |value| options[:exit_status] = value }
    opts.on("--deployment-id ID", String) { |value| options[:deployment_id] = value }
    opts.on("--started-at VALUE", String) { |value| options[:started_at] = value }
    opts.on("--finished-at VALUE", String) { |value| options[:finished_at] = value }
    opts.on("--elapsed-seconds VALUE", Integer) { |value| options[:elapsed_seconds] = value }
    opts.on("--build-elapsed-seconds VALUE", Integer) { |value| options[:build_elapsed_seconds] = value }
    opts.on("--package-sha256 VALUE", String) { |value| options[:package_sha256] = value }
    opts.on("--package-size VALUE", String) { |value| options[:package_size] = value }
    opts.on("--package-created-at VALUE", String) { |value| options[:package_created_at] = value }
    opts.on("--remote VALUE", String) { |value| options[:remote] = value }
    opts.on("--remote-dir VALUE", String) { |value| options[:remote_dir] = value }
    opts.on("--service VALUE", String) { |value| options[:service] = value }
    opts.on("--public-app-url VALUE", String) { |value| options[:public_app_url] = value }
    opts.on("--public-base-path VALUE", String) { |value| options[:public_base_path] = value }
    opts.on("--api-base-url VALUE", String) { |value| options[:api_base_url] = value }
  end
  parser.parse!(argv)
  options
rescue OptionParser::ParseError => error
  fail_with(error.message)
end

def required_option(options, key)
  value = options[key]
  fail_with("--#{key.to_s.tr("_", "-")} is required") unless value.is_a?(String) && !value.empty?
  value
end

def read_json(path, default = nil)
  return default unless path && File.file?(path)

  JSON.parse(File.read(path))
rescue JSON::ParserError => error
  fail_with("invalid JSON #{path}: #{error.message}")
end

def write_json(path, value)
  output = Pathname(path)
  output.dirname.mkpath
  temporary = Pathname("#{path}.tmp.#{$$}")
  File.write(temporary, JSON.pretty_generate(value) + "\n")
  File.rename(temporary, output)
end

def run_git(root, *args)
  stdout, stderr, status = Open3.capture3("git", "-C", root.to_s, *args)
  fail_with("git #{args.join(" ")} failed: #{stderr.strip}") unless status.success?

  stdout
end

def normalize_path(path)
  path.to_s.tr("\\", "/").sub(%r{\A\./}, "")
end

def ignored_deploy_path?(path)
  IGNORED_DEPLOY_PATHS.any? { |pattern| pattern.match?(path) }
end

def fingerprint_path?(path)
  FINGERPRINT_PATHS.any? { |pattern| pattern.match?(path) } && !ignored_deploy_path?(path)
end

def changed_paths(root)
  tracked = run_git(root, "diff", "--name-only", "HEAD", "--")
  untracked = run_git(root, "ls-files", "--others", "--exclude-standard")
  (tracked.lines + untracked.lines).map { |line| normalize_path(line.strip) }
    .reject(&:empty?)
    .uniq
end

def file_digest(path)
  Digest::SHA256.file(path).hexdigest
end

def worktree_fingerprint(root)
  paths = run_git(root, "ls-files", "-co", "--exclude-standard")
    .lines.map { |line| normalize_path(line.strip) }
    .select { |path| fingerprint_path?(path) }
    .select { |path| File.file?(root.join(path)) }
    .uniq.sort
  digest = Digest::SHA256.new
  paths.each do |path|
    digest.update(path)
    digest.update("\0")
    digest.update(file_digest(root.join(path)))
    digest.update("\n")
  end
  digest.hexdigest
end

def package_fingerprint(root, package_root)
  paths = Dir.glob(package_root.join("**", "*").to_s, File::FNM_DOTMATCH)
    .select { |path| File.file?(path) }
    .map { |path| normalize_path(Pathname(path).relative_path_from(root).to_s) }
    .select { |path| fingerprint_path?(path) }
    .sort
  digest = Digest::SHA256.new
  paths.each do |path|
    digest.update(path)
    digest.update("\0")
    digest.update(file_digest(root.join(path)))
    digest.update("\n")
  end
  digest.update("Cargo.lock\0") if root.join("Cargo.lock").file?
  digest.update(file_digest(root.join("Cargo.lock"))) if root.join("Cargo.lock").file?
  digest.hexdigest
end

def package_graph(root, metadata)
  packages = metadata.fetch("packages").select do |package|
    normalize_path(Pathname(package.fetch("manifest_path")).expand_path.relative_path_from(root).to_s)
      .start_with?("crates/")
  end
  names = packages.map { |package| package.fetch("name") }.to_h { |name| [name, true] }
  reverse = Hash.new { |hash, key| hash[key] = [] }
  packages.each do |package|
    package.fetch("dependencies", []).each do |dependency|
      dependency_name = dependency.fetch("name")
      reverse[dependency_name] << package.fetch("name") if names[dependency_name]
    end
  end
  roots = packages.to_h do |package|
    path = Pathname(package.fetch("manifest_path")).expand_path.dirname
    [package.fetch("name"), path]
  end
  [reverse, roots]
end

def package_for_path(root, package_roots, path)
  absolute = root.join(path).expand_path
  package_roots
    .select { |_name, package_root| absolute.to_s.start_with?("#{package_root}/") }
    .max_by { |_name, package_root| package_root.to_s.length }
    &.first
end

def affected_packages(changed_package_names, reverse)
  affected = changed_package_names.to_h { |name| [name, true] }
  queue = changed_package_names.dup
  until queue.empty?
    current = queue.shift
    reverse.fetch(current, []).each do |dependent|
      next if affected[dependent]

      affected[dependent] = true
      queue << dependent
    end
  end
  affected.keys.sort
end

def catalog(root)
  workers = read_json(root.join("crates/a4n-process-catalog/worker-catalog.json"), [])
  services = read_json(root.join("crates/a4n-process-catalog/service-catalog.json"), [])
  tools = read_json(root.join("crates/a4n-process-catalog/release-tools.json"), [])
  [workers, services, tools]
end

def artifact_inventory(root)
  workers, services, tools = catalog(root)
  artifacts = Hash.new { |hash, key| hash[key] = [] }
  roles = {}
  workers.each do |entry|
    package = entry.fetch("binary")
    artifacts[package] << "bin/services/#{package}"
    roles[package] = "stateful_worker"
  end
  services.each do |entry|
    package = entry.fetch("binary")
    artifacts[package] << "bin/services/#{package}"
    roles[package] ||= "sidecar"
  end
  tools.each do |entry|
    package = entry.fetch("package")
    next if package == "a4n-server"

    binary = entry.fetch("binary")
    artifacts[package] << "bin/#{binary}"
    roles[package] ||= "tool"
  end
  artifacts["a4n-server"] << "bin/a4n-server"
  roles["a4n-server"] = "host"
  roles["a4n-control"] = "host_child" if artifacts.key?("a4n-control")
  roles["a4n-edge"] = "host_child" if artifacts.key?("a4n-edge")
  artifacts.each_value(&:uniq!)
  [artifacts, roles]
end

def package_fingerprints(root, package_roots, package_names)
  package_names.to_h do |name|
    package_root = package_roots[name]
    [name, package_root ? package_fingerprint(root, package_root) : nil]
  end
end

def plan(options)
  root = Pathname(required_option(options, :root)).expand_path
  metadata = read_json(required_option(options, :metadata))
  state = read_json(options[:state], {}) || {}
  requested_mode = options.fetch(:mode, "auto")
  fail_with("unsupported mode: #{requested_mode}") unless %w[auto full].include?(requested_mode)

  changed = changed_paths(root)
  relevant_changed = changed.reject { |path| ignored_deploy_path?(path) }
  unknown = relevant_changed.reject { |path| fingerprint_path?(path) }
  reverse, package_roots = package_graph(root, metadata)
  changed_packages = relevant_changed.filter_map { |path| package_for_path(root, package_roots, path) }.uniq.sort
  affected = affected_packages(changed_packages, reverse)
  artifacts, roles = artifact_inventory(root)
  all_packages = artifacts.keys.sort
  full_reason = nil

  if requested_mode == "full"
    full_reason = "forced"
  elsif state["release_id"].to_s.empty?
    full_reason = "remote deployment state is missing"
  elsif options[:public_app_url] && state["public_app_url"] && state["public_app_url"] != options[:public_app_url]
    full_reason = "public app URL changed"
  elsif options[:public_base_path] && state["public_base_path"] && state["public_base_path"] != options[:public_base_path]
    full_reason = "public base path changed"
  elsif options[:api_base_url] && state["api_base_url"] && state["api_base_url"] != options[:api_base_url]
    full_reason = "API base URL changed"
  elsif options[:target] && state["target"] && state["target"] != options[:target]
    full_reason = "target platform changed"
  elsif options[:builder] && state["builder_image"] && state["builder_image"] != options[:builder]
    full_reason = "builder image changed"
  elsif state.key?("cargo_offline") && state["cargo_offline"] != (options[:cargo_offline] == "true")
    full_reason = "Cargo offline mode changed"
  elsif !unknown.empty?
    full_reason = "unclassified deployment paths: #{unknown.join(", ")}"
  elsif relevant_changed.any? { |path| path == "Cargo.toml" || path == "Cargo.lock" }
    full_reason = "workspace Cargo metadata changed"
  elsif changed_packages.any? { |name| FULL_CHANGE_PACKAGES.include?(name) || FULL_CHANGE_CRATES.include?(name) }
    full_reason = "a full-restart package or protocol changed"
  elsif relevant_changed.any? { |path| path.start_with?("prompts/") || path.start_with?("tools/onnxruntime/") }
    full_reason = "runtime prompt or ONNX input changed"
  elsif affected.any? { |name| roles[name] == "sidecar" }
    full_reason = "capability sidecar changed"
  end

  if full_reason
    mode = "full"
    selected_packages = all_packages
    artifact_paths = all_packages.flat_map { |name| artifacts.fetch(name, []) }
    payloads = %w[static prompts onnxruntime]
  else
    mode = relevant_changed.empty? ? "noop" : "delta"
    selected_packages = affected.select { |name| artifacts.key?(name) }
    artifact_paths = selected_packages.flat_map { |name| artifacts.fetch(name, []) }
    payloads = []
    payloads << "static" if relevant_changed.any? { |path| path.start_with?("frontend/") }
  end

  component_fingerprints = package_fingerprints(root, package_roots, all_packages)
  components = selected_packages.map do |package|
    {
      "package" => package,
      "role" => roles.fetch(package),
      "restart" => %w[stateful_worker host_child].include?(roles.fetch(package)) ? "component" : "full",
      "paths" => artifacts.fetch(package),
      "fingerprint" => component_fingerprints[package],
    }
  end

  {
    "schema_version" => SCHEMA_VERSION,
    "mode" => mode,
    "fallback_reason" => full_reason,
    "branch" => run_git(root, "branch", "--show-current").strip,
    "head_commit" => run_git(root, "rev-parse", "HEAD").strip,
    "dirty" => !changed.empty?,
    "worktree_fingerprint" => worktree_fingerprint(root),
    "changed_paths" => changed,
    "changed_packages" => changed_packages,
    "affected_packages" => affected,
    "build_packages" => selected_packages,
    "artifact_paths" => artifact_paths.uniq.sort,
    "payloads" => payloads,
    "components" => components,
    "component_fingerprints" => component_fingerprints,
    "base_release_id" => state["release_id"],
    "base_state_present" => !state.empty?,
    "target" => options[:target],
    "builder_image" => options[:builder],
    "cargo_offline" => options[:cargo_offline] == "true",
    "public_app_url" => options[:public_app_url],
    "public_base_path" => options[:public_base_path],
    "api_base_url" => options[:api_base_url],
  }
end

def manifest_release_id(path)
  return nil unless path && File.file?(path)

  Digest::SHA256.file(path).hexdigest
end

def record(options)
  plan = read_json(options[:plan], {}) || {}
  manifest = options[:manifest]
  result = plan.merge(
    "record_schema_version" => SCHEMA_VERSION,
    "deployment_id" => options[:deployment_id],
    "status" => options[:status],
    "exit_status" => options[:exit_status],
    "started_at" => options[:started_at],
    "finished_at" => options[:finished_at],
    "elapsed_seconds" => options[:elapsed_seconds],
    "build_elapsed_seconds" => options[:build_elapsed_seconds],
    "package_sha256" => options[:package_sha256],
    "package_size" => options[:package_size],
    "package_created_at" => options[:package_created_at],
    "remote" => options[:remote],
    "remote_dir" => options[:remote_dir],
    "service" => options[:service],
    "public_app_url" => options[:public_app_url],
    "public_base_path" => options[:public_base_path],
    "api_base_url" => options[:api_base_url],
    "release_id" => options[:release_id] || manifest_release_id(manifest),
  )
  write_json(required_option(options, :output), result)
end

def delta_manifest(options)
  plan = read_json(required_option(options, :plan))
  manifest = required_option(options, :manifest)
  target_release_id = manifest_release_id(manifest)
  fail_with("manifest is missing") unless target_release_id
  files = plan.fetch("artifact_paths").map do |path|
    { "path" => path, "kind" => path == "static" ? "directory" : "file" }
  end
  files.concat(plan.fetch("payloads").filter_map do |payload|
    next unless payload == "static"

    { "path" => payload, "kind" => "directory" }
  end)
  {
    "schema_version" => SCHEMA_VERSION,
    "mode" => "delta",
    "base_release_id" => plan["base_release_id"],
    "target_release_id" => target_release_id,
    "files" => files,
  }.tap { |value| write_json(required_option(options, :output), value) }
end

def emit(options)
  plan = read_json(required_option(options, :plan))
  case required_option(options, :output)
  when "build-packages"
    puts plan.fetch("build_packages")
  when "artifact-paths"
    puts plan.fetch("artifact_paths")
  when "payloads"
    puts plan.fetch("payloads")
  when "mode"
    puts plan.fetch("mode")
  when "fallback-reason"
    puts plan["fallback_reason"].to_s
  else
    fail_with("unknown emit value")
  end
end

command = ARGV.shift
options = options_for(ARGV)
case command
when "plan"
  write_json(required_option(options, :output), plan(options))
when "record"
  record(options)
when "delta"
  delta_manifest(options)
when "emit"
  emit(options)
else
  fail_with("usage: deploy-plan.rb plan|record|delta|emit [options]")
end
