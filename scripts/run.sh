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
REUSE_SERVER=false
REUSE_SERVER_URL="${A4N_REUSE_SERVER_URL:-}"
RESTART_SERVER=false
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
    --reuse-server)
      REUSE_SERVER=true
      ;;
    --reuse-server-url)
      [[ $# -ge 2 && -n "${2:-}" ]] || die "--reuse-server-url 需要 URL"
      REUSE_SERVER=true
      REUSE_SERVER_URL="$2"
      shift
      ;;
    --restart-server)
      RESTART_SERVER=true
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
  --reuse-server       local-server 模式复用当前数据库已有 Server
  --reuse-server-url   指定要复用的 Server URL（隐含 --reuse-server）
  --restart-server     启动前停止当前数据库已有的 A4N Server
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

if $REUSE_SERVER && $RESTART_SERVER; then
  die "--reuse-server 与 --restart-server 不能同时使用"
fi
if $REUSE_SERVER && ($SERVER || [[ "$MODE" != "local-server" ]]); then
  die "--reuse-server 仅适用于 Tauri local-server 模式"
fi
if $RESTART_SERVER && [[ "$MODE" == "native" || "$MODE" == "remote" ]]; then
  die "--restart-server 不能与 native/remote 模式同时使用"
fi

server_process_pids() {
  local database_url="${A4N_DATABASE_URL:-}"
  [[ -n "$database_url" ]] || return 0

  local pid executable process_database
  while read -r pid; do
    executable="$(ps -p "$pid" -o comm= 2>/dev/null || true)"
    [[ "${executable##*/}" == a4n-server* ]] || continue
    process_database="$(process_env_value "$pid" A4N_DATABASE_URL)"
    [[ "$process_database" == "$database_url" ]] || continue
    printf '%s\n' "$pid"
  done < <(pgrep -f 'a4n-server' 2>/dev/null || true)
}

process_env_value() {
  local pid="$1" name="$2"
  if [[ -r "/proc/$pid/environ" ]]; then
    tr '\0' '\n' < "/proc/$pid/environ" 2>/dev/null \
      | sed -n "s/^${name}=//p" \
      | head -n 1
    return
  fi
  ps eww -p "$pid" -o command= 2>/dev/null \
    | tr ' ' '\n' \
    | sed -n "s/^${name}=//p" \
    | head -n 1
}

server_url_for_pid() {
  local pid="$1" address
  address="$(process_env_value "$pid" A4N_WEB_ADDR)"
  [[ -n "$address" ]] || return 1
  printf 'http://%s/api\n' "$address"
}

normalize_server_url() {
  local server_url="${1%/}"
  [[ "$server_url" == */api ]] || server_url="$server_url/api"
  printf '%s\n' "$server_url"
}

server_health_url() {
  local server_url="${1%/}"
  printf '%s/health\n' "$server_url"
}

find_reusable_server_url() {
  local candidate pid
  if [[ -n "$REUSE_SERVER_URL" ]]; then
    candidate="$(normalize_server_url "$REUSE_SERVER_URL")"
    curl -fsS --max-time 3 "$(server_health_url "$candidate")" >/dev/null \
      || die "指定的复用 Server 不健康: $candidate"
    printf '%s\n' "$candidate"
    return
  fi

  while read -r pid; do
    candidate="$(server_url_for_pid "$pid" || true)"
    [[ -n "$candidate" ]] || continue
    if curl -fsS --max-time 3 "$(server_health_url "$candidate")" >/dev/null 2>&1; then
      printf '%s\n' "$candidate"
      return
    fi
  done < <(server_process_pids)
  die "没有找到可复用的同数据库 A4N Server；请先启动 Server，或使用 --restart-server"
}

stop_existing_servers() {
  local pids pid deadline
  mapfile -t pids < <(server_process_pids)
  ((${#pids[@]} > 0)) || return 0

  printf '停止同数据库 A4N Server: %s\n' "${pids[*]}" >&2
  for pid in "${pids[@]}"; do
    kill -TERM "$pid" 2>/dev/null || true
  done
  deadline=$((SECONDS + 10))
  while ((SECONDS < deadline)); do
    local remaining=0
    for pid in "${pids[@]}"; do
      if kill -0 "$pid" 2>/dev/null; then
        remaining=1
        break
      fi
    done
    ((remaining == 0)) && return
    sleep 1
  done

  for pid in "${pids[@]}"; do
    if server_process_pids | grep -Fxq "$pid"; then
      printf '强制停止未退出的 A4N Server: %s\n' "$pid" >&2
      kill -KILL "$pid" 2>/dev/null || true
    fi
  done
}

if $RESTART_SERVER; then
  stop_existing_servers
fi

if $REUSE_SERVER; then
  REMOTE_API=true
  MODE=remote
  REMOTE_API_URL="$(find_reusable_server_url)"
fi

qdrant_ready() {
  curl -fsS --max-time 1 http://127.0.0.1:6333/readyz >/dev/null 2>&1 \
    && (exec 3<>/dev/tcp/127.0.0.1/6334) 2>/dev/null
}

qdrant_reachable() {
  curl --http2-prior-knowledge -sS --max-time 2 -o /dev/null "${1%/}/" 2>/dev/null
}

ensure_qdrant() {
  local qdrant_url="${A4N_QDRANT_URL:-}" qdrant_log qdrant_pid
  qdrant_url="${qdrant_url%/}"
  [[ -n "$qdrant_url" ]] || die "本地 RAG 需要 A4N_QDRANT_URL；请设置为 http://127.0.0.1:6334"

  case "$qdrant_url" in
    http://127.0.0.1:6334|http://localhost:6334)
      qdrant_ready && return
      ;;
    *)
      qdrant_reachable "$qdrant_url" && return
      die "A4N_QDRANT_URL $qdrant_url 不可连接；远程或自定义端口 Qdrant 请由外部管理"
      ;;
  esac

  [[ -x "$PROJECT_ROOT/tools/qdrant/bin/qdrant" ]] \
    || die "本地 Qdrant 未安装；请先运行 ./scripts/dev/setup-qdrant.sh"
  qdrant_log="${A4N_LOG_DIR:-data/logs}/qdrant.log"
  [[ "$qdrant_log" = /* ]] || qdrant_log="$PROJECT_ROOT/$qdrant_log"
  mkdir -p "$(dirname "$qdrant_log")"
  printf '启动本地 Qdrant；日志: %s\n' "$qdrant_log" >&2
  nohup "$SCRIPT_DIR/dev/run-qdrant.sh" >"$qdrant_log" 2>&1 </dev/null &
  qdrant_pid=$!
  wait_for_qdrant "$qdrant_pid" "$qdrant_log"
}

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

if ! $REMOTE_API && ($MANAGED_SIDECARS || $FULL_HOST); then
  ensure_qdrant
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
