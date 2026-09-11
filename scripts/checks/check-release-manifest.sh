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
export_sidecar_binary=""
first_unselected_binary=""
while IFS=$'\t' read -r service binary _; do
    if [[ "$service" == "export" ]]; then
        export_sidecar_binary="$binary"
    elif [[ -z "$first_unselected_binary" ]]; then
        first_unselected_binary="$binary"
    fi
done < <(sidecar_catalog_rows)
[[ -n "$export_sidecar_binary" && -n "$first_unselected_binary" ]] || {
    echo "release manifest check requires export and another catalog service" >&2
    exit 1
}

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
(cd "$tmp_dir" && sha256sum -c release-manifest.sha256 >/dev/null)
rg -q '^[0-9a-fA-F]{64}  release-manifest\.json$' "$tmp_dir/release-manifest.sha256"
printf 'tampered\n' >> "$tmp_dir/bin/services/$first_sidecar_binary"
if (cd "$tmp_dir" && sha256sum -c release-manifest.sha256 >/dev/null 2>&1); then
    echo "tampered release binary unexpectedly passed checksum verification" >&2
    exit 1
fi
printf '%s\n' "$first_sidecar_binary" > "$tmp_dir/bin/services/$first_sidecar_binary"

ruby scripts/release_manifest.rb generate \
    --package-root "$tmp_dir" \
    --target linux-x86_64 \
    --package a4n-tauri \
    --services export \
    --output "$tmp_dir/release-manifest-export.json"
ruby scripts/release_manifest.rb verify \
    --package-root "$tmp_dir" \
    --manifest "$tmp_dir/release-manifest-export.json"

mv "$tmp_dir/bin/services/$export_sidecar_binary" "$tmp_dir/bin/services/$export_sidecar_binary.missing"
if ruby scripts/release_manifest.rb verify \
    --package-root "$tmp_dir" \
    --manifest "$tmp_dir/release-manifest-export.json" >/dev/null 2>&1; then
    echo "selected service missing unexpectedly passed manifest verification" >&2
    exit 1
fi
mv "$tmp_dir/bin/services/$export_sidecar_binary.missing" "$tmp_dir/bin/services/$export_sidecar_binary"

mv "$tmp_dir/bin/services/$first_unselected_binary" "$tmp_dir/bin/services/$first_unselected_binary.missing"
ruby scripts/release_manifest.rb verify \
    --package-root "$tmp_dir" \
    --manifest "$tmp_dir/release-manifest-export.json"
mv "$tmp_dir/bin/services/$first_unselected_binary.missing" "$tmp_dir/bin/services/$first_unselected_binary"

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

tauri_package_root="$tmp_dir/macos-bundle"
tauri_app="$tauri_package_root/A4N.app"
mkdir -p "$tauri_app/Contents/MacOS" "$tauri_app/Contents/Resources"
printf 'signed-host\n' > "$tauri_app/Contents/MacOS/a4n-tauri"
while IFS= read -r tool; do
    printf '%s-signed\n' "$tool" > "$tauri_app/Contents/MacOS/$tool"
done <<<"$tool_binaries"
printf 'signed-runtime\n' > "$tauri_app/Contents/Resources/libonnxruntime.dylib"
while IFS= read -r binary; do
    printf '%s-signed\n' "$binary" > "$tauri_app/Contents/MacOS/$binary"
done <<<"$sidecar_binaries"
tauri_metadata="$tmp_dir/macos-metadata"
mkdir -p "$tauri_metadata"
ruby scripts/release_manifest.rb generate \
    --package-root "$tauri_package_root" \
    --target aarch64-apple-darwin \
    --package a4n-tauri \
    --tauri-app "$tauri_app" \
    --output "$tauri_metadata/release-manifest-tauri.json"
ruby scripts/release_manifest.rb verify \
    --package-root "$tauri_package_root" \
    --manifest "$tauri_metadata/release-manifest-tauri.json"
printf 'tampered\n' >> "$tauri_app/Contents/MacOS/$first_sidecar_binary"
if ruby scripts/release_manifest.rb verify --package-root "$tauri_package_root" --manifest "$tauri_metadata/release-manifest-tauri.json" >/dev/null 2>&1; then
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

cp "$tmp_dir/release-manifest.json" "$tmp_dir/release-manifest.valid.json"
ruby -rjson -e '
  path = ARGV.fetch(0)
  data = JSON.parse(File.read(path))
  data["protocol_version"] += 1
  data.fetch("binaries").each { |entry| entry["protocol_version"] = data["protocol_version"] }
  File.write(path, JSON.pretty_generate(data) + "\n")
' "$tmp_dir/release-manifest.json"
if ruby scripts/release_manifest.rb verify --package-root "$tmp_dir" --manifest "$tmp_dir/release-manifest.json" >/dev/null 2>&1; then
    echo "manifest protocol mismatch unexpectedly passed verification" >&2
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
