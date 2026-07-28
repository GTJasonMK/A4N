#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORKFLOW="$ROOT/.github/workflows/release.yml"
MODULE_WORKFLOW="$ROOT/.github/workflows/module-checks.yml"
BUNDLE_CONFIG="$ROOT/crates/a4n-tauri/tauri.sidecars.conf.json"
SBOM_SCRIPT="$ROOT/scripts/checks/generate-release-sbom.sh"

for workflow in "$WORKFLOW" "$MODULE_WORKFLOW"; do
    [[ -r "$workflow" ]] || {
        echo "missing workflow: $workflow" >&2
        exit 1
    }
done
[[ -x "$SBOM_SCRIPT" ]] || {
    echo "missing executable release SBOM generator: $SBOM_SCRIPT" >&2
    exit 1
}

ruby -e 'require "yaml"; ARGV.each { |path| YAML.safe_load(File.read(path), aliases: true) }' "$WORKFLOW" "$MODULE_WORKFLOW"
ruby -ryaml -e '
  def require_pinned_actions(path)
    workflow = YAML.safe_load(File.read(path), aliases: true)
    workflow.fetch("jobs").each do |job_name, job|
      job.fetch("steps", []).each do |step|
        uses = step["uses"]
        next unless uses
        next if uses.start_with?("./") || uses.match?(/\A[^@\s]+@[0-9a-f]{40}\z/)
        abort "workflow action is not pinned to a commit: #{File.basename(path)}: #{job_name}: #{uses}"
      end
    end
    workflow
  end

  release = require_pinned_actions(ARGV.fetch(0))
  module_checks = require_pinned_actions(ARGV.fetch(1))
  abort "release workflow default contents permission must be read" unless release.dig("permissions", "contents") == "read"
  abort "module checks workflow default contents permission must be read" unless module_checks.dig("permissions", "contents") == "read"
  release_checkouts = release.fetch("jobs").values.flat_map do |job|
    job.fetch("steps", []).select { |step| step["uses"]&.start_with?("actions/checkout@") }
  end
  unless release_checkouts.size == 4 && release_checkouts.all? { |step| step.dig("with", "persist-credentials") == false }
    abort "every release checkout must disable credential persistence"
  end
  write_jobs = release.fetch("jobs").filter_map do |job_name, job|
    job_name if job.dig("permissions", "contents") == "write"
  end
  abort "only macOS and Windows release jobs may write contents" unless write_jobs.sort == %w[macos-arm64 windows-x86_64]
  release.fetch("jobs").each do |job_name, job|
    job.fetch("services", {}).each do |service_name, service|
      image = service.fetch("image")
      next if image.match?(/\A[^@\s]+@sha256:[0-9a-f]{64}\z/)
      abort "release workflow service image is not pinned to a digest: #{job_name}: #{service_name}: #{image}"
    end
  end
' "$WORKFLOW" "$MODULE_WORKFLOW"
ruby -rjson -e 'abort "macOS hardened runtime must be enabled" unless JSON.parse(File.read(ARGV.fetch(0))).dig("bundle", "macOS", "hardenedRuntime") == true' "$BUNDLE_CONFIG"

required=(
    "- 'v*'"
    "contents: read"
    "contents: write"
    "environment: release"
    'supply-chain:'
    'name: Release advisory, license and SBOM gate'
    'runs-on: ubuntu-24.04'
    'cargo audit'
    'cargo deny check'
    'generate-release-sbom.sh artifacts/supply-chain "$GITHUB_REF_NAME"'
    'A4N-${GITHUB_REF_NAME}-cyclonedx.tar.gz'
    'name: release-supply-chain'
    'linux-x86_64:'
    'name: Linux x86_64 portable release'
    'needs: supply-chain'
    'postgres:16@sha256:33f923b05f64ca54ac4401c01126a6b92afe839a0aa0a52bc5aeb5cc958e5f20'
    'qdrant-x86_64-unknown-linux-gnu.tar.gz'
    '60663a254cf421dba4db45710872895cd3a714fe1e6978f7927923b5cfae4718'
    'smoke-linux-release-package.sh'
    'name: linux-x86_64-release'
    'needs: linux-x86_64'
    'A4N-${GITHUB_REF_NAME}-linux-x86_64.tar.gz'
    "runs-on: macos-15"
    'APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}'
    'APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}'
    'APPLE_ID: ${{ secrets.APPLE_ID }}'
    'APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}'
    'APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}'
    'Developer ID Application:'
    '--ci'
    '--bundles app,dmg'
    'codesign --verify --deep --strict'
    'xcrun stapler validate'
    'spctl --assess --type execute'
    'release_manifest.rb generate'
    '--tauri-app "$app"'
    'security cms -S -u 6 -H SHA256 -Y NONE'
    '-N "$APPLE_SIGNING_IDENTITY"'
    'security cms -D -u 6'
    'cmp "$manifest"'
    'hdiutil attach'
    '--package-root "$mountpoint"'
    'smoke-macos-tauri-bundle.sh'
    'gh release create'
    '--draft'
    '--json isDraft'
    'gh release download'
    'shasum -a 256 -c'
    'needs: macos-arm64'
    'runs-on: windows-2025'
    'AZURE_CLIENT_ID: ${{ secrets.AZURE_CLIENT_ID }}'
    'AZURE_CLIENT_SECRET: ${{ secrets.AZURE_CLIENT_SECRET }}'
    'AZURE_TENANT_ID: ${{ secrets.AZURE_TENANT_ID }}'
    'AZURE_ARTIFACT_SIGNING_ENDPOINT: ${{ vars.AZURE_ARTIFACT_SIGNING_ENDPOINT }}'
    'AZURE_ARTIFACT_SIGNING_CERTIFICATE_PROFILE: ${{ vars.AZURE_ARTIFACT_SIGNING_CERTIFICATE_PROFILE }}'
    '$env:AZURE_CLI_PATH = (Get-Command az.cmd -ErrorAction Stop).Source'
    '$env:SIGNTOOL_PATH = $signTools[0].FullName'
    'cargo install artifact-signing-cli --version 0.11.0 --locked'
    'Get-AuthenticodeSignature'
    'deploy.sh --target windows-x86_64 --no-build'
    'smoke-windows-release-package.ps1'
    'name: Publish complete verified GitHub release'
    'release draft does not contain the exact verified asset set'
    'gh release edit $tag --draft=false'
)
for text in "${required[@]}"; do
    rg -Fq -- "$text" "$WORKFLOW" || {
        echo "signed release workflow is missing: $text" >&2
        exit 1
    }
done

sbom_required=(
    'expected host and eight sidecar packages'
    'cargo metadata --no-deps --locked --format-version 1'
    'cargo cyclonedx'
    '--all-features'
    '--target all'
    'release SBOM set must contain host and eight sidecars'
)
for text in "${sbom_required[@]}"; do
    rg -Fq -- "$text" "$SBOM_SCRIPT" || {
        echo "release SBOM generator is missing: $text" >&2
        exit 1
    }
done

[[ "$(rg -c 'security cms -D -u 6' "$WORKFLOW")" -eq 3 ]] || {
    echo "every release manifest CMS verification must use the object-signer policy" >&2
    exit 1
}

if rg -n 'continue-on-error|--skip-stapling|--no-sign|Import-PfxCertificate|certificateThumbprint|AZURE_ARTIFACT_SIGNING_PROFILE|Get-Command[^\n]*signtool|signingIdentity[[:space:]]*:[[:space:]]*"?-"?' "$WORKFLOW"; then
    echo "signed release workflow contains an unsigned or best-effort fallback" >&2
    exit 1
fi
if rg -n 'macos-15-arm64' "$ROOT/.github/workflows" -g '*.yml'; then
    echo "GitHub arm64 macOS runner label must be macos-15" >&2
    exit 1
fi

echo "cross-platform signed release workflow is fail-closed"
