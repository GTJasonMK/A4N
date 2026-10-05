#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MANIFEST_TOOL="$PROJECT_ROOT/scripts/release_manifest.rb"
PACKAGE_NAME="a4n-tauri"
TARGET_PLATFORM="linux-x86_64"
SELECTED_SERVICE="export"

fail() {
    printf 'release installation check failed: %s\n' "$*" >&2
    exit 1
}

usage() {
    printf 'usage: %s [LINUX_RELEASE_ARCHIVE]\n' "$0"
    printf 'runs the fixture check; with an archive, also checks real package install/reinstall behavior\n'
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
    usage
    exit 0
fi
[[ "$#" -le 1 ]] || {
    usage >&2
    exit 2
}
REAL_ARCHIVE="${1:-}"

require_command() {
    command -v "$1" >/dev/null 2>&1 || fail "missing command: $1"
}

require_command ruby
require_command mktemp
require_command tar
require_command sha256sum
require_command find
require_command cp
require_command wc
require_command timeout
[[ -r "$MANIFEST_TOOL" ]] || fail "missing release manifest tool: $MANIFEST_TOOL"
[[ "$(uname -s)" == "Linux" ]] || fail "Linux-only installation check"
if [[ -n "$REAL_ARCHIVE" && ! -f "$REAL_ARCHIVE" ]]; then
    fail "release archive does not exist: $REAL_ARCHIVE"
fi

temp_parent="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}/tmp"
mkdir -p "$temp_parent"
[[ -d "$temp_parent" ]] || fail "temporary parent does not exist: $temp_parent"
temp_parent="$(cd "$temp_parent" && pwd -P)"
tmp_dir="$(mktemp -d "$temp_parent/a4n-release-install.XXXXXX")"
case "$tmp_dir" in
    "$temp_parent"/a4n-release-install.*) ;;
    *) fail "mktemp returned an unsafe fixture path: $tmp_dir" ;;
esac

cleanup() {
    local status=$?
    trap - EXIT
    if [[ -n "${tmp_dir:-}" && "$tmp_dir" != / && "$tmp_dir" == "$temp_parent"/a4n-release-install.* ]]; then
        rm -rf -- "$tmp_dir"
    fi
    exit "$status"
}
trap cleanup EXIT

catalog_rows="$(ruby "$MANIFEST_TOOL" service-rows)" || fail "unable to load service catalog"
selected_binary=""
unselected_service_names=()
while IFS=$'\t' read -r service binary _rest; do
    [[ -n "$service" ]] || continue
    if [[ "$service" == "$SELECTED_SERVICE" ]]; then
        selected_binary="$binary"
    else
        unselected_service_names+=("$service")
    fi
done <<<"$catalog_rows"
[[ -n "$selected_binary" ]] || fail "service catalog has no selected service: $SELECTED_SERVICE"
[[ "${#unselected_service_names[@]}" -gt 0 ]] || fail "service catalog has no unselected service"
[[ "$selected_binary" == "a4n-component-host" ]] || fail "service catalog does not use the shared component host"
unselected_service="${unselected_service_names[0]}"

tool_rows="$(ruby "$MANIFEST_TOOL" tool-rows)" || fail "unable to load release tool catalog"
tool_binaries=()
while IFS=$'\t' read -r _tool_package tool_binary; do
    [[ -n "$tool_binary" ]] && tool_binaries+=("$tool_binary")
done <<<"$tool_rows"
[[ "${#tool_binaries[@]}" -gt 0 ]] || fail "release tool catalog is empty"

remove_fixture_dir() {
    local path="$1"
    case "$path" in
        "$tmp_dir"/*) ;;
        *) fail "refusing to remove outside fixture: $path" ;;
    esac
    case "/$path/" in
        *"/../"*|*"/./"*) fail "refusing an unsafe fixture path: $path" ;;
    esac
    case "$path" in
        *"//"*) fail "refusing an unsafe fixture path: $path" ;;
    esac
    [[ "$path" != "$tmp_dir" ]] || fail "refusing to remove fixture root"
    rm -rf -- "$path"
}

verify_archive_paths() {
    local archive="$1" entry members

    members="$(tar -tzf "$archive")" || {
        printf 'unable to list release archive: %s\n' "$archive" >&2
        return 1
    }
    while IFS= read -r entry; do
        [[ -n "$entry" ]] || continue
        [[ "$entry" != /* ]] || {
            printf 'release archive contains an absolute path: %s\n' "$entry" >&2
            return 1
        }
        [[ "$entry" != *\\* ]] || {
            printf 'release archive contains a platform-ambiguous path: %s\n' "$entry" >&2
            return 1
        }
        case "/$entry/" in
            *"/../"*|*"/./"*)
                printf 'release archive contains an unsafe path: %s\n' "$entry" >&2
                return 1
                ;;
        esac
        case "$entry" in
            *"//"*)
                printf 'release archive contains an unsafe path: %s\n' "$entry" >&2
                return 1
                ;;
        esac
    done <<<"$members"
}

write_fixture_package() {
    local package_root="$1"
    local fixture_id="$2"
    local tool

    mkdir -p "$package_root/bin/services" "$package_root/data/assets" "$package_root/data/checkpoints"
    {
        printf '%s\n' '#!/usr/bin/env bash'
        printf '%s\n' 'set -euo pipefail'
        printf '%s\n' '[[ -f "$PWD/bin/'"$selected_binary"'" ]]'
        printf '%s\n' '[[ ! -e "$PWD/bin/services/'"$selected_binary"'" ]]'
        printf '%s\n' '[[ -n "${A4N_FIXTURE_HOST_MARKER:-}" ]]'
        printf '%s\n' 'printf "%s\n" "${A4N_FIXTURE_RELEASE_ID:-unknown}" > "$A4N_FIXTURE_HOST_MARKER"'
    } >"$package_root/bin/$PACKAGE_NAME"
    chmod +x "$package_root/bin/$PACKAGE_NAME"

    for tool in "${tool_binaries[@]}"; do
        printf 'fixture-tool:%s:%s\n' "$tool" "$fixture_id" >"$package_root/bin/$tool"
        chmod +x "$package_root/bin/$tool"
    done

    printf 'fixture-runtime:%s\n' "$fixture_id" >"$package_root/bin/services/libonnxruntime.so.1"
    printf 'fixture-default-config:%s\n' "$fixture_id" >"$package_root/data/config.toml"

    ruby "$MANIFEST_TOOL" generate \
        --package-root "$package_root" \
        --target "$TARGET_PLATFORM" \
        --package "$PACKAGE_NAME" \
        --services "$SELECTED_SERVICE" \
        --output "$package_root/release-manifest.json"

    write_fixture_start_script "$package_root"
}

write_fixture_start_script() {
    local package_root="$1"
    local checksum_entries
    checksum_entries="$(wc -l <"$package_root/release-manifest.sha256")"
    [[ "$checksum_entries" =~ ^[1-9][0-9]*$ ]] || fail "invalid fixture checksum count"

    {
        printf '%s\n' '#!/usr/bin/env bash' "EXPECTED_RELEASE_CHECKSUM_ENTRIES=$checksum_entries"
        cat <<'START_EOF'
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

verify_release_manifest() {
    local checksum_file="$SCRIPT_DIR/release-manifest.sha256"
    local line expected relative count=0
    declare -A seen=()

    [[ -r "$SCRIPT_DIR/release-manifest.json" ]] || {
        printf 'missing release-manifest.json\n' >&2
        return 1
    }
    [[ -r "$checksum_file" ]] || {
        printf 'missing release-manifest.sha256\n' >&2
        return 1
    }
    while IFS= read -r line || [[ -n "$line" ]]; do
        [[ "$line" =~ ^([[:xdigit:]]{64})[[:space:]][[:space:]]([^[:space:]]+)$ ]] || {
            printf 'invalid release manifest checksum line\n' >&2
            return 1
        }
        expected="${BASH_REMATCH[1],,}"
        relative="${BASH_REMATCH[2]}"
        case "/$relative/" in
            *"/../"*|//*|*\\*)
                printf 'unsafe release manifest path: %s\n' "$relative" >&2
                return 1
                ;;
        esac
        [[ -z "${seen[$relative]+x}" ]] || {
            printf 'duplicate release manifest checksum path: %s\n' "$relative" >&2
            return 1
        }
        seen["$relative"]=1
        [[ -f "$SCRIPT_DIR/$relative" ]] || {
            printf 'missing release manifest file: %s\n' "$relative" >&2
            return 1
        }
        printf '%s  %s\n' "$expected" "$relative" | sha256sum -c - >/dev/null || return 1
        count=$((count + 1))
    done <"$checksum_file"
    [[ "$count" -eq "$EXPECTED_RELEASE_CHECKSUM_ENTRIES" ]] || {
        printf 'release manifest checksum count mismatch\n' >&2
        return 1
    }
    [[ -n "${seen[release-manifest.json]+x}" ]] || {
        printf 'release manifest checksum omits release-manifest.json\n' >&2
        return 1
    }
}

verify_release_manifest
exec bin/a4n-tauri
START_EOF
    } >"$package_root/start.sh"
    chmod +x "$package_root/start.sh"
}

verify_selected_layout() {
    local package_root="$1"
    local manifest="$package_root/release-manifest.json"

    ruby "$MANIFEST_TOOL" verify \
        --package-root "$package_root" \
        --manifest "$manifest" || return 1
    (cd "$package_root" && sha256sum -c release-manifest.sha256 >/dev/null) || {
        printf 'fixture checksum verification failed: %s\n' "$package_root" >&2
        return 1
    }
    ruby -rjson -e '
      path, selected = ARGV
      data = JSON.parse(File.read(path))
      services = data.fetch("binaries").map { |entry| entry.fetch("service") }.sort
      expected = ["host", selected].sort
      abort "manifest selected services mismatch" unless services == expected && data.fetch("services") == [selected]
    ' "$manifest" "$SELECTED_SERVICE" || return 1
    [[ -x "$package_root/start.sh" ]] || {
        printf 'fixture is missing executable start.sh\n' >&2
        return 1
    }
    [[ -x "$package_root/bin/$PACKAGE_NAME" ]] || {
        printf 'fixture is missing executable host\n' >&2
        return 1
    }
    [[ -x "$package_root/bin/$selected_binary" ]] || {
        printf 'fixture is missing shared component host: %s\n' "$selected_binary" >&2
        return 1
    }
    local unselected_service
    for unselected_service in "${unselected_service_names[@]}"; do
        [[ ! -e "$package_root/bin/services/$unselected_service" ]] || {
            printf 'legacy per-service binary is present in selected install: %s\n' "$unselected_service" >&2
            return 1
        }
    done
}

verify_real_layout() {
    local package_root="$1"

    ruby "$MANIFEST_TOOL" verify \
        --package-root "$package_root" \
        --manifest "$package_root/release-manifest.json" || return 1
    [[ -x "$package_root/start.sh" ]] || {
        printf 'release package is missing executable start.sh: %s\n' "$package_root" >&2
        return 1
    }
    [[ -x "$package_root/bin/$PACKAGE_NAME" ]] || {
        printf 'release package is missing executable host: %s\n' "$package_root" >&2
        return 1
    }
}

archive_package() {
    local package_root="$1"
    local archive="$2"
    tar -czf "$archive" -C "$(dirname "$package_root")" "$(basename "$package_root")"
}

clone_package_for_candidate() {
    local source="$1" target="$2"

    # Candidate fixtures share unchanged release files; only edited metadata is copied.
    cp -al -- "$source" "$target"
}

make_manifest_mutable() {
    local manifest="$1" mutable
    mutable="$manifest.mutable"

    cp -- "$manifest" "$mutable"
    mv -- "$mutable" "$manifest"
}

clear_installation_code() {
    local install_root="$1" child

    [[ -d "$install_root" ]] || return 0
    while IFS= read -r -d '' child; do
        rm -rf -- "$child"
    done < <(find "$install_root" -mindepth 1 -maxdepth 1 ! -name data -print0)
}

uninstall_installation() {
    local install_root="$1"

    case "$install_root" in
        "$tmp_dir"/*) ;;
        *) printf 'refusing to uninstall outside fixture: %s\n' "$install_root" >&2; return 1 ;;
    esac
    [[ "$install_root" != "$tmp_dir" ]] || {
        printf 'refusing to uninstall fixture root\n' >&2
        return 1
    }
    [[ -d "$install_root" ]] || return 0
    [[ ! -L "$install_root/data" ]] || {
        printf 'persistent data directory must not be a symlink\n' >&2
        return 1
    }
    clear_installation_code "$install_root"
    [[ ! -e "$install_root/start.sh" ]] || {
        printf 'uninstall left release code behind\n' >&2
        return 1
    }
}

PERSISTENT_FILES=(
    data/project.db
    data/assets/cover.bin
    data/checkpoints/workflow.json
)

write_persistent_state() {
    local package_root="$1" label="$2"

    mkdir -p "$package_root/data/assets" "$package_root/data/checkpoints"
    printf 'database-state:%s\n' "$label" >"$package_root/data/project.db"
    printf 'asset-state:%s\n' "$label" >"$package_root/data/assets/cover.bin"
    printf '{"checkpoint":"%s"}\n' "$label" >"$package_root/data/checkpoints/workflow.json"
}

snapshot_persistent_state() {
    local package_root="$1" snapshot="$2"

    (cd "$package_root" && sha256sum "${PERSISTENT_FILES[@]}") >"$snapshot"
}

assert_persistent_state() {
    local package_root="$1" snapshot="$2"

    (cd "$package_root" && sha256sum -c "$snapshot" >/dev/null)
}

expect_start_rejection() {
    local package_root="$1" label="$2" log status
    log="$tmp_dir/start-$label.log"

    if timeout 10s "$package_root/start.sh" >"$log" 2>&1; then
        printf 'start.sh accepted an invalid package: %s\n' "$label" >&2
        return 1
    else
        status=$?
        [[ "$status" -ne 124 ]] || {
            printf 'start.sh did not reject invalid package promptly: %s\n' "$label" >&2
            return 1
        }
    fi
}

INSTALLED_PACKAGE=""
install_archive() {
    local archive="$1"
    local install_root="$2"
    local layout="${3:-selected}"
    local staging candidate persistent_backup=""
    local roots=()
    local top_entries=()

    case "$install_root" in
        "$tmp_dir"/*) ;;
        *) printf 'refusing to install outside fixture: %s\n' "$install_root" >&2; return 1 ;;
    esac
    [[ "$install_root" != "$tmp_dir" ]] || {
        printf 'refusing to install into fixture root\n' >&2
        return 1
    }
    [[ "$layout" == selected || "$layout" == real ]] || {
        printf 'unknown installation layout: %s\n' "$layout" >&2
        return 1
    }
    verify_archive_paths "$archive" || return 1
    [[ ! -e "$install_root" || -d "$install_root" ]] || {
        printf 'installation root is not a directory: %s\n' "$install_root" >&2
        return 1
    }
    [[ ! -L "$install_root/data" ]] || {
        printf 'persistent data directory must not be a symlink\n' >&2
        return 1
    }

    staging="$(mktemp -d "$tmp_dir/staging.XXXXXX")"
    if ! tar -xzf "$archive" -C "$staging"; then
        remove_fixture_dir "$staging"
        return 1
    fi
    mapfile -t top_entries < <(find "$staging" -mindepth 1 -maxdepth 1 -print)
    mapfile -t roots < <(find "$staging" -mindepth 1 -maxdepth 1 -type d -print)
    [[ "${#top_entries[@]}" -eq 1 && "${#roots[@]}" -eq 1 ]] || {
        printf 'archive must contain exactly one package root\n' >&2
        remove_fixture_dir "$staging"
        return 1
    }
    candidate="${roots[0]}"
    if [[ "$layout" == selected ]]; then
        if ! verify_selected_layout "$candidate"; then
            remove_fixture_dir "$staging"
            return 1
        fi
    else
        if ! verify_real_layout "$candidate"; then
            remove_fixture_dir "$staging"
            return 1
        fi
    fi

    mkdir -p "$install_root"
    if [[ -d "$install_root/data" ]]; then
        persistent_backup="$(mktemp -d "$tmp_dir/persistent.XXXXXX")/data"
        mv "$install_root/data" "$persistent_backup"
        [[ ! -d "$candidate/data" ]] || rm -rf -- "$candidate/data"
    fi
    clear_installation_code "$install_root"
    cp -a "$candidate"/. "$install_root"/
    if [[ -n "$persistent_backup" ]]; then
        rm -rf -- "$install_root/data"
        mv "$persistent_backup" "$install_root/data"
    fi
    INSTALLED_PACKAGE="$install_root"
    remove_fixture_dir "$staging"
    printf 'installed package: %s\n' "$INSTALLED_PACKAGE"
}

run_fixture_host() {
    local package_root="$1"
    local run_id="$2"
    local marker="$tmp_dir/markers/$run_id"

    mkdir -p "$tmp_dir/markers"
    rm -f -- "$marker"
    A4N_FIXTURE_HOST_MARKER="$marker" \
    A4N_FIXTURE_RELEASE_ID="$run_id" \
        "$package_root/start.sh"
    [[ -f "$marker" && "$(<"$marker")" == "$run_id" ]] || {
        printf 'fixture host did not start: %s\n' "$run_id" >&2
        return 1
    }
}

check_real_archive() {
    local archive="$1"
    local install_root="$tmp_dir/real-installed"
    local package_root state_snapshot selected_path selected_backup
    local current_install version_source_parent version_source version_archive
    local protocol_source_parent protocol_source protocol_archive
    local traversal_source traversal_archive

    printf 'checking real Linux release archive: %s\n' "$archive"
    verify_archive_paths "$archive" || fail "real release archive contains unsafe paths"
    install_archive "$archive" "$install_root" real || fail "real release archive failed staged verification"
    package_root="$INSTALLED_PACKAGE"

    write_persistent_state "$package_root" real
    state_snapshot="$tmp_dir/real-state.sha256"
    snapshot_persistent_state "$package_root" "$state_snapshot"

    printf 'checking real archive uninstall and reinstall state preservation\n'
    uninstall_installation "$package_root" || fail "real archive uninstall failed"
    assert_persistent_state "$package_root" "$state_snapshot" || fail "real archive uninstall lost persistent state"
    install_archive "$archive" "$install_root" real || fail "real archive reinstall failed"
    package_root="$INSTALLED_PACKAGE"
    assert_persistent_state "$package_root" "$state_snapshot" || fail "real archive reinstall lost persistent state"

    selected_path="$package_root/bin/$selected_binary"
    selected_backup="$selected_path.original"
    printf 'checking real package startup rejection for a missing sidecar and checksum mismatch\n'
    [[ -f "$selected_path" ]] || fail "real release archive has no selected sidecar: $selected_binary"
    mv -- "$selected_path" "$selected_path.missing"
    expect_start_rejection "$package_root" real-missing-selected || fail "real package accepted a missing selected sidecar"
    mv -- "$selected_path.missing" "$selected_path"
    cp -p -- "$selected_path" "$selected_backup"
    printf 'tampered\n' >>"$selected_path"
    expect_start_rejection "$package_root" real-checksum-mismatch || fail "real package accepted a checksum mismatch"
    mv -- "$selected_backup" "$selected_path"
    assert_persistent_state "$package_root" "$state_snapshot" || fail "real package preflight changed persistent state"

    current_install="$INSTALLED_PACKAGE"
    version_source_parent="$tmp_dir/real-version-source"
    version_source="$version_source_parent/a4n-tauri-0.1.0-linux-x86_64"
    mkdir -p "$version_source_parent"
    clone_package_for_candidate "$package_root" "$version_source"
    make_manifest_mutable "$version_source/release-manifest.json"
    ruby -rjson -e '
      path = ARGV.fetch(0)
      data = JSON.parse(File.read(path))
      data["version"] = "incompatible-version"
      File.write(path, JSON.pretty_generate(data) + "\n")
    ' "$version_source/release-manifest.json"
    version_archive="$tmp_dir/real-version-mismatch.tar.gz"
    archive_package "$version_source" "$version_archive"
    remove_fixture_dir "$version_source_parent"
    printf 'checking rejection of a real package version mismatch\n'
    if install_archive "$version_archive" "$install_root" real; then
        fail "real package version mismatch was installed"
    fi
    remove_fixture_dir "$version_archive"
    [[ "$INSTALLED_PACKAGE" == "$current_install" ]] || fail "version mismatch replaced the real installation"
    assert_persistent_state "$package_root" "$state_snapshot" || fail "version mismatch lost persistent state"

    protocol_source_parent="$tmp_dir/real-protocol-source"
    protocol_source="$protocol_source_parent/a4n-tauri-0.1.0-linux-x86_64"
    mkdir -p "$protocol_source_parent"
    clone_package_for_candidate "$package_root" "$protocol_source"
    make_manifest_mutable "$protocol_source/release-manifest.json"
    ruby -rjson -e '
      path = ARGV.fetch(0)
      data = JSON.parse(File.read(path))
      data["protocol_version"] += 1
      data.fetch("binaries").each { |entry| entry["protocol_version"] = data["protocol_version"] }
      File.write(path, JSON.pretty_generate(data) + "\n")
    ' "$protocol_source/release-manifest.json"
    protocol_archive="$tmp_dir/real-protocol-mismatch.tar.gz"
    archive_package "$protocol_source" "$protocol_archive"
    remove_fixture_dir "$protocol_source_parent"
    printf 'checking rejection of a real package protocol mismatch\n'
    if install_archive "$protocol_archive" "$install_root" real; then
        fail "real package protocol mismatch was installed"
    fi
    remove_fixture_dir "$protocol_archive"
    [[ "$INSTALLED_PACKAGE" == "$current_install" ]] || fail "protocol mismatch replaced the real installation"
    assert_persistent_state "$package_root" "$state_snapshot" || fail "protocol mismatch lost persistent state"

    traversal_source="$tmp_dir/archive-traversal-source"
    mkdir -p "$traversal_source"
    printf 'archive traversal payload\n' >"$traversal_source/safe"
    traversal_archive="$tmp_dir/real-path-traversal.tar.gz"
    tar --transform='s#^safe$#../escape#' -czf "$traversal_archive" -C "$traversal_source" safe
    printf 'checking rejection of a real archive path traversal\n'
    if install_archive "$traversal_archive" "$install_root" real; then
        fail "real archive path traversal was installed"
    fi
    [[ "$INSTALLED_PACKAGE" == "$current_install" ]] || fail "path traversal replaced the real installation"
    assert_persistent_state "$package_root" "$state_snapshot" || fail "path traversal lost persistent state"
    verify_real_layout "$package_root" || fail "valid real installation failed after rejected candidates"
    printf 'real Linux release archive check passed: staged install, startup preflight, checksum, uninstall/reinstall state preservation, version/protocol mismatch, path traversal\n'
}

source_parent="$tmp_dir/source"
package_root="$source_parent/a4n-tauri-fixture"
mkdir -p "$source_parent"
write_fixture_package "$package_root" v1
verify_selected_layout "$package_root"

archive_v1="$tmp_dir/a4n-tauri-v1.tar.gz"
archive_package "$package_root" "$archive_v1"
install_root="$tmp_dir/installed"
install_archive "$archive_v1" "$install_root" selected
run_fixture_host "$INSTALLED_PACKAGE" initial || fail "installed fixture did not start"
write_persistent_state "$INSTALLED_PACKAGE" fixture
state_snapshot="$tmp_dir/fixture-state.sha256"
snapshot_persistent_state "$INSTALLED_PACKAGE" "$state_snapshot"

printf 'checking fixture startup rejection for a missing selected sidecar\n'
selected_path="$INSTALLED_PACKAGE/bin/$selected_binary"
mv -- "$selected_path" "$selected_path.missing"
if run_fixture_host "$INSTALLED_PACKAGE" missing-selected; then
    fail "startup accepted a package with the selected sidecar removed"
fi
[[ ! -e "$tmp_dir/markers/missing-selected" ]] || fail "host ran after selected sidecar removal"
mv -- "$selected_path.missing" "$selected_path"
assert_persistent_state "$INSTALLED_PACKAGE" "$state_snapshot" || fail "missing sidecar check changed persistent state"
selected_backup="$selected_path.original"
cp -p -- "$selected_path" "$selected_backup"
printf 'tampered\n' >>"$selected_path"
if run_fixture_host "$INSTALLED_PACKAGE" checksum-mismatch; then
    fail "startup accepted a checksum mismatch"
fi
mv -- "$selected_backup" "$selected_path"
assert_persistent_state "$INSTALLED_PACKAGE" "$state_snapshot" || fail "checksum check changed persistent state"

printf 'checking fixture uninstall and reinstall with data/assets/checkpoints preserved\n'
uninstall_installation "$install_root" || fail "fixture uninstall failed"
assert_persistent_state "$install_root" "$state_snapshot" || fail "fixture uninstall lost persistent state"
install_archive "$archive_v1" "$install_root" selected
assert_persistent_state "$INSTALLED_PACKAGE" "$state_snapshot" || fail "fixture reinstall lost persistent state"
run_fixture_host "$INSTALLED_PACKAGE" reinstalled || fail "reinstalled fixture did not start"

release_version="$(ruby -rjson -e 'puts JSON.parse(File.read(ARGV.fetch(0))).fetch("version")' "$package_root/release-manifest.json")"
upgrade_source_parent="$tmp_dir/upgrade-source"
upgrade_source="$upgrade_source_parent/a4n-tauri-fixture"
mkdir -p "$upgrade_source_parent"
cp -a "$package_root" "$upgrade_source"
ruby -rjson -e '
  path, version = ARGV
  data = JSON.parse(File.read(path))
  data["version"] = version
  File.write(path, JSON.pretty_generate(data) + "\n")
' "$upgrade_source/release-manifest.json" "${release_version}-next"
upgrade_archive="$tmp_dir/a4n-tauri-upgrade.tar.gz"
archive_package "$upgrade_source" "$upgrade_archive"
current_install="$INSTALLED_PACKAGE"
printf 'checking rejection of an incompatible version upgrade candidate\n'
if install_archive "$upgrade_archive" "$install_root" selected; then
    fail "incompatible version upgrade candidate was installed"
fi
[[ "$INSTALLED_PACKAGE" == "$current_install" ]] || fail "failed upgrade replaced the current installation"
assert_persistent_state "$INSTALLED_PACKAGE" "$state_snapshot" || fail "failed version upgrade lost persistent state"
run_fixture_host "$INSTALLED_PACKAGE" after-rejected-upgrade || fail "current installation was damaged by failed upgrade"

protocol_source_parent="$tmp_dir/protocol-source"
protocol_source="$protocol_source_parent/a4n-tauri-fixture"
mkdir -p "$protocol_source_parent"
cp -a "$package_root" "$protocol_source"
ruby -rjson -e '
  path = ARGV.fetch(0)
  data = JSON.parse(File.read(path))
  data["protocol_version"] += 1
  data.fetch("binaries").each { |entry| entry["protocol_version"] = data["protocol_version"] }
  File.write(path, JSON.pretty_generate(data) + "\n")
' "$protocol_source/release-manifest.json"
protocol_archive="$tmp_dir/a4n-tauri-protocol-mismatch.tar.gz"
archive_package "$protocol_source" "$protocol_archive"
printf 'checking rejection of an incompatible protocol upgrade candidate\n'
if install_archive "$protocol_archive" "$install_root" selected; then
    fail "incompatible protocol upgrade candidate was installed"
fi
[[ "$INSTALLED_PACKAGE" == "$current_install" ]] || fail "failed protocol upgrade replaced the current installation"
assert_persistent_state "$INSTALLED_PACKAGE" "$state_snapshot" || fail "failed protocol upgrade lost persistent state"

forged_source_parent="$tmp_dir/forged-source"
forged_source="$forged_source_parent/a4n-tauri-fixture"
mkdir -p "$forged_source_parent"
cp -a "$package_root" "$forged_source"
printf 'forged-unselected-sidecar\n' >"$forged_source/bin/services/$unselected_service"
chmod +x "$forged_source/bin/services/$unselected_service"
forged_archive="$tmp_dir/a4n-tauri-forged-unselected.tar.gz"
archive_package "$forged_source" "$forged_archive"
printf 'checking that an unselected sidecar cannot masquerade as installed\n'
if install_archive "$forged_archive" "$install_root" selected; then
    fail "package containing an unselected sidecar was accepted"
fi
[[ "$INSTALLED_PACKAGE" == "$current_install" ]] || fail "forged package replaced the current installation"
assert_persistent_state "$INSTALLED_PACKAGE" "$state_snapshot" || fail "forged package lost persistent state"

traversal_source="$tmp_dir/fixture-traversal-source"
mkdir -p "$traversal_source"
printf 'archive traversal payload\n' >"$traversal_source/safe"
traversal_archive="$tmp_dir/fixture-path-traversal.tar.gz"
tar --transform='s#^safe$#../escape#' -czf "$traversal_archive" -C "$traversal_source" safe
printf 'checking rejection of an archive path traversal\n'
if install_archive "$traversal_archive" "$install_root" selected; then
    fail "archive path traversal was installed"
fi
[[ "$INSTALLED_PACKAGE" == "$current_install" ]] || fail "path traversal replaced the current installation"
assert_persistent_state "$INSTALLED_PACKAGE" "$state_snapshot" || fail "path traversal lost persistent state"
run_fixture_host "$INSTALLED_PACKAGE" final || fail "valid installation failed after rejected candidates"

printf 'fixture installation check passed: selected host/sidecar, missing-package rejection, checksum, path traversal, uninstall/reinstall state preservation, version/protocol mismatch\n'

if [[ -n "$REAL_ARCHIVE" ]]; then
    check_real_archive "$REAL_ARCHIVE"
fi
