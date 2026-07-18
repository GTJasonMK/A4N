#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

CHECK_ONLY=false
FULL_HOST=false
AUTO_START_SIDECARS=true
SIDECAR_MODULES="prompt,llm,embedding,rag,image,video,import,export"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check)
      CHECK_ONLY=true
      shift
      ;;
    --full|--embedded)
      FULL_HOST=true
      shift
      ;;
    --no-services)
      AUTO_START_SIDECARS=false
      shift
      ;;
    --help|-h)
      cat <<'EOF'
用法:
  ./scripts/dev/run-web.sh          # 构建前端并启动 a4n-server
  ./scripts/dev/run-web.sh --check  # 只检查 Web 服务必需环境变量
  ./scripts/dev/run-web.sh --full   # 使用 embedded/full host
  ./scripts/dev/run-web.sh --no-services # sidecar profile 下不自动启动 sidecar
  ./scripts/dev/run-web.sh --help   # 显示帮助

必需环境变量:
  A4N_JWT_SECRET
  A4N_DATABASE_URL=postgres://... 或 postgresql://...

可选 bootstrap 管理员变量必须同时设置:
  A4N_BOOTSTRAP_ADMIN_USERNAME
  A4N_BOOTSTRAP_ADMIN_EMAIL
  A4N_BOOTSTRAP_ADMIN_PASSWORD
EOF
      exit 0
      ;;
    *)
      die "未知参数: $1；使用 --help 查看用法"
      ;;
  esac
done

load_dotenv

if [[ -z "${A4N_JWT_SECRET:-}" ]]; then
  die "A4N_JWT_SECRET 不能为空"
fi

if [[ -z "${A4N_DATABASE_URL:-}" ]]; then
  die "A4N_DATABASE_URL 不能为空"
fi
case "$A4N_DATABASE_URL" in
  postgres://*|postgresql://*) ;;
  *)
    die "A4N_DATABASE_URL 必须是 postgres:// 或 postgresql:// 连接串"
    ;;
esac

bootstrap_count=0
[[ -n "${A4N_BOOTSTRAP_ADMIN_USERNAME:-}" ]] && bootstrap_count=$((bootstrap_count + 1))
[[ -n "${A4N_BOOTSTRAP_ADMIN_EMAIL:-}" ]] && bootstrap_count=$((bootstrap_count + 1))
[[ -n "${A4N_BOOTSTRAP_ADMIN_PASSWORD:-}" ]] && bootstrap_count=$((bootstrap_count + 1))
if [[ "$bootstrap_count" -ne 0 && "$bootstrap_count" -ne 3 ]]; then
  die "A4N_BOOTSTRAP_ADMIN_USERNAME、A4N_BOOTSTRAP_ADMIN_EMAIL、A4N_BOOTSTRAP_ADMIN_PASSWORD 必须同时设置"
fi

if $CHECK_ONLY; then
  log_ok "Web 服务环境变量检查通过"
  exit 0
fi

cd frontend
npm run build
cd "$PROJECT_ROOT"

guard_build_cache

if $FULL_HOST; then
  require_onnxruntime
  export A4N_RUNTIME_PROFILE="${A4N_RUNTIME_PROFILE:-full}"
  exec cargo run -p a4n-server --features embedded-capabilities
fi

export A4N_RUNTIME_PROFILE="${A4N_RUNTIME_PROFILE:-sidecar}"
if $AUTO_START_SIDECARS; then
  log_section "启动 Sidecar 服务"
  "$PROJECT_ROOT/scripts/dev/run-services.sh" "$SIDECAR_MODULES"
fi
exec cargo run -p a4n-server --no-default-features
