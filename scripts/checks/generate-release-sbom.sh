#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_DIR="${1:-}"
RELEASE_ID="${2:-}"

if [[ -z "$OUTPUT_DIR" || ! "$RELEASE_ID" =~ ^[A-Za-z0-9._-]+$ ]]; then
    echo "usage: $0 OUTPUT_DIR RELEASE_ID" >&2
    exit 2
fi

cd "$ROOT"
mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd)"
SBOM_DIR="$OUTPUT_DIR/sbom"
CATALOG="$ROOT/crates/a4n-service-protocol/service-catalog.json"
METADATA="$OUTPUT_DIR/cargo-metadata.json"

mkdir -p "$SBOM_DIR"
if find "$SBOM_DIR" -mindepth 1 -print -quit | rg -q .; then
    echo "SBOM output directory is not empty: $SBOM_DIR" >&2
    exit 1
fi

cargo metadata --no-deps --locked --format-version 1 > "$METADATA"
mapfile -t PACKAGES < <(
    ruby -rjson -e '
      catalog = JSON.parse(File.read(ARGV.fetch(0)))
      metadata = JSON.parse(File.read(ARGV.fetch(1)))
      binaries = ["a4n-tauri", *catalog.map { |service| service.fetch("binary") }]
      packages = binaries.map do |binary|
        matches = metadata.fetch("packages").select do |package|
          package.fetch("targets").any? do |target|
            target.fetch("name") == binary && target.fetch("kind").include?("bin")
          end
        end
        abort "expected one workspace package for binary #{binary}" unless matches.size == 1
        matches.first.fetch("name")
      end.uniq
      abort "expected host and eight sidecar packages" unless packages.size == 9
      puts packages
    ' "$CATALOG" "$METADATA"
)

for package in "${PACKAGES[@]}"; do
    manifest="$(ruby -rjson -e '
      metadata = JSON.parse(File.read(ARGV.fetch(0)))
      matches = metadata.fetch("packages").select { |item| item.fetch("name") == ARGV.fetch(1) }
      abort "expected one workspace package named #{ARGV.fetch(1)}" unless matches.size == 1
      print matches.first.fetch("manifest_path")
    ' "$METADATA" "$package")"
    filename="A4N-${RELEASE_ID}-${package}.cdx.json"
    generated="$(dirname "$manifest")/$filename"
    [[ ! -e "$generated" ]] || {
        echo "stale generated SBOM exists: $generated" >&2
        exit 1
    }
    cargo cyclonedx \
        --manifest-path "$manifest" \
        --format json \
        --all-features \
        --target all \
        --override-filename "$filename"
    [[ -s "$generated" ]] || {
        echo "cargo-cyclonedx did not create: $generated" >&2
        exit 1
    }
    mv "$generated" "$SBOM_DIR/$filename"
done

[[ "$(find "$SBOM_DIR" -maxdepth 1 -type f -name '*.cdx.json' | wc -l | tr -d ' ')" -eq 9 ]] || {
    echo "release SBOM set must contain host and eight sidecars" >&2
    exit 1
}

archive="$OUTPUT_DIR/A4N-${RELEASE_ID}-cyclonedx.tar.gz"
tar --sort=name --mtime='UTC 1970-01-01' --owner=0 --group=0 --numeric-owner \
    -czf "$archive" -C "$SBOM_DIR" .
(
    cd "$OUTPUT_DIR"
    sha256sum "$(basename "$archive")" > "$(basename "$archive").sha256"
)

printf 'release SBOM: packages=%s archive=%s\n' "${#PACKAGES[@]}" "$archive"
