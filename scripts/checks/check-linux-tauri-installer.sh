#!/usr/bin/env bash
# Verify the contents and metadata of an extracted Tauri deb installer.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

package="${1:-}"
[[ -n "$package" && -f "$package" ]] || {
    printf 'usage: %s <linux .deb installer>\n' "$0" >&2
    exit 2
}
require_command dpkg-deb

architecture="$(dpkg-deb --field "$package" Architecture)"
[[ "$architecture" == "amd64" ]] || {
    printf 'unexpected Linux installer architecture: %s\n' "$architecture" >&2
    exit 1
}

tmp_root="$(make_target_temp_dir linux-tauri-installer-check)"
trap 'rm -rf "$tmp_root"' EXIT
dpkg-deb --extract "$package" "$tmp_root"

find_file() {
    local name="$1"
    find "$tmp_root" -type f -name "$name" -print -quit
}

host="$(find_file a4n-tauri)"
[[ -n "$host" && -x "$host" ]] || {
    printf 'Linux installer is missing executable a4n-tauri\n' >&2
    exit 1
}

while IFS= read -r package_name; do
    [[ -n "$package_name" ]] || continue
    binary="$(find_file "$package_name")"
    [[ -n "$binary" && -x "$binary" ]] || {
        printf 'Linux installer is missing executable sidecar: %s\n' "$package_name" >&2
        exit 1
    }
done < <(sidecar_catalog_binaries)

while IFS= read -r binary_name; do
    [[ -n "$binary_name" ]] || continue
    binary="$(find_file "$binary_name")"
    [[ -n "$binary" && -x "$binary" ]] || {
        printf 'Linux installer is missing executable release tool: %s\n' "$binary_name" >&2
        exit 1
    }
done < <(release_tool_binaries)

[[ -n "$(find_file libonnxruntime.so.1)" ]] || {
    printf 'Linux installer is missing libonnxruntime.so.1\n' >&2
    exit 1
}
[[ -n "$(find_file liba4n_db.so)" ]] || {
    printf 'Linux installer is missing liba4n_db.so\n' >&2
    exit 1
}
find "$tmp_root" -type f -name 'libstd-*.so' -print -quit | rg -q . || {
    printf 'Linux installer is missing the Rust shared standard library\n' >&2
    exit 1
}
[[ -n "$(find_file _registry.yaml)" ]] || {
    printf 'Linux installer is missing prompts/_registry.yaml\n' >&2
    exit 1
}

printf 'Linux deb installer layout verified: %s\n' "$(basename "$package")"
