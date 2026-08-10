#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

CATALOG="crates/a4n-service-protocol/service-catalog.json"

extract_catalog_packages() {
    ruby -rjson -e '
      JSON.parse(File.read(ARGV.fetch(0))).each { |entry| puts entry.fetch("binary") }
    ' "$CATALOG"
}

validate_catalog_fields() {
    ruby -rjson -e '
      JSON.parse(File.read(ARGV.fetch(0))).each do |entry|
        puts [entry.fetch("name"), entry.fetch("addr_env"), entry.fetch("url_env"), entry.fetch("health_path"), entry.fetch("default_addr")].join("\t")
      end
    ' "$CATALOG" | while IFS=$'\t' read -r name addr_env url_env health_path default_addr; do
        for file in scripts/deploy.sh; do
            rg -q --fixed-strings "$addr_env" "$file" || {
                printf 'catalog field %s (%s) missing from %s\n' "$addr_env" "$name" "$file" >&2
                return 1
            }
        done
        for file in scripts/deploy.sh; do
            rg -q --fixed-strings "$url_env" "$file" || {
                printf 'catalog field %s (%s) missing from %s\n' "$url_env" "$name" "$file" >&2
                return 1
            }
        done
        [[ "$health_path" == "/healthz" ]] || {
            printf 'unsupported catalog health path for %s: %s\n' "$name" "$health_path" >&2
            return 1
        }
        [[ "$default_addr" =~ ^127\.0\.0\.1:[0-9]+$ ]] || {
            printf 'catalog default address must be loopback host:port for %s: %s\n' "$name" "$default_addr" >&2
            return 1
        }
    done
}

validate_run_services_catalog_consumer() {
    local listing
    rg -q --fixed-strings 'SERVICE_CATALOG_FILE="$PROJECT_ROOT/crates/a4n-service-protocol/service-catalog.json"' \
        scripts/dev/run-services.sh || {
        printf 'run-services.sh does not bind the sidecar catalog\n' >&2
        return 1
    }
    rg -q --fixed-strings 'catalog_field "$1" binary' scripts/dev/run-services.sh || {
        printf 'run-services.sh does not derive package names from the sidecar catalog\n' >&2
        return 1
    }
    listing="$(scripts/dev/run-services.sh --list)" || return 1
    while IFS=$'\t' read -r name binary; do
        [[ "$listing" == *"$name"* && "$listing" == *"$binary"* ]] || {
            printf 'run-services.sh --list is missing catalog entry %s (%s)\n' "$name" "$binary" >&2
            return 1
        }
    done < <(
        ruby -rjson -e '
          JSON.parse(File.read(ARGV.fetch(0))).each do |entry|
            puts [entry.fetch("name"), entry.fetch("binary")].join("\t")
          end
        ' "$CATALOG"
    )
}

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
    local expected_count="${#expected[@]}"
    local actual_count="${#actual[@]}"

    if [[ "$actual_count" -ne "$expected_count" ]]; then
        printf 'sidecar inventory mismatch in %s: expected %s entries, got %s\n' \
            "$label" "$expected_count" "$actual_count" >&2
        printf 'actual entries:\n' >&2
        printf '  %s\n' "${actual[@]}" >&2
        return 1
    fi

    local index
    for index in "${!expected[@]}"; do
        if [[ "${actual[$index]}" != "${expected[$index]}" ]]; then
            printf 'sidecar inventory mismatch in %s at index %s: expected %s, got %s\n' \
                "$label" "$index" "${expected[$index]}" "${actual[$index]}" >&2
            return 1
        fi
    done
}

check_portable_launcher_syntax() {
    awk '/cat > "\$PACKAGE_DIR\/start\.sh" << '\''SHEOF'\''/{capture=1; next} capture && /^SHEOF$/{exit} capture{print}' \
        scripts/deploy.sh | bash -n
}

require_ruby() {
    if ! command -v ruby >/dev/null 2>&1; then
        printf 'ruby is required to parse crates/a4n-tauri/tauri.sidecars.conf.json\n' >&2
        exit 1
    fi
}

require_ruby

[[ -f "$CATALOG" ]] || {
    printf 'missing sidecar catalog: %s\n' "$CATALOG" >&2
    exit 1
}
mapfile -t expected < <(extract_catalog_packages)
validate_catalog_fields
validate_run_services_catalog_consumer

mapfile -t deploy_packages < <(extract_bash_sidecar_packages "scripts/deploy.sh")
mapfile -t tauri_prepare_packages < <(extract_bash_sidecar_packages "scripts/dev/prepare-tauri-external-bin.sh")
mapfile -t all_tauri_external_bins < <(extract_tauri_external_bins "crates/a4n-tauri/tauri.sidecars.conf.json")
tauri_external_bins=("${all_tauri_external_bins[@]:0:${#expected[@]}}")
tauri_tools=("${all_tauri_external_bins[@]:${#expected[@]}}")

compare_inventory "scripts/deploy.sh SIDECAR_PACKAGES" "${deploy_packages[@]}"
compare_inventory "scripts/dev/prepare-tauri-external-bin.sh SIDECAR_PACKAGES" "${tauri_prepare_packages[@]}"
compare_inventory "crates/a4n-tauri/tauri.sidecars.conf.json bundle.externalBin" "${tauri_external_bins[@]}"
expected_tools=(a4n a4n-server a4n-migration)
[[ "${tauri_tools[*]}" == "${expected_tools[*]}" ]] || {
    printf 'Tauri tool inventory mismatch: expected [%s], got [%s]\n' \
        "${expected_tools[*]}" "${tauri_tools[*]}" >&2
    exit 1
}
check_portable_launcher_syntax

printf 'sidecar package inventory is consistent (%s entries)\n' "${#expected[@]}"
"$SCRIPT_DIR/check-release-manifest.sh"
