#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

load_dotenv
export A4N_PORTABLE_HOME="${A4N_PORTABLE_HOME:-$PROJECT_ROOT}"
cd "$PROJECT_ROOT"

MODE=tauri
HOT_RELOAD=true
RELEASE=false
REMOTE_API=false
REMOTE_API_URL="${A4N_REMOTE_API_URL:-}"
FULL_HOST=false
MANAGED_SIDECARS=true
MIGRATE_ONLY=false
CHECK_ONLY=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --release|-r) RELEASE=true; HOT_RELOAD=false ;;
    --server|-s) MODE=server ;;
    --frontend|-f) MODE=frontend ;;
    --remote-api) REMOTE_API=true ;;
    --remote-api-url)
      [[ $# -ge 2 && -n "${2:-}" ]] || die "--remote-api-url 需要 URL"
      REMOTE_API=true
      REMOTE_API_URL="$2"
      shift
      ;;
    --full|--embedded) FULL_HOST=true ;;
    --no-services) MANAGED_SIDECARS=false ;;
    --migrate|-m) MIGRATE_ONLY=true ;;
    --no-hot|--no-reload) HOT_RELOAD=false ;;
    --check) CHECK_ONLY=true ;;
    --help|-h)
      cat <<'EOF'
用法: ./scripts/run.sh [选项]
  --server            启动 Web Server
  --frontend          仅启动 Vite
  --no-hot            静态前端、关闭 watcher
  --release           Release 模式
  --remote-api[-url]  桌面壳连接远程 API
  --full              使用 embedded capabilities
  --no-services       不启动 managed sidecar
  --migrate           仅迁移数据库
  --check             运行 a4n doctor
EOF
      exit 0
      ;;
    *) die "未知参数: $1；使用 --help 查看用法" ;;
  esac
  shift
done

if $REMOTE_API; then
  [[ -n "$REMOTE_API_URL" ]] || die "--remote-api 需要 A4N_REMOTE_API_URL"
  export VITE_A4N_API_TRANSPORT=web
  export VITE_A4N_API_BASE_URL="$REMOTE_API_URL"
  export A4N_REMOTE_API_ONLY=1
fi

if $CHECK_ONLY; then
  exec cargo run -p a4n-cli -- doctor
fi

if [[ "$MODE" == frontend ]]; then
  exec npm --prefix frontend run dev
fi

guard_build_cache

if $MIGRATE_ONLY; then
  $REMOTE_API && exit 0
  exec cargo run -p a4n-cli -- local migrate
fi

if ! $REMOTE_API; then
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
  mapfile -t sidecar_packages < <(
    sed -n 's/.*"binary":"\([^"]*\)".*/\1/p' \
      crates/a4n-service-protocol/service-catalog.json
  )
  [[ ${#sidecar_packages[@]} -gt 0 ]] || die "sidecar 服务目录为空"
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

if [[ "$MODE" == server ]]; then
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
