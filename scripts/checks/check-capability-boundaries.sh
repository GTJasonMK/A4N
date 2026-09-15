#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

ALLOWLIST="$SCRIPT_DIR/capability-client-allowlist.tsv"
[[ -r "$ALLOWLIST" ]] || {
    printf 'capability boundary allowlist is missing: %s\n' "$ALLOWLIST" >&2
    exit 1
}

metadata_file="$(mktemp "${TMPDIR:-/tmp}/a4n-capability-boundaries.XXXXXX.json")"
trap 'rm -f "$metadata_file"' EXIT
cargo metadata --locked --format-version 1 --no-deps >"$metadata_file"

ruby -rjson -rset - "$metadata_file" "$ALLOWLIST" <<'RUBY'
metadata_path, allowlist_path = ARGV
metadata = JSON.parse(File.read(metadata_path))
packages = metadata.fetch("packages").to_h { |package| [package.fetch("name"), package] }
workspace_names = packages.keys.to_set

allowlist = Hash.new { |rules, name| rules[name] = [] }
File.foreach(allowlist_path).with_index(1) do |raw_line, line_number|
  line = raw_line.chomp
  next if line.empty? || line.start_with?("#")

  fields = line.split("\t", -1)
  abort "invalid capability allowlist row #{line_number}: expected 4 tab-separated fields" unless fields.length == 4

  rule, subject, dependency, kind = fields
  abort "invalid dependency kind in capability allowlist row #{line_number}: #{kind}" unless %w[normal dev build source].include?(kind)
  allowlist[rule] << [subject, dependency, kind]
end

errors = []

dependency_entries = lambda do |package_name|
  packages.fetch(package_name).fetch("dependencies").map do |dependency|
    [dependency.fetch("name"), dependency.fetch("kind", "normal") || "normal"]
  end
end

workspace_dependency_pairs = lambda do |package_name, &filter|
  dependency_entries.call(package_name).select do |dependency, kind|
    workspace_names.include?(dependency) && (filter.nil? || filter.call(dependency, kind))
  end
end

allowlist_pairs = lambda do |rule, subject, dependency = nil|
  allowlist.fetch(rule).select { |row_subject, row_dependency, _| row_subject == subject && (dependency.nil? || row_dependency == dependency) }.map { |_, row_dependency, kind| [row_dependency, kind] }.sort
end

compare_exact = lambda do |label, actual, expected|
  actual = actual.sort
  expected = expected.sort
  errors << "#{label}: unexpected=#{(actual - expected).inspect} missing=#{(expected - actual).inspect}" unless actual == expected
end

legacy_contract_dependencies = lambda do |package_name|
  allowlist_pairs.call("legacy-contract-dependency", package_name)
end

capability_contracts = %w[
  a4n-prompt-contract
  a4n-embedding-contract
  a4n-rag-contract
  a4n-rag-sidecar-contract
  a4n-image-contract
  a4n-video-contract
  a4n-video-sidecar-contract
  a4n-import-contract
  a4n-export-contract
]

forbidden_contract_dependency = lambda do |dependency|
  explicit = %w[
    a4n-archive-core
    a4n-db
    a4n-db-models
    a4n-domain
    a4n-embedding
    a4n-embedding-fastembed
    a4n-image
    a4n-image-providers
    a4n-import-core
    a4n-llm
    a4n-llm-http
    a4n-model-discovery
    a4n-orchestrator
    a4n-prompt
    a4n-prompt-git
    a4n-rag-core
    a4n-rag-index
    a4n-runtime
    a4n-service-client
    a4n-service-manager
    a4n-sidecar-bootstrap
    a4n-video
    a4n-video-providers
  ]
  dependency.end_with?("-core") || dependency.end_with?("-providers") || dependency.end_with?("-service") ||
    dependency.end_with?("-client") || dependency == "a4n-llm-gateway" || explicit.include?(dependency)
end

capability_contracts.each do |package_name|
  next unless packages.key?(package_name)

  exceptions = legacy_contract_dependencies.call(package_name)
  actual_forbidden = workspace_dependency_pairs.call(package_name) do |dependency, _kind|
    forbidden_contract_dependency.call(dependency)
  end
  allowed_exceptions = actual_forbidden.select { |pair| exceptions.include?(pair) }
  unexpected = actual_forbidden - allowed_exceptions
  errors << "#{package_name} has forbidden direct dependencies: #{unexpected.inspect}" unless unexpected.empty?
end

%w[a4n-orchestrator a4n-runtime].each do |package_name|
  next unless packages.key?(package_name)

  clients = workspace_dependency_pairs.call(package_name) do |dependency, _kind|
    dependency.end_with?("-client")
  end
  compare_exact.call(
    "#{package_name} capability client dependencies",
    clients,
    allowlist_pairs.call(package_name == "a4n-orchestrator" ? "orchestrator-client" : "runtime-client", package_name),
  )
end

client_packages = packages.keys.select { |name| name.start_with?("a4n-") && name.end_with?("-client") && name != "a4n-service-client" }
client_packages.each do |package_name|
  expected = allowlist_pairs.call("client-dependency", package_name)
  errors << "#{package_name} is missing from client dependency allowlist" if expected.empty?
  actual = workspace_dependency_pairs.call(package_name)
  compare_exact.call("#{package_name} workspace dependencies", actual, expected)
end

if packages.key?("a4n-service-client")
  compare_exact.call(
    "a4n-service-client workspace dependencies",
    workspace_dependency_pairs.call("a4n-service-client"),
    allowlist_pairs.call("transport-dependency", "a4n-service-client"),
  )
end

service_packages = %w[
  a4n-prompt-service
  a4n-llm-gateway
  a4n-embedding-service
  a4n-rag-service
  a4n-image-service
  a4n-video-service
  a4n-import-service
  a4n-export-service
]
service_packages.each do |package_name|
  next unless packages.key?(package_name)

  dependency_entries.call(package_name).each do |dependency, kind|
    if dependency.end_with?("-client") && kind != "dev"
      errors << "#{package_name} depends on client #{dependency} (#{kind})"
    end
  end
end

service_packages.each do |package_name|
  next unless packages.key?(package_name)

  binary_dependencies = dependency_entries.call(package_name).map(&:first) & service_packages
  binary_dependencies -= [package_name]
  errors << "#{package_name} depends on another service binary: #{binary_dependencies.inspect}" unless binary_dependencies.empty?
end

source_files = Dir["crates/**/*.rs"]
registry_hits = source_files.select do |path|
  content = File.read(path)
  content.include?("A4nCapabilities") || content.include?("a4n_capabilities")
end.sort
allowed_registry_files = allowlist.fetch("legacy-registry-file", []).map { |subject, _, _| subject }.sort
compare_exact.call("legacy capability registry source files", registry_hits, allowed_registry_files)

equivalent_registry_hits = source_files.select do |path|
  File.read(path).match?(/\b(?:AllClients|CapabilityRegistry|RuntimeClients)\b/)
end
errors << "new aggregate registry names found: #{equivalent_registry_hits.inspect}" unless equivalent_registry_hits.empty?

if errors.empty?
  puts "capability boundary checks passed: contracts=#{capability_contracts.count { |name| packages.key?(name) }} clients=#{client_packages.length} services=#{service_packages.count { |name| packages.key?(name) }}"
else
  warn errors.map { |error| "- #{error}" }
  abort "capability boundary checks failed"
end
RUBY
