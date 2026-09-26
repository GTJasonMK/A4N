#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

ruby -rjson -e '
  workers = JSON.parse(File.read("crates/process/a4n-process-catalog/worker-catalog.json"))
  release_tools = JSON.parse(File.read("crates/process/a4n-process-catalog/release-tools.json"))
  abort "worker catalog must not be empty" unless workers.is_a?(Array) && !workers.empty?
  release_packages = release_tools.to_h { |tool| [tool.fetch("package"), true] }

  names = {}
  owners = {}
  workers.each do |worker|
    name = worker.fetch("name")
    abort "duplicate worker name: #{name}" if names.key?(name)
    names[name] = true
    abort "#{name} still declares data_scope" if worker.key?("data_scope")
    dependencies = worker.fetch("dependencies", [])
    abort "#{name}.dependencies contains duplicates" unless dependencies.uniq == dependencies
    dependencies.each do |dependency|
      abort "#{name} depends on unreleasable tool: #{dependency}" unless release_packages[dependency]
    end

    %w[read_scope write_scope].each do |field|
      scopes = worker.fetch(field)
      abort "#{name}.#{field} must be an array" unless scopes.is_a?(Array)
      abort "#{name}.#{field} contains invalid or duplicate scopes" unless
        scopes.all? { |scope| scope.is_a?(String) && scope.match?(/\A[a-z][a-z0-9_]*\z/) } &&
        scopes.uniq == scopes
    end

    worker.fetch("write_scope").each do |scope|
      previous = owners[scope]
      abort "#{scope} has multiple write owners: #{previous}, #{name}" if previous
      owners[scope] = name
    end
  end

  puts "Worker ownership catalog passed: #{workers.length} workers, #{owners.length} write scopes."
'
