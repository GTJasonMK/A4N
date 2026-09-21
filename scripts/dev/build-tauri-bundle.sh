#!/usr/bin/env bash
#=============================================================================
# Build native Tauri bundle with release externalBin enabled.
#
# Usage:
#   ./scripts/dev/build-tauri-bundle.sh
#   ./scripts/dev/build-tauri-bundle.sh --target x86_64-unknown-linux-gnu
#   ./scripts/dev/build-tauri-bundle.sh --debug --no-bundle
#   ./scripts/dev/build-tauri-bundle.sh --no-build-sidecars
#   ./scripts/dev/build-tauri-bundle.sh --full
#   ./scripts/dev/build-tauri-bundle.sh --dry-run
#
# This wrapper keeps the default tauri.conf.json friendly to cargo check, then
# merges tauri.sidecars.conf.json only for native bundle builds.
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

TAURI_SIDECAR_CONFIG="$PROJECT_ROOT/crates/host/a4n-tauri/tauri.sidecars.conf.json"
PREPARE_ARGS=()
TAURI_ARGS=()
CARGO_BUILD_ARGS=()
FULL_HOST=false
DRY_RUN=false

print_command() {
    local arg
    printf '  '
    for arg in "$@"; do
        printf '%q ' "$arg"
    done
    printf '\n'
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --target)
            if [[ $# -lt 2 || -z "${2:-}" ]]; then
                die "--target 需要提供 Rust target triple"
            fi
            PREPARE_ARGS+=(--target "$2")
            TAURI_ARGS+=(--target "$2")
            shift 2
            ;;
        --debug)
            PREPARE_ARGS+=(--debug)
            TAURI_ARGS+=(--debug)
            shift
            ;;
        --release)
            PREPARE_ARGS+=(--release)
            shift
            ;;
        --full)
            FULL_HOST=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --no-build-sidecars)
            PREPARE_ARGS+=(--no-build)
            shift
            ;;
        --help|-h)
            print_script_usage "$0" 14
            cat <<'EOF'

默认行为:
  build-tauri-bundle.sh 默认构建带启动迁移的 minimal host，并合并发布
  externalBin 配置。生成的 minimal Tauri host 未显式设置 A4N_RUNTIME_PROFILE
  时会以 sidecar + managed sidecars 启动。

  如需构建 embedded/full host，追加 --full。
EOF
            exit 0
            ;;
        *)
            TAURI_ARGS+=("$1")
            shift
            ;;
    esac
done

if $FULL_HOST; then
    CARGO_BUILD_ARGS+=(--features embedded-capabilities)
fi

prepare_command=("$SCRIPT_DIR/prepare-tauri-external-bin.sh" "${PREPARE_ARGS[@]}")
sidecar_package_lines="$(sidecar_catalog_binaries)"
BUNDLE_BINARIES=()
while IFS= read -r package; do
    [[ -z "$package" ]] || BUNDLE_BINARIES+=("$package")
done <<<"$sidecar_package_lines"
tool_binary_lines="$(release_tool_binaries)"
while IFS= read -r binary; do
    BUNDLE_BINARIES+=("$binary")
done <<<"$tool_binary_lines"
tauri_config="$(ruby -rjson -e '
  config = JSON.parse(File.read(ARGV.fetch(0)))
  binaries = ARGV.drop(1)
  abort "release externalBin entries are duplicated" unless binaries.uniq.length == binaries.length
  config.fetch("bundle")["externalBin"] = binaries.map { |binary| "../../../target/tauri-sidecars/#{binary}" }
  print JSON.generate(config)
' "$TAURI_SIDECAR_CONFIG" "${BUNDLE_BINARIES[@]}")"
tauri_command=(cargo tauri build --config "$tauri_config" "${TAURI_ARGS[@]}")
if [[ ${#CARGO_BUILD_ARGS[@]} -gt 0 ]]; then
    tauri_command+=(-- "${CARGO_BUILD_ARGS[@]}")
fi

if $DRY_RUN; then
    log_section "Tauri sidecar bundle dry-run"
    log_info "prepare sidecars:"
    print_command "${prepare_command[@]}"
    log_info "build tauri:"
    log_info "config: tauri.sidecars.conf.json + process-catalog/service-catalog.json + release-tools.json"
    print_command cargo tauri build --config '<generated>' "${TAURI_ARGS[@]}"
    exit 0
fi

"${prepare_command[@]}"
require_onnxruntime

log_section "构建 Tauri 原生包"
(
    cd "$PROJECT_ROOT"
    "${tauri_command[@]}"
)
