#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

sidecar_binaries="$(sidecar_catalog_binaries)"
first_sidecar_binary="${sidecar_binaries%%$'\n'*}"
tool_binaries="$(release_tool_binaries)"
first_tool_binary="${tool_binaries%%$'\n'*}"

tmp_dir="$(make_target_temp_dir release-manifest-check)"
cleanup() { rm -rf "$tmp_dir"; }
trap cleanup EXIT

mkdir -p "$tmp_dir/bin/services"
printf 'host-binary\n' > "$tmp_dir/bin/a4n-tauri"
while IFS= read -r tool; do
    printf '%s\n' "$tool" > "$tmp_dir/bin/$tool"
done <<<"$tool_binaries"
printf 'onnx-runtime\n' > "$tmp_dir/bin/services/libonnxruntime.so.1"
while IFS= read -r binary; do
    printf '%s\n' "$binary" > "$tmp_dir/bin/services/$binary"
done <<<"$sidecar_binaries"

ruby scripts/release_manifest.rb generate \
    --package-root "$tmp_dir" \
    --target linux-x86_64 \
    --package a4n-tauri \
    --output "$tmp_dir/release-manifest.json"
ruby scripts/release_manifest.rb verify \
    --package-root "$tmp_dir" \
    --manifest "$tmp_dir/release-manifest.json"

printf 'host-binary\n' > "$tmp_dir/bin/a4n-tauri.exe"
while IFS= read -r tool; do
    printf '%s\n' "$tool" > "$tmp_dir/bin/$tool.exe"
done <<<"$tool_binaries"
printf 'onnx-runtime\n' > "$tmp_dir/bin/services/onnxruntime.dll"
while IFS= read -r binary; do
    printf '%s\n' "$binary" > "$tmp_dir/bin/services/$binary.exe"
done <<<"$sidecar_binaries"
ruby scripts/release_manifest.rb generate \
    --package-root "$tmp_dir" \
    --target windows-x86_64 \
    --package a4n-tauri \
    --output "$tmp_dir/release-manifest-windows.json"
ruby scripts/release_manifest.rb verify \
    --package-root "$tmp_dir" \
    --manifest "$tmp_dir/release-manifest-windows.json"

tauri_app="$tmp_dir/A4N.app"
mkdir -p "$tauri_app/Contents/MacOS" "$tauri_app/Contents/Resources"
printf 'signed-host\n' > "$tauri_app/Contents/MacOS/a4n-tauri"
while IFS= read -r tool; do
    printf '%s-signed\n' "$tool" > "$tauri_app/Contents/MacOS/$tool"
done <<<"$tool_binaries"
printf 'signed-runtime\n' > "$tauri_app/Contents/Resources/libonnxruntime.dylib"
while IFS= read -r binary; do
    printf '%s-signed\n' "$binary" > "$tauri_app/Contents/MacOS/$binary"
done <<<"$sidecar_binaries"
ruby scripts/release_manifest.rb generate \
    --package-root "$tmp_dir" \
    --target aarch64-apple-darwin \
    --package a4n-tauri \
    --tauri-app "$tauri_app" \
    --output "$tmp_dir/release-manifest-tauri.json"
ruby scripts/release_manifest.rb verify \
    --package-root "$tmp_dir" \
    --manifest "$tmp_dir/release-manifest-tauri.json"
printf 'tampered\n' >> "$tauri_app/Contents/MacOS/$first_sidecar_binary"
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

cp "$tmp_dir/release-manifest.json" "$tmp_dir/release-manifest.valid.json"
ruby -rjson -e '
  path = ARGV.fetch(0)
  data = JSON.parse(File.read(path))
  data["version"] = "different-version"
  File.write(path, JSON.pretty_generate(data) + "\n")
' "$tmp_dir/release-manifest.json"
if ruby scripts/release_manifest.rb verify --package-root "$tmp_dir" --manifest "$tmp_dir/release-manifest.json" >/dev/null 2>&1; then
    echo "manifest/host version mismatch unexpectedly passed verification" >&2
    exit 1
fi
mv "$tmp_dir/release-manifest.valid.json" "$tmp_dir/release-manifest.json"

printf 'tampered\n' >> "$tmp_dir/bin/$first_tool_binary"
if ruby scripts/release_manifest.rb verify --package-root "$tmp_dir" --manifest "$tmp_dir/release-manifest.json" >/dev/null 2>&1; then
    echo "tampered release tool unexpectedly passed manifest verification" >&2
    exit 1
fi
printf '%s\n' "$first_tool_binary" > "$tmp_dir/bin/$first_tool_binary"

printf 'tampered\n' >> "$tmp_dir/bin/services/libonnxruntime.so.1"
if ruby scripts/release_manifest.rb verify --package-root "$tmp_dir" --manifest "$tmp_dir/release-manifest.json" >/dev/null 2>&1; then
    echo "tampered ONNX Runtime unexpectedly passed manifest verification" >&2
    exit 1
fi
printf 'onnx-runtime\n' > "$tmp_dir/bin/services/libonnxruntime.so.1"

printf 'tampered\n' >> "$tmp_dir/bin/services/$first_sidecar_binary"
if ruby scripts/release_manifest.rb verify --package-root "$tmp_dir" --manifest "$tmp_dir/release-manifest.json" >/dev/null 2>&1; then
    echo "tampered release binary unexpectedly passed manifest verification" >&2
    exit 1
fi

echo "release manifest generation and tamper check passed"
