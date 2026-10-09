#!/usr/bin/env ruby

require "base64"
require "json"
require "open3"
require "optparse"
require "tmpdir"
require "tempfile"
require "uri"

options = {}
OptionParser.new do |parser|
  parser.banner = "Usage: check-updater-release.rb --assets DIR --manifest FILE --version VERSION --tag TAG --repository OWNER/REPO --tauri-config FILE"
  parser.on("--assets DIR") { |value| options[:assets] = value }
  parser.on("--manifest FILE") { |value| options[:manifest] = value }
  parser.on("--version VERSION") { |value| options[:version] = value }
  parser.on("--tag TAG") { |value| options[:tag] = value }
  parser.on("--repository OWNER/REPO") { |value| options[:repository] = value }
  parser.on("--tauri-config FILE") { |value| options[:tauri_config] = value }
end.parse!

missing = %i[assets manifest version tag repository tauri_config].reject { |key| options[key] }
abort "missing options: #{missing.join(", ")}" unless missing.empty?
manifest = JSON.parse(File.read(options.fetch(:manifest)))
abort "release manifest version mismatch" unless manifest["version"] == options[:version]
abort "release manifest has no data_migrations array" unless manifest["data_migrations"].is_a?(Array)
abort "release manifest platform targets are invalid" unless manifest["platforms"]&.keys&.sort == %w[darwin-aarch64-app linux-x86_64-deb linux-x86_64-rpm windows-x86_64-msi]

config = JSON.parse(File.read(options.fetch(:tauri_config)))
encoded_public_key = config.dig("plugins", "updater", "pubkey")
abort "Tauri updater public key is missing" unless encoded_public_key.is_a?(String) && !encoded_public_key.empty?
public_key = Base64.decode64(encoded_public_key)

Dir.mktmpdir("a4n-updater-key") do |directory|
  public_key_path = File.join(directory, "updater.pub")
  File.binwrite(public_key_path, public_key)
  {
    "windows-x86_64-msi" => "*.msi.zip",
    "darwin-aarch64-app" => "*.app.tar.gz",
    "linux-x86_64-deb" => "*.deb",
    "linux-x86_64-rpm" => "*.rpm"
  }.each do |target, pattern|
    platform = manifest.fetch("platforms").fetch(target)
    uri = URI(platform.fetch("url"))
    asset_name = File.basename(uri.path)
    expected_url = "https://github.com/#{options[:repository]}/releases/download/#{options[:tag]}/#{asset_name}"
    abort "updater URL mismatch for #{target}" unless platform["url"] == expected_url
    asset_path = File.join(options[:assets], asset_name)
    signature_path = "#{asset_path}.sig"
    abort "missing updater asset: #{asset_name}" unless File.file?(asset_path)
    abort "missing updater signature: #{File.basename(signature_path)}" unless File.file?(signature_path)
    abort "signature differs from latest.json for #{target}" unless File.read(signature_path).strip == platform["signature"]

    _stdout, stderr, status = Open3.capture3("minisign", "-Vm", asset_path, "-x", signature_path, "-p", public_key_path)
    abort "signature verification failed for #{target}: #{stderr.strip}" unless status.success?
  end

  { "*.msi" => "Windows MSI", "*.dmg" => "macOS DMG", "*.deb" => "Linux deb", "*.rpm" => "Linux rpm" }.each do |pattern, label|
    matches = Dir.glob(File.join(options[:assets], pattern)).select { |path| File.file?(path) }
    abort "expected one #{label}, found #{matches.length}" unless matches.length == 1
  end
end

puts "updater manifest assets and signatures verified"
