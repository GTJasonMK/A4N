#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

EXPECTED_PACKAGES=(
    "a4n-prompt-service"
    "a4n-llm-gateway"
    "a4n-embedding-service"
    "a4n-rag-service"
    "a4n-image-service"
    "a4n-video-service"
    "a4n-import-service"
    "a4n-export-service"
)

extract_bash_sidecar_packages() {
    local file="$1"
    awk '
        /^SIDECAR_PACKAGES=\(/ {
            in_packages = 1
            next
        }
        in_packages && /^\)/ {
            exit
        }
        in_packages {
            value = $0
            gsub(/^[[:space:]]*"/, "", value)
            gsub(/"[[:space:]]*$/, "", value)
            if (value != "") {
                print value
            }
        }
    ' "$file"
}

extract_tauri_external_bins() {
    ruby -rjson -e '
      data = JSON.parse(File.read(ARGV.fetch(0)))
      data.fetch("bundle").fetch("externalBin").each do |entry|
        puts File.basename(entry)
      end
    ' "$1"
}

compare_inventory() {
    local label="$1"
    shift
    local actual=("$@")
    local expected_count="${#EXPECTED_PACKAGES[@]}"
    local actual_count="${#actual[@]}"

    if [[ "$actual_count" -ne "$expected_count" ]]; then
        printf 'sidecar inventory mismatch in %s: expected %s entries, got %s\n' \
            "$label" "$expected_count" "$actual_count" >&2
        printf 'actual entries:\n' >&2
        printf '  %s\n' "${actual[@]}" >&2
        return 1
    fi

    local index
    for index in "${!EXPECTED_PACKAGES[@]}"; do
        if [[ "${actual[$index]}" != "${EXPECTED_PACKAGES[$index]}" ]]; then
            printf 'sidecar inventory mismatch in %s at index %s: expected %s, got %s\n' \
                "$label" "$index" "${EXPECTED_PACKAGES[$index]}" "${actual[$index]}" >&2
            return 1
        fi
    done
}

require_ruby() {
    if ! command -v ruby >/dev/null 2>&1; then
        printf 'ruby is required to parse crates/a4n-tauri/tauri.sidecars.conf.json\n' >&2
        exit 1
    fi
}

require_ruby

mapfile -t deploy_packages < <(extract_bash_sidecar_packages "scripts/deploy.sh")
mapfile -t tauri_prepare_packages < <(extract_bash_sidecar_packages "scripts/dev/prepare-tauri-external-bin.sh")
mapfile -t tauri_external_bins < <(extract_tauri_external_bins "crates/a4n-tauri/tauri.sidecars.conf.json")

compare_inventory "scripts/deploy.sh SIDECAR_PACKAGES" "${deploy_packages[@]}"
compare_inventory "scripts/dev/prepare-tauri-external-bin.sh SIDECAR_PACKAGES" "${tauri_prepare_packages[@]}"
compare_inventory "crates/a4n-tauri/tauri.sidecars.conf.json bundle.externalBin" "${tauri_external_bins[@]}"

printf 'sidecar package inventory is consistent (%s entries)\n' "${#EXPECTED_PACKAGES[@]}"
