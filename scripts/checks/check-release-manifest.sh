#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

command -v ruby >/dev/null 2>&1 || {
    echo "ruby is required for release manifest checks" >&2
    exit 1
}

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/a4n-release-manifest.XXXXXX")"
cleanup() { rm -rf "$tmp_dir"; }
trap cleanup EXIT

mkdir -p "$tmp_dir/bin/services"
printf 'host-binary\n' > "$tmp_dir/bin/a4n-tauri"
printf 'onnx-runtime\n' > "$tmp_dir/bin/services/libonnxruntime.so.1"
ruby -rjson -e 'JSON.parse(File.read("crates/a4n-service-protocol/service-catalog.json")).each { |entry| File.write(ARGV.fetch(0) + "/bin/services/" + entry.fetch("binary"), entry.fetch("name") + "\n") }' "$tmp_dir"

ruby scripts/release_manifest.rb generate \
    --package-root "$tmp_dir" \
    --target linux-x86_64 \
    --package a4n-tauri \
    --host a4n-tauri \
    --catalog crates/a4n-service-protocol/service-catalog.json \
    --output "$tmp_dir/release-manifest.json"
ruby scripts/release_manifest.rb verify \
    --package-root "$tmp_dir" \
    --manifest "$tmp_dir/release-manifest.json" \
    --catalog crates/a4n-service-protocol/service-catalog.json

printf 'host-binary\n' > "$tmp_dir/bin/a4n-tauri.exe"
printf 'onnx-runtime\n' > "$tmp_dir/bin/services/onnxruntime.dll"
ruby -rjson -e 'JSON.parse(File.read("crates/a4n-service-protocol/service-catalog.json")).each { |entry| File.write(ARGV.fetch(0) + "/bin/services/" + entry.fetch("binary") + ".exe", entry.fetch("name") + "\n") }' "$tmp_dir"
ruby scripts/release_manifest.rb generate \
    --package-root "$tmp_dir" \
    --target windows-x86_64 \
    --package a4n-tauri \
    --host a4n-tauri \
    --catalog crates/a4n-service-protocol/service-catalog.json \
    --output "$tmp_dir/release-manifest-windows.json"
ruby scripts/release_manifest.rb verify \
    --package-root "$tmp_dir" \
    --manifest "$tmp_dir/release-manifest-windows.json" \
    --catalog crates/a4n-service-protocol/service-catalog.json

tauri_app="$tmp_dir/A4N.app"
mkdir -p "$tauri_app/Contents/MacOS" "$tauri_app/Contents/Resources"
printf 'signed-host\n' > "$tauri_app/Contents/MacOS/a4n-tauri"
printf 'signed-runtime\n' > "$tauri_app/Contents/Resources/libonnxruntime.dylib"
ruby -rjson -e 'JSON.parse(File.read("crates/a4n-service-protocol/service-catalog.json")).each { |entry| File.write(ARGV.fetch(0) + "/Contents/MacOS/" + entry.fetch("binary"), entry.fetch("name") + "-signed\n") }' "$tauri_app"
ruby scripts/release_manifest.rb generate \
    --package-root "$tmp_dir" \
    --target aarch64-apple-darwin \
    --package a4n-tauri \
    --host a4n-tauri \
    --catalog crates/a4n-service-protocol/service-catalog.json \
    --tauri-app "$tauri_app" \
    --output "$tmp_dir/release-manifest-tauri.json"
ruby scripts/release_manifest.rb verify \
    --package-root "$tmp_dir" \
    --manifest "$tmp_dir/release-manifest-tauri.json" \
    --catalog crates/a4n-service-protocol/service-catalog.json
printf 'tampered\n' >> "$tauri_app/Contents/MacOS/a4n-prompt-service"
if ruby scripts/release_manifest.rb verify --package-root "$tmp_dir" --manifest "$tmp_dir/release-manifest-tauri.json" >/dev/null 2>&1; then
    echo "tampered Tauri sidecar unexpectedly passed manifest verification" >&2
    exit 1
fi

cp "$tmp_dir/release-manifest.json" "$tmp_dir/release-manifest.valid.json"
ruby -rjson -e '
  path = ARGV.fetch(0)
  data = JSON.parse(File.read(path))
  data.fetch("binaries").fetch(1)["path"] = "bin/../escape"
  File.write(path, JSON.pretty_generate(data) + "\n")
' "$tmp_dir/release-manifest.json"
if ruby scripts/release_manifest.rb verify --package-root "$tmp_dir" --manifest "$tmp_dir/release-manifest.json" >/dev/null 2>&1; then
    echo "manifest traversal unexpectedly passed verification" >&2
    exit 1
fi
mv "$tmp_dir/release-manifest.valid.json" "$tmp_dir/release-manifest.json"

printf 'tampered\n' >> "$tmp_dir/bin/services/libonnxruntime.so.1"
if ruby scripts/release_manifest.rb verify --package-root "$tmp_dir" --manifest "$tmp_dir/release-manifest.json" >/dev/null 2>&1; then
    echo "tampered ONNX Runtime unexpectedly passed manifest verification" >&2
    exit 1
fi
printf 'onnx-runtime\n' > "$tmp_dir/bin/services/libonnxruntime.so.1"

printf 'tampered\n' >> "$tmp_dir/bin/services/a4n-prompt-service"
if ruby scripts/release_manifest.rb verify --package-root "$tmp_dir" --manifest "$tmp_dir/release-manifest.json" >/dev/null 2>&1; then
    echo "tampered release binary unexpectedly passed manifest verification" >&2
    exit 1
fi

echo "release manifest generation and tamper check passed"
