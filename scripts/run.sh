#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

load_dotenv
export A4N_PORTABLE_HOME="${A4N_PORTABLE_HOME:-$PROJECT_ROOT}"
cd "$PROJECT_ROOT"

# 当前 GNOME Wayland 环境下 Tauri 窗口可能创建但不映射；有 Xwayland 时优先使用它。
if [[ "${OSTYPE:-}" == linux* && -n "${WAYLAND_DISPLAY:-}" && -n "${DISPLAY:-}" ]]; then
  export WINIT_UNIX_BACKEND="${WINIT_UNIX_BACKEND:-x11}"
  export GDK_BACKEND="${GDK_BACKEND:-x11}"
fi

SERVER=false
HOT_RELOAD=true
RELEASE=false
REMOTE_API=false
REMOTE_API_URL="${A4N_REMOTE_API_URL:-}"
FULL_HOST=false
MANAGED_SIDECARS=true
MODE="local-server"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --release) RELEASE=true; HOT_RELOAD=false ;;
    --server) SERVER=true ;;
    --remote-api) REMOTE_API=true ;;
    --remote-api-url)
      [[ $# -ge 2 && -n "${2:-}" ]] || die "--remote-api-url 需要 URL"
      REMOTE_API=true
      REMOTE_API_URL="$2"
      shift
      ;;
    --mode)
      [[ $# -ge 2 && -n "${2:-}" ]] || die "--mode 需要 native、local-server 或 remote"
      case "$2" in
        native|local-server|remote) MODE="$2" ;;
        *) die "无效 --mode: $2；允许值为 native、local-server 或 remote" ;;
      esac
      shift
      ;;
    --full) FULL_HOST=true ;;
    --no-sidecars) MANAGED_SIDECARS=false ;;
    --no-hot) HOT_RELOAD=false ;;
    --help|-h)
      cat <<'EOF'
用法: ./scripts/run.sh [选项]
  --server            启动 Web Server
  --no-hot            静态前端、关闭 watcher
  --release           Release 模式
  --remote-api[-url]  桌面壳连接远程 API
  --mode MODE          native、local-server（默认）或 remote
  --full              使用 embedded capabilities
  --no-sidecars       不启动 managed sidecar
EOF
      exit 0
      ;;
    *) die "未知参数: $1；使用 --help 查看用法" ;;
  esac
  shift
done

if $REMOTE_API; then
  [[ "$MODE" == "local-server" || "$MODE" == "remote" ]] || die "--remote-api 不能与 --mode $MODE 同时使用"
  MODE=remote
fi

case "$MODE" in
  native)
    unset VITE_A4N_API_BASE_URL
    export VITE_A4N_API_TRANSPORT=tauri
    export A4N_REMOTE_API_ONLY=0
    export A4N_TAURI_LOCAL_SERVER=0
    ;;
  local-server)
    unset VITE_A4N_API_BASE_URL
    export VITE_A4N_API_TRANSPORT=web
    export A4N_REMOTE_API_ONLY=0
    export A4N_TAURI_LOCAL_SERVER=1
    ;;
  remote)
    REMOTE_API=true
    ;;
esac

if $SERVER && [[ "$MODE" != "local-server" ]]; then
  die "--mode 仅适用于 Tauri；--server 请使用默认 local-server 模式"
fi
if $SERVER && $REMOTE_API; then
  die "--remote-api 仅适用于 Tauri 桌面壳"
fi
if ! $SERVER && [[ "$MODE" == "local-server" && "$MANAGED_SIDECARS" == false ]]; then
  die "local-server 模式需要 managed sidecars，不能使用 --no-sidecars"
fi
if $FULL_HOST && ! $MANAGED_SIDECARS; then
  die "--full 与 --no-sidecars 不能同时使用"
fi
if $REMOTE_API && $FULL_HOST; then
  die "--remote-api 与 --full 不能同时使用"
fi
if $REMOTE_API && ! $MANAGED_SIDECARS; then
  die "--remote-api 已禁用本地 Sidecar，不能再使用 --no-sidecars"
fi

if $REMOTE_API; then
  [[ -n "$REMOTE_API_URL" ]] || die "--remote-api 需要 A4N_REMOTE_API_URL"
  export VITE_A4N_API_TRANSPORT=web
  export VITE_A4N_API_BASE_URL="$REMOTE_API_URL"
  export A4N_REMOTE_API_ONLY=1
  export A4N_TAURI_LOCAL_SERVER=0
fi

if ! $SERVER && [[ "$MODE" == "local-server" ]]; then
  migration_command=(cargo build)
  $RELEASE && migration_command+=(--release)
  migration_command+=(-p a4n-migration)
  "${migration_command[@]}"

  server_command=(cargo build)
  $RELEASE && server_command+=(--release)
  server_command+=(-p a4n-server)
  $FULL_HOST && server_command+=(--features embedded-capabilities)
  "${server_command[@]}"
fi

if $SERVER && ! $REMOTE_API; then
  cargo run -p a4n-cli -- local migrate
fi

if $FULL_HOST; then
  require_onnxruntime
  export A4N_RUNTIME_PROFILE=full
  export A4N_MANAGED_SIDECARS=0
elif $MANAGED_SIDECARS && ! $REMOTE_API; then
  require_onnxruntime
  export A4N_RUNTIME_PROFILE=sidecar
  export A4N_MANAGED_SIDECARS=1
  sidecar_package_lines="$(sidecar_catalog_binaries)"
  mapfile -t sidecar_packages <<<"$sidecar_package_lines"
  build_args=()
  $RELEASE && build_args+=(--release)
  for package in "${sidecar_packages[@]}"; do
    build_args+=(-p "$package")
  done
  cargo build "${build_args[@]}"
else
  export A4N_RUNTIME_PROFILE=sidecar
  export A4N_MANAGED_SIDECARS=0
fi

if $SERVER; then
  npm --prefix frontend run build
  command=(cargo run)
  $RELEASE && command+=(--release)
  command+=(-p a4n-server)
  $FULL_HOST && command+=(--features embedded-capabilities)
  exec "${command[@]}"
fi

command=(cargo tauri dev)
$RELEASE && command+=(--release)
$FULL_HOST && command+=(-f embedded-capabilities)
if ! $HOT_RELOAD; then
  npm --prefix frontend run build
  command+=(--no-watch --config crates/a4n-tauri/tauri.no-hot.conf.json)
fi
exec "${command[@]}"
