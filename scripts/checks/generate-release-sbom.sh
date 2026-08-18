#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
OUTPUT_DIR="${1:-}"
RELEASE_ID="${2:-}"

if [[ -z "$OUTPUT_DIR" || ! "$RELEASE_ID" =~ ^[A-Za-z0-9._-]+$ ]]; then
    echo "usage: $0 OUTPUT_DIR RELEASE_ID" >&2
    exit 2
fi

cd "$PROJECT_ROOT"
mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd)"
SBOM_DIR="$OUTPUT_DIR/sbom"
METADATA="$OUTPUT_DIR/cargo-metadata.json"

mkdir -p "$SBOM_DIR"
if find "$SBOM_DIR" -mindepth 1 -print -quit | rg -q .; then
    echo "SBOM output directory is not empty: $SBOM_DIR" >&2
    exit 1
fi

sidecar_binary_lines="$(sidecar_catalog_binaries)"
mapfile -t SIDECAR_BINARIES <<<"$sidecar_binary_lines"
tool_binary_lines="$(release_tool_binaries)"
mapfile -t TOOL_BINARIES <<<"$tool_binary_lines"
RELEASE_BINARIES=("a4n-tauri" "${SIDECAR_BINARIES[@]}" "${TOOL_BINARIES[@]}")

cargo metadata --no-deps --locked --format-version 1 > "$METADATA"
package_lines="$(ruby -rjson -e '
  metadata = JSON.parse(File.read(ARGV.fetch(0)))
  binaries = ARGV.drop(1)
  packages = binaries.map do |binary|
    matches = metadata.fetch("packages").select do |package|
      package.fetch("targets").any? do |target|
        target.fetch("name") == binary && target.fetch("kind").include?("bin")
      end
    end
    abort "expected one workspace package for binary #{binary}" unless matches.size == 1
    matches.first.fetch("name")
  end.uniq
  puts packages
' "$METADATA" "${RELEASE_BINARIES[@]}")"
mapfile -t PACKAGES <<<"$package_lines"

generated_lines="$(ruby -rjson -e '
  metadata = JSON.parse(File.read(ARGV.fetch(0)))
  members = metadata.fetch("workspace_members").to_h { |id| [id, true] }
  metadata.fetch("packages").each do |package|
    next unless members.key?(package.fetch("id"))
    puts File.join(
      File.dirname(package.fetch("manifest_path")),
      "#{package.fetch("name")}.cdx.json"
    )
  end
' "$METADATA")"
mapfile -t GENERATED <<<"$generated_lines"
for generated in "${GENERATED[@]}"; do
    [[ ! -e "$generated" ]] || {
        echo "stale generated SBOM exists: $generated" >&2
        exit 1
    }
done
cleanup_generated() {
    rm -f -- "${GENERATED[@]}"
}
trap cleanup_generated EXIT

cargo cyclonedx \
    --manifest-path "$PROJECT_ROOT/Cargo.toml" \
    --format json \
    --all-features \
    --target all

for package in "${PACKAGES[@]}"; do
    manifest="$(ruby -rjson -e '
      metadata = JSON.parse(File.read(ARGV.fetch(0)))
      matches = metadata.fetch("packages").select { |item| item.fetch("name") == ARGV.fetch(1) }
      abort "expected one workspace package named #{ARGV.fetch(1)}" unless matches.size == 1
      print matches.first.fetch("manifest_path")
    ' "$METADATA" "$package")"
    filename="A4N-${RELEASE_ID}-${package}.cdx.json"
    generated="$(dirname "$manifest")/${package}.cdx.json"
    [[ -s "$generated" ]] || {
        echo "cargo-cyclonedx did not create: $generated" >&2
        exit 1
    }
    mv "$generated" "$SBOM_DIR/$filename"
done

archive="$OUTPUT_DIR/A4N-${RELEASE_ID}-cyclonedx.tar.gz"
tar --sort=name --mtime='UTC 1970-01-01' --owner=0 --group=0 --numeric-owner \
    -czf "$archive" -C "$SBOM_DIR" .
(
    cd "$OUTPUT_DIR"
    sha256sum "$(basename "$archive")" > "$(basename "$archive").sha256"
)

printf 'release SBOM: packages=%s archive=%s\n' "${#PACKAGES[@]}" "$archive"
