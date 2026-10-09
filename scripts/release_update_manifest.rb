#!/usr/bin/env ruby

require "json"
require "optparse"
require "time"

options = {}
OptionParser.new do |parser|
  parser.banner = "Usage: release_update_manifest.rb --version VERSION --tag TAG --repository OWNER/REPO --assets DIR --notes FILE --migrations FILE --output FILE"
  parser.on("--version VERSION") { |value| options[:version] = value }
  parser.on("--tag TAG") { |value| options[:tag] = value }
  parser.on("--repository OWNER/REPO") { |value| options[:repository] = value }
  parser.on("--assets DIR") { |value| options[:assets] = value }
  parser.on("--notes FILE") { |value| options[:notes] = value }
  parser.on("--migrations FILE") { |value| options[:migrations] = value }
  parser.on("--output FILE") { |value| options[:output] = value }
end.parse!

missing = %i[version tag repository assets notes migrations output].reject { |key| options[key] }
abort "missing options: #{missing.join(", ")}" unless missing.empty?
abort "invalid release version" unless options[:version].match?(/\A\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?\z/)
abort "tag must match v#{options[:version]}" unless options[:tag] == "v#{options[:version]}"

def find_asset(root, pattern)
  matches = Dir.glob(File.join(root, "**", pattern)).select { |path| File.file?(path) }
  abort "expected one #{pattern} updater asset, found #{matches.length}" unless matches.length == 1
  matches.first
end

def signed_asset(root, pattern, repository, tag)
  path = find_asset(root, pattern)
  signature_path = "#{path}.sig"
  abort "missing updater signature: #{signature_path}" unless File.file?(signature_path)
  signature = File.read(signature_path).strip
  abort "empty updater signature: #{signature_path}" if signature.empty?
  name = File.basename(path)
  [name, {
    "url" => "https://github.com/#{repository}/releases/download/#{tag}/#{name}",
    "signature" => signature
  }]
end

asset_root = options.fetch(:assets)
repository = options.fetch(:repository)
tag = options.fetch(:tag)
platforms = {}
platforms["windows-x86_64-msi"] = signed_asset(asset_root, "*.msi.zip", repository, tag).last
platforms["darwin-aarch64-app"] = signed_asset(asset_root, "*.app.tar.gz", repository, tag).last
platforms["linux-x86_64-deb"] = signed_asset(asset_root, "*.deb", repository, tag).last
platforms["linux-x86_64-rpm"] = signed_asset(asset_root, "*.rpm", repository, tag).last

%w[*.msi *.dmg *.deb *.rpm].each { |pattern| find_asset(asset_root, pattern) }
migrations = JSON.parse(File.read(options.fetch(:migrations)))
abort "data migrations must be a unique array of names" unless migrations.is_a?(Array) && migrations.all? { |name| name.is_a?(String) && !name.empty? } && migrations.uniq == migrations

manifest = {
  "version" => options.fetch(:version),
  "notes" => File.read(options.fetch(:notes)),
  "pub_date" => Time.now.utc.iso8601,
  "data_migrations" => migrations,
  "platforms" => platforms
}
File.write(options.fetch(:output), JSON.pretty_generate(manifest) + "\n")
