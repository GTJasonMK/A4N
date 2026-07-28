#!/usr/bin/env bash
#=============================================================================
# A4N 项目运行脚本（一键启动 / 默认热重载）
#
# 用法:
#   ./scripts/run.sh                 # 开发模式（精细热重载）：
#                                    #   - Vite HMR 永不重启（前端 session 保留）
#                                    #   - cargo-watch 监视 Rust，改动后只重启二进制
#   ./scripts/dev/run-no-hot.sh      # 静态前端入口（先构建 dist，不启动 Vite/watch）
#   ./scripts/run.sh --no-hot        # 同 ./scripts/dev/run-no-hot.sh
#   ./scripts/run.sh --no-reload     # 同 --no-hot
#   ./scripts/run.sh --release       # Release 模式运行
#   ./scripts/run.sh --server        # 仅启动后端（无 GUI 窗口）
#   ./scripts/run.sh --frontend      # 仅启动前端开发服务器（Vite HMR）
#   A4N_REMOTE_API_URL=https://server.example.com/a4n/api ./scripts/run.sh --remote-api
#   ./scripts/run.sh --remote-api-url https://server.example.com/a4n/api
#   ./scripts/run.sh --full          # 使用 embedded/full host（会编译重 provider）
#   ./scripts/run.sh --no-services   # sidecar profile 下不自动启动 sidecar
#   ./scripts/run.sh --migrate       # 仅运行数据库迁移
#   ./scripts/run.sh --check         # 检查运行环境
#   ./scripts/run.sh --help          # 显示帮助
#
# 跨平台：Linux / macOS / Windows(Git Bash / WSL)
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# —— 配置 ——
load_dotenv

FRONTEND_DIR="$PROJECT_ROOT/frontend"
DEFAULT_VITE_PORT=5173

APP_PORTABLE_HOME="$(resolve_project_path "${A4N_PORTABLE_HOME:-$PROJECT_ROOT}")"
export A4N_PORTABLE_HOME="$APP_PORTABLE_HOME"
APP_DATA_DIR="$(resolve_project_path "${A4N_DATA_DIR:-$APP_PORTABLE_HOME/data}")"
APP_CACHE_DIR="$(resolve_project_path "${A4N_CACHE_DIR:-$APP_DATA_DIR/cache}")"
APP_LOG_DIR="$(resolve_project_path "${A4N_LOG_DIR:-$APP_DATA_DIR/logs}")"
APP_TMP_DIR="$(resolve_project_path "${A4N_TEMP_DIR:-$APP_DATA_DIR/tmp}")"
export A4N_DATA_DIR="$APP_DATA_DIR"
export A4N_CACHE_DIR="$APP_CACHE_DIR"
export A4N_LOG_DIR="$APP_LOG_DIR"
export A4N_TEMP_DIR="$APP_TMP_DIR"
VITE_PORT_FILE="$APP_TMP_DIR/vite-port"
VITE_PID_FILE="$APP_TMP_DIR/vite.pid"
TAURI_PID_FILE="$APP_TMP_DIR/tauri.pid"
CARGO_WATCH_PID_FILE="$APP_TMP_DIR/cargo-watch.pid"
VITE_LOG_FILE="$APP_LOG_DIR/a4n-vite.log"
RUN_MODE="dev"
RUN_MIGRATE_ONLY=false
CHECK_ONLY=false
FRONTEND_ONLY=false
HOT_RELOAD=true  # 默认开启精细热重载
REMOTE_API=false
REMOTE_API_URL="${A4N_REMOTE_API_URL:-}"
FULL_HOST=false
AUTO_START_SIDECARS=true
SIDECAR_MODULES="prompt,llm,embedding,rag,image,video,import,export"
HOST_CARGO_RUN_ARGS=()
if [[ -n "${A4N_HOST_CARGO_RUN_ARGS:-}" ]]; then
    # shellcheck disable=SC2206
    HOST_CARGO_RUN_ARGS=(${A4N_HOST_CARGO_RUN_ARGS})
fi
TAURI_DEV_ARGS=()
if [[ -n "${A4N_TAURI_DEV_ARGS:-}" ]]; then
    # shellcheck disable=SC2206
    TAURI_DEV_ARGS=(${A4N_TAURI_DEV_ARGS})
fi

# —— 标记：trap 是否已注册 ——
TRAP_REGISTERED=false

# —— 解析参数 ——
while [[ $# -gt 0 ]]; do
    case "$1" in
        --release|-r)
            RUN_MODE="release"
            shift
            ;;
        --server|-s)
            RUN_MODE="server"
            shift
            ;;
        --frontend|-f)
            FRONTEND_ONLY=true
            shift
            ;;
        --remote-api)
            REMOTE_API=true
            shift
            ;;
        --remote-api-url)
            REMOTE_API=true
            if [[ $# -lt 2 || -z "${2:-}" ]]; then
                echo -e "${RED}--remote-api-url 需要提供 URL，例如 https://server.example.com/a4n/api${NC}"
                exit 1
            fi
            REMOTE_API_URL="$2"
            shift 2
            ;;
        --full|--embedded)
            FULL_HOST=true
            shift
            ;;
        --no-services)
            AUTO_START_SIDECARS=false
            shift
            ;;
        --migrate|-m)
            RUN_MIGRATE_ONLY=true
            shift
            ;;
        --no-hot|--no-reload)
            HOT_RELOAD=false
            shift
            ;;
        --check)
            CHECK_ONLY=true
            shift
            ;;
        --help|-h)
            print_script_usage "$0" 21
            exit 0
            ;;
        *)
            die "未知参数: $1；使用 --help 查看用法"
            ;;
    esac
done

if $REMOTE_API && [[ -z "$REMOTE_API_URL" ]]; then
    die "--remote-api 需要 A4N_REMOTE_API_URL，或改用 --remote-api-url URL"
fi

reload_cargo_args_from_env() {
    HOST_CARGO_RUN_ARGS=()
    if [[ -n "${A4N_HOST_CARGO_RUN_ARGS:-}" ]]; then
        # shellcheck disable=SC2206
        HOST_CARGO_RUN_ARGS=(${A4N_HOST_CARGO_RUN_ARGS})
    fi
    TAURI_DEV_ARGS=()
    if [[ -n "${A4N_TAURI_DEV_ARGS:-}" ]]; then
        # shellcheck disable=SC2206
        TAURI_DEV_ARGS=(${A4N_TAURI_DEV_ARGS})
    fi
}

configure_runtime_profile() {
    local profile
    if $FULL_HOST; then
        profile="full"
        export A4N_HOST_CARGO_RUN_ARGS="${A4N_HOST_CARGO_RUN_ARGS:---features embedded-capabilities}"
        export A4N_TAURI_DEV_ARGS="${A4N_TAURI_DEV_ARGS:---features embedded-capabilities}"
    else
        profile="${A4N_RUNTIME_PROFILE:-sidecar}"
    fi
    export A4N_RUNTIME_PROFILE="$profile"

    case "${profile,,}" in
        sidecar|minimal)
            export A4N_HOST_CARGO_RUN_ARGS="${A4N_HOST_CARGO_RUN_ARGS:---no-default-features}"
            export A4N_TAURI_DEV_ARGS="${A4N_TAURI_DEV_ARGS:---no-default-features}"
            ;;
    esac

    reload_cargo_args_from_env
}

runtime_uses_sidecars() {
    [[ "${A4N_RUNTIME_PROFILE,,}" == "sidecar" ]]
}

# ========== 进程清理与资源回收 ==========

# 按端口终止进程
kill_port() {
    local port=$1
    local pids
    pids=$(lsof -ti:"$port" 2>/dev/null || true)
    if [[ -n "$pids" ]]; then
        echo -e "  ${YELLOW}→${NC} 清理端口 ${port} 上的残留进程: $pids"
        kill $pids 2>/dev/null || true
        sleep 0.3
        # 如果还没死，强制结束
        kill -9 $pids 2>/dev/null || true
    fi
}

terminate_stale_tauri_processes() {
    local force_kill=${1:-false}
    local announce=${2:-false}
    local pids
    pids=$(pgrep -f "target/debug/a4n-tauri" 2>/dev/null || true)
    if [[ -z "$pids" ]]; then
        return
    fi

    if [[ "$announce" == "true" ]]; then
        echo -e "  ${YELLOW}⚠${NC} 发现残留 a4n-tauri 进程 (PID: $pids)，正在清理..."
    fi
    kill -TERM $pids 2>/dev/null || true
    if [[ "$force_kill" == "true" ]]; then
        ( sleep 0.2 && kill -KILL $pids 2>/dev/null || true ) &
    fi
}

ensure_runtime_dirs() {
    mkdir -p "$APP_LOG_DIR" "$APP_TMP_DIR"
}

configure_frontend_api_transport() {
    if $REMOTE_API; then
        export VITE_A4N_API_TRANSPORT="web"
        export VITE_A4N_API_BASE_URL="$REMOTE_API_URL"
        export A4N_REMOTE_API_ONLY=1
        echo -e "  ${CYAN}→${NC} 前端业务 API: ${CYAN}${VITE_A4N_API_BASE_URL}${NC}"
        echo -e "  ${YELLOW}提示: Tauri 只作为本地桌面壳启动，业务数据和生成请求连接远程后端${NC}"
        echo ""
        return
    fi

    # no-hot 使用 Tauri CLI 的静态开发服务器，但不会启动 Vite 或 watcher。
    if [[ "$RUN_MODE" == "dev" ]] && ! $HOT_RELOAD; then
        if cargo tauri --version &>/dev/null; then
            echo -e "  ${GREEN}✓${NC} cargo-tauri $(cargo tauri --version 2>/dev/null || echo 'v2')"
        else
            echo -e "  ${YELLOW}⚠${NC} Tauri CLI 未安装，正在自动安装..."
            cargo install tauri-cli --version "^2" 2>&1 || {
                echo -e "${RED}✗ Tauri CLI 安装失败，请手动执行: cargo install tauri-cli${NC}"
                exit 1
            }
            echo -e "  ${GREEN}✓${NC} Tauri CLI 安装完成"
        fi
    fi

    if [[ -n "${VITE_A4N_API_TRANSPORT:-}" || -n "${VITE_A4N_API_BASE_URL:-}" ]]; then
        echo -e "  ${CYAN}→${NC} 前端业务 API 环境变量: transport=${VITE_A4N_API_TRANSPORT:-auto} base=${VITE_A4N_API_BASE_URL:-/api}"
        echo ""
    fi
}

configure_linux_input_method() {
    if [[ "$(uname -s)" != "Linux" ]]; then
        return
    fi

    local im_module=""
    if [[ "${XMODIFIERS:-}" =~ @im=([^[:space:]]+) ]]; then
        im_module="${BASH_REMATCH[1]}"
    elif [[ -n "${GTK_IM_MODULE:-}" ]]; then
        im_module="$GTK_IM_MODULE"
    elif [[ -n "${QT_IM_MODULE:-}" ]]; then
        im_module="$QT_IM_MODULE"
    fi

    if [[ -z "$im_module" ]]; then
        return
    fi

    if [[ -z "${GTK_IM_MODULE:-}" ]]; then
        export GTK_IM_MODULE="$im_module"
    fi
    if [[ -z "${QT_IM_MODULE:-}" ]]; then
        export QT_IM_MODULE="$im_module"
    fi
    if [[ -z "${XMODIFIERS:-}" ]]; then
        export XMODIFIERS="@im=$im_module"
    fi
}

has_configured_jwt_secret() {
    if [[ -n "${A4N_JWT_SECRET:-}" ]]; then
        return 0
    fi

    local config_file="$APP_DATA_DIR/config.toml"
    if [[ -f "$config_file" ]] && grep -Eq '^[[:space:]]*jwt_secret[[:space:]]*=[[:space:]]*"[^"]+"' "$config_file"; then
        return 0
    fi

    return 1
}

validate_database_url() {
    local database_url="$1"
    case "$database_url" in
        postgres://*|postgresql://*) return 0 ;;
        *) return 1 ;;
    esac
}

database_endpoint() {
    local database_url="$1"
    local without_scheme="${database_url#postgres://}"
    without_scheme="${without_scheme#postgresql://}"
    local authority="${without_scheme%%/*}"
    authority="${authority%%\?*}"
    authority="${authority##*@}"

    local host port
    if [[ "$authority" == \[*\]* ]]; then
        host="${authority%%]*}"
        host="${host#[}"
        local rest="${authority#*]}"
        port="${rest#:}"
    else
        host="${authority%%:*}"
        port="${authority#*:}"
        if [[ "$port" == "$authority" ]]; then
            port="5432"
        fi
    fi

    if [[ -z "$host" || -z "$port" ]]; then
        return 1
    fi
    printf '%s %s\n' "$host" "$port"
}

check_database_reachable() {
    local database_url="$1"
    local endpoint host port
    if ! endpoint="$(database_endpoint "$database_url")"; then
        echo -e "  ${RED}✗${NC} 无法解析 PostgreSQL 地址"
        return 1
    fi
    read -r host port <<< "$endpoint"

    if timeout 2 bash -c "</dev/tcp/${host}/${port}" 2>/dev/null; then
        echo -e "  ${GREEN}✓${NC} PostgreSQL 可连接: ${host}:${port}"
        return 0
    fi

    echo -e "  ${YELLOW}⚠${NC} PostgreSQL 暂不可连接: ${host}:${port}"
    echo -e "    如使用本地 Docker 数据库，请先执行：${YELLOW}./scripts/dev/dev-db.sh up${NC}"
    echo -e "    当前项目已配置连接串，但数据库服务还没有启动。"
    return 1
}

configured_database_url() {
    if [[ -n "${A4N_DATABASE_URL:-}" ]]; then
        printf '%s\n' "$A4N_DATABASE_URL"
        return 0
    fi

    local config_file="$APP_DATA_DIR/config.toml"
    if [[ -f "$config_file" ]]; then
        local value
        value=$(sed -nE 's/^[[:space:]]*database_url[[:space:]]*=[[:space:]]*"([^"]+)".*$/\1/p' "$config_file" | head -n 1)
        if [[ -n "$value" ]]; then
            printf '%s\n' "$value"
            return 0
        fi
    fi

    return 1
}

# 清理所有子进程和残留资源（带重入防护）
cleanup() {
    # 重入防护：只执行一次
    if [[ -n "${_CLEANUP_DONE:-}" ]]; then
        return
    fi
    _CLEANUP_DONE=1

    # 在清理函数内部关闭 errexit，防止子命令失败导致清理中断
    set +e

    if $TRAP_REGISTERED; then
        echo ""
        echo -e "${CYAN}━━━ 资源回收 ━━━${NC}"
    fi

    # 先解除 trap，防止信号循环触发
    trap - EXIT INT TERM

    # 清理 Vite 开发服务器
    kill_port "$DEFAULT_VITE_PORT"

    # 清理通过 PID 文件记录的进程
    for pid_file in "$VITE_PID_FILE" "$TAURI_PID_FILE" "$CARGO_WATCH_PID_FILE"; do
        if [[ -f "$pid_file" ]]; then
            local pid
            pid=$(cat "$pid_file" 2>/dev/null || true)
            if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
                local label
                case "$pid_file" in
                    *vite*) label="Vite";;
                    *tauri*) label="Tauri";;
                    *cargo-watch*) label="cargo-watch";;
                    *) label="子进程";;
                esac
                echo -e "  ${YELLOW}→${NC} 终止 ${label} 进程 (PID: $pid)"
                # 先 TERM，给清理机会
                kill -TERM "$pid" 2>/dev/null || true
                # 200ms 后如果还活着，KILL
                ( sleep 0.2 && kill -KILL "$pid" 2>/dev/null || true ) &
            fi
            rm -f "$pid_file"
        fi
    done

    # 清理 cargo-watch 重启过程中可能遗留的 a4n-tauri 进程
    terminate_stale_tauri_processes true false

    rm -f "$VITE_PORT_FILE" "$VITE_PID_FILE" "$TAURI_PID_FILE" "$CARGO_WATCH_PID_FILE"

    if $TRAP_REGISTERED; then
        echo -e "${GREEN}✓ 资源回收完成${NC}"
    fi
}

# 注册信号处理
register_trap() {
    if ! $TRAP_REGISTERED; then
        TRAP_REGISTERED=true
        trap cleanup EXIT INT TERM
    fi
}

# 启动前清理残留进程
preflight_cleanup() {
    echo -e "  ${CYAN}→${NC} 检查端口 ${DEFAULT_VITE_PORT} 占用..."

    # 先尝试优雅终止
    local pids
    pids=$(lsof -ti:"$DEFAULT_VITE_PORT" 2>/dev/null || true)
    if [[ -n "$pids" ]]; then
        echo -e "  ${YELLOW}⚠${NC} 端口 ${DEFAULT_VITE_PORT} 被占用 (PID: $pids)，正在清理..."
        kill_port "$DEFAULT_VITE_PORT"
        echo -e "  ${GREEN}✓${NC} 端口已释放"
    else
        echo -e "  ${GREEN}✓${NC} 端口 ${DEFAULT_VITE_PORT} 可用"
    fi

    # 清理残留的 a4n-tauri 进程（非当前会话）
    terminate_stale_tauri_processes false true
}

# —— 环境检查 ——
check_environment() {
    echo -e "${CYAN}━━━ 环境检查 ━━━${NC}"

    # Rust
    if command -v cargo &>/dev/null; then
        echo -e "  ${GREEN}✓${NC} cargo $(cargo --version | cut -d' ' -f2)"
    else
        echo -e "  ${RED}✗${NC} cargo 未安装"
        exit 1
    fi

    # Node.js（前端必需）
    if command -v node &>/dev/null; then
        echo -e "  ${GREEN}✓${NC} node $(node --version)"
    else
        echo -e "  ${RED}✗${NC} Node.js 未安装（前端构建必需）${NC}"
        exit 1
    fi

    # npm
    if command -v npm &>/dev/null; then
        echo -e "  ${GREEN}✓${NC} npm $(npm --version)"
    else
        echo -e "  ${RED}✗${NC} npm 未安装${NC}"
        exit 1
    fi

    if $REMOTE_API; then
        echo -e "  ${GREEN}✓${NC} 远程 API 模式：跳过本地 JWT 和 PostgreSQL 检查"
    else
        if has_configured_jwt_secret; then
            echo -e "  ${GREEN}✓${NC} JWT secret 已配置"
        else
            echo -e "  ${RED}✗${NC} 缺少 JWT secret"
            echo -e "    请设置环境变量：${YELLOW}export A4N_JWT_SECRET=replace-with-a-long-random-secret${NC}"
            echo -e "    或在 ${YELLOW}${APP_DATA_DIR}/config.toml${NC} 中添加："
            echo -e "    ${YELLOW}[security]${NC}"
            echo -e "    ${YELLOW}jwt_secret = \"replace-with-a-long-random-secret\"${NC}"
            exit 1
        fi

        local database_url
        if ! database_url="$(configured_database_url)"; then
            echo -e "  ${RED}✗${NC} 缺少 PostgreSQL 连接串"
            echo -e "    请设置环境变量：${YELLOW}export A4N_DATABASE_URL=postgres://user:password@host:5432/database${NC}"
            echo -e "    或在 ${YELLOW}${APP_DATA_DIR}/config.toml${NC} 中添加："
            echo -e "    ${YELLOW}database_url = \"postgres://user:password@host:5432/database\"${NC}"
            exit 1
        fi
        if validate_database_url "$database_url"; then
            echo -e "  ${GREEN}✓${NC} PostgreSQL 连接串已配置"
        else
            echo -e "  ${RED}✗${NC} 数据库连接串必须是 postgres:// 或 postgresql://"
            exit 1
        fi
        check_database_reachable "$database_url" || exit 1
    fi

    # cargo-watch（精细热重载需要）
    if [[ "$RUN_MODE" == "dev" ]] && $HOT_RELOAD; then
        # 通过 cargo subcommand 探测（不依赖 PATH）
        if cargo watch --version &>/dev/null; then
            echo -e "  ${GREEN}✓${NC} cargo-watch $(cargo watch --version 2>/dev/null | cut -d' ' -f2 | head -1)"
        else
            echo -e "  ${YELLOW}⚠${NC} cargo-watch 未安装（精细热重载需要），正在自动安装..."
            cargo install cargo-watch 2>&1 || {
                echo -e "${RED}✗ cargo-watch 安装失败，请手动执行: cargo install cargo-watch${NC}"
                exit 1
            }
            echo -e "  ${GREEN}✓${NC} cargo-watch 安装完成"
        fi
    fi

    # 前端依赖检查
    if [[ -d "$FRONTEND_DIR/node_modules" ]]; then
        echo -e "  ${GREEN}✓${NC} 前端依赖已安装"
    else
        echo -e "  ${YELLOW}⚠${NC} 前端依赖未安装，将自动安装..."
        install_frontend_deps
    fi

    # 系统
    local os_name
    os_name=$(uname -s)
    echo -e "  ${GREEN}✓${NC} 操作系统: ${os_name}"

    # Linux 下的 Tauri 依赖
    if [[ "$os_name" == "Linux" ]]; then
        echo -e "  ${CYAN}→${NC} 检查 Tauri 系统依赖..."

        local missing_libs=()
        local pkg version
        for pkg in webkit2gtk-4.1 gtk+-3.0 javascriptcoregtk-4.1; do
            if pkg-config --exists "$pkg"; then
                version=$(pkg-config --modversion "$pkg" 2>/dev/null || true)
                echo -e "    ${GREEN}✓${NC} ${pkg} ${version}"
            else
                echo -e "    ${RED}✗${NC} ${pkg}"
                missing_libs+=("$pkg")
            fi
        done

        if pkg-config --exists appindicator3-0.1; then
            version=$(pkg-config --modversion appindicator3-0.1 2>/dev/null || true)
            echo -e "    ${GREEN}✓${NC} appindicator3-0.1 ${version}"
        elif pkg-config --exists ayatana-appindicator3-0.1; then
            version=$(pkg-config --modversion ayatana-appindicator3-0.1 2>/dev/null || true)
            echo -e "    ${GREEN}✓${NC} ayatana-appindicator3-0.1 ${version}"
        else
            echo -e "    ${RED}✗${NC} appindicator3-0.1 / ayatana-appindicator3-0.1"
            missing_libs+=("appindicator3-0.1")
        fi

        if [[ ${#missing_libs[@]} -gt 0 ]]; then
            echo ""
            echo -e "${YELLOW}安装缺失依赖：${NC}"
            echo "  Arch:   sudo pacman -S webkit2gtk-4.1 gtk3 libappindicator"
            echo "  Ubuntu: sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev \\"
            echo "          libayatana-appindicator3-dev libjavascriptcoregtk-4.1-dev"
            echo ""
            if ! $FRONTEND_ONLY && ! $RUN_MIGRATE_ONLY; then
                echo -e "${RED}✗ 缺少 Tauri 系统依赖，无法启动桌面端${NC}"
                exit 1
            fi
        fi
    elif [[ "$os_name" == "Darwin" ]]; then
        echo -e "  ${GREEN}✓${NC} macOS（无需额外系统依赖）"
    fi

    # 磁盘空间
    local available_space
    available_space=$(df -h "$PROJECT_ROOT" | tail -1 | awk '{print $4}')
    echo -e "  ${GREEN}✓${NC} 可用磁盘空间: ${available_space}"

    # 数据目录
    local data_dir="$APP_DATA_DIR"
    if [[ ! -d "$data_dir" ]]; then
        mkdir -p "$data_dir"
        echo -e "  ${CYAN}→${NC} 已创建数据目录: ${data_dir}"
    fi
    ensure_runtime_dirs

    # 配置文件检查
    local config_file="$APP_DATA_DIR/config.toml"
    if [[ ! -f "$config_file" ]]; then
        echo -e "  ${YELLOW}⚠${NC} 未找到配置文件，首次启动会生成: ${config_file}"
    else
        echo -e "  ${GREEN}✓${NC} 配置文件: ${config_file}"
    fi

    echo ""
}

# —— 安装前端依赖 ——
install_frontend_deps() {
    echo -e "${CYAN}→${NC} 正在安装前端依赖..."
    cd "$FRONTEND_DIR"
    npm install --silent 2>&1 || {
        echo -e "${RED}✗ 前端依赖安装失败${NC}"
        exit 1
    }
    echo -e "  ${GREEN}✓${NC} 前端依赖安装完成"
    cd "$PROJECT_ROOT"
}

# —— 运行数据库迁移 ——
run_migration() {
    echo -e "${CYAN}━━━ 数据库迁移 ━━━${NC}"

    cd "$PROJECT_ROOT"

    local database_url
    database_url="$(configured_database_url)"
    echo -e "  使用项目迁移器: ${CYAN}${database_url}${NC}"
    cargo run -p a4n-migration -- "$database_url" 2>&1 || {
        echo -e "${RED}✗ 数据库迁移失败${NC}"
        exit 1
    }

    echo -e "  ${GREEN}✓${NC} 数据库 schema 已同步"

    echo ""
}

start_dev_sidecars() {
    if ! runtime_uses_sidecars || ! $AUTO_START_SIDECARS || $REMOTE_API; then
        return
    fi

    echo -e "${CYAN}━━━ 启动 Sidecar 服务 ━━━${NC}"
    echo -e "  profile: ${CYAN}${A4N_RUNTIME_PROFILE}${NC}"
    echo -e "  services: ${CYAN}${SIDECAR_MODULES}${NC}"
    echo ""
    "$PROJECT_ROOT/scripts/dev/run-services.sh" "$SIDECAR_MODULES"
    echo ""
}

# —— 查找可用端口（从起始端口递增） ——
find_available_port() {
    local port=$1
    local max_tries=${2:-20}
    local tries=0
    while lsof -ti:"$port" &>/dev/null && [[ $tries -lt $max_tries ]]; do
        ((port++))
        ((tries++))
    done
    echo "$port"
}

# —— 启动前端开发服务器 ——
launch_frontend() {
    echo -e "${CYAN}━━━ 启动前端开发服务器 ━━━${NC}"

    local actual_port
    actual_port=$(find_available_port "$DEFAULT_VITE_PORT")

    if [[ "$actual_port" != "$DEFAULT_VITE_PORT" ]]; then
        echo -e "  ${YELLOW}⚠${NC} 端口 ${DEFAULT_VITE_PORT} 被占用，使用端口 ${actual_port}"
    fi

    echo -e "  地址: ${CYAN}http://localhost:${actual_port}${NC}"
    echo -e "  ${YELLOW}提示: Vite HMR 已启用，修改代码即时生效${NC}"
    echo ""

    ensure_runtime_dirs
    cd "$FRONTEND_DIR"
    VITE_PORT="$actual_port" npm run dev 2>&1 &
    local vite_pid=$!
    echo "$vite_pid" > "$VITE_PID_FILE"
    echo "$actual_port" > "$VITE_PORT_FILE"

    cd "$PROJECT_ROOT"

    # 等待并显示实际端口
    wait "$vite_pid" 2>/dev/null || true
}

# —— 等待 Vite 端口可达（最多 60s） ——
wait_for_port() {
    local port=$1
    local max=${2:-60}
    local i=0
    while [[ $i -lt $max ]]; do
        if lsof -ti:"$port" &>/dev/null; then
            return 0
        fi
        sleep 0.5
        ((i++))
    done
    return 1
}

# —— 启动应用 ——
launch_app() {
    echo -e "${CYAN}━━━ 启动 A4N ━━━${NC}"
    echo ""

    cd "$PROJECT_ROOT"

    case "$RUN_MODE" in
        dev)
            if $HOT_RELOAD; then
                launch_app_hot
            else
                launch_app_classic
            fi
            ;;
        release)
            echo -e "  模式: ${CYAN}Release 模式${NC}"
            echo ""
            # 确保已构建
            if [[ ! -f "$PROJECT_ROOT/target/release/a4n-tauri" ]]; then
                echo -e "${YELLOW}  未找到 Release 二进制，先执行构建...${NC}"
                echo -e "  ${CYAN}→${NC} 构建前端..."
                cd "$FRONTEND_DIR" && npm run build 2>&1
                cd "$PROJECT_ROOT"
                echo -e "  ${CYAN}→${NC} 构建后端..."
                cargo build --release 2>&1
            fi
            "$PROJECT_ROOT/target/release/a4n-tauri" 2>&1
            ;;
        server)
            echo -e "  模式: ${CYAN}后端服务（无 GUI 窗口）${NC}"
            echo -e "  ${YELLOW}提示: 通过环境变量 HEADLESS=1 启动 Tauri 无头模式${NC}"
            echo ""
            # 先构建前端（Tauri 需要 dist 目录）
            if [[ ! -d "$FRONTEND_DIR/dist" ]] || [[ ! -f "$FRONTEND_DIR/dist/index.html" ]]; then
                echo -e "${YELLOW}  前端未构建，正在构建...${NC}"
                cd "$FRONTEND_DIR" && npm run build 2>&1
                cd "$PROJECT_ROOT"
            fi
            HEADLESS=1 cargo run "${HOST_CARGO_RUN_ARGS[@]}" --bin a4n-tauri 2>&1
            ;;
    esac
}

# —— 非热重载模式（静态 dist + 单次 cargo run，不启动任何 watcher） ——
launch_app_classic() {
    echo -e "  模式: ${CYAN}静态前端 + 单次 Tauri 运行（完全无热更新）${NC}"
    echo -e "  ${YELLOW}提示: 启动时会构建一次 frontend/dist${NC}"
    echo -e "  ${YELLOW}提示: 前端或 Rust 代码改动后都必须 Ctrl+C 并重新启动${NC}"
    echo ""

    ensure_runtime_dirs

    # 默认配置的 beforeDevCommand 会启动 Vite。no-hot 配置显式清除 devUrl 和
    # beforeDevCommand，使 Tauri 只提供本次构建的 dist 静态文件。
    echo -e "  ${CYAN}→${NC} 构建静态前端资源..."
    (
        cd "$FRONTEND_DIR"
        npm run build
    )
    echo -e "  ${GREEN}✓${NC} 前端静态资源已构建"
    echo ""

    unset A4N_DISABLE_VITE_HMR VITE_PORT TAURI_DEV_URL
    local no_hot_config="$PROJECT_ROOT/crates/a4n-tauri/tauri.no-hot.conf.json"
    local tauri_command=(cargo tauri dev --no-watch --config "$no_hot_config")
    if [[ ${#TAURI_DEV_ARGS[@]} -gt 0 ]]; then
        tauri_command+=(-- "${TAURI_DEV_ARGS[@]}")
    fi
    echo -e "  ${CYAN}→${NC} 启动 Tauri 静态服务器（无 Vite、无 cargo-watch、无 Tauri watcher）..."
    "${tauri_command[@]}" 2>&1 &
    local tauri_pid=$!
    echo "$tauri_pid" > "$TAURI_PID_FILE"
    wait "$tauri_pid" 2>/dev/null || true
}

# —— 精细热重载模式（Vite 后台常驻 + cargo-watch 触发 Rust 重启） ——
launch_app_hot() {
    echo -e "  模式: ${BOLD}${CYAN}精细热重载${NC}"
    echo -e "  ${CYAN}•${NC} 前端：Vite HMR 不重启（session/state 保留）"
    echo -e "  ${CYAN}•${NC} 后端：cargo-watch 监视 crates/，Rust 改动后只重启二进制"
    echo ""

    # 1. 启动 Vite（后台），固定 5173 端口
    local actual_port
    actual_port=$(find_available_port "$DEFAULT_VITE_PORT")
    if [[ "$actual_port" != "$DEFAULT_VITE_PORT" ]]; then
        echo -e "  ${YELLOW}⚠${NC} 端口 ${DEFAULT_VITE_PORT} 被占用，使用端口 ${actual_port}"
    fi
    export VITE_PORT="$actual_port"
    export TAURI_DEV_URL="http://localhost:${actual_port}"

    echo -e "  ${CYAN}→${NC} 启动 Vite dev server（端口 ${actual_port}）..."
    ensure_runtime_dirs
    cd "$FRONTEND_DIR"
    VITE_PORT="$actual_port" npm run dev > "$VITE_LOG_FILE" 2>&1 &
    local vite_pid=$!
    echo "$vite_pid" > "$VITE_PID_FILE"
    echo "$actual_port" > "$VITE_PORT_FILE"
    cd "$PROJECT_ROOT"

    # 等待 Vite 起来
    if wait_for_port "$actual_port" 60; then
        echo -e "  ${GREEN}✓${NC} Vite 已就绪（PID: ${vite_pid}）"
        echo -e "    前端日志：${YELLOW}${VITE_LOG_FILE}${NC}"
    else
        echo -e "${RED}✗ Vite 60s 内未就绪，请查看日志 ${VITE_LOG_FILE}${NC}"
        cleanup
        exit 1
    fi

    # 2. cargo-watch 监视 crates/，每次改动重启 cargo run
    echo ""
    echo -e "  ${CYAN}→${NC} 启动 cargo-watch 监视 crates/ ..."
    echo -e "    ${YELLOW}首次编译需要几分钟，请耐心等待${NC}"
    echo ""
    echo -e "  ${BOLD}${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "  ${BOLD}修改 Rust 代码后会自动重新编译；修改前端代码会自动 HMR${NC}"
    echo -e "  ${BOLD}按 Ctrl+C 退出${NC}"
    echo -e "  ${BOLD}${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""

    # -w crates           只监视 crates/ 目录
    # -i a4n-migration    忽略迁移 crate（不需要热重载）
    # -i tests            忽略测试目录
    # -i target           保护：cargo-watch 默认忽略 target，但写明保险
    # --no-vcs-ignores    不读 .gitignore（避免 prompts 等被排除）
    # -s "..."            shell 命令：每次改动后执行
    cargo watch \
        -w crates \
        -i "**/tests/**" \
        -i "**/target/**" \
        -i "crates/a4n-migration/**" \
        -s "cargo run ${A4N_HOST_CARGO_RUN_ARGS:-} --bin a4n-tauri 2>&1" 2>&1 &
    local watch_pid=$!
    ensure_runtime_dirs
    echo "$watch_pid" > "$CARGO_WATCH_PID_FILE"

    # 等待 cargo-watch 退出（用户 Ctrl+C 时）
    wait "$watch_pid" 2>/dev/null || true
}

# —— 主流程 ——
echo -e "${BOLD}${CYAN}╔══════════════════════════════════╗${NC}"
echo -e "${BOLD}${CYAN}║     A4N 项目运行脚本             ║${NC}"
echo -e "${BOLD}${CYAN}╚══════════════════════════════════╝${NC}"
echo ""

configure_runtime_profile
configure_frontend_api_transport

if $CHECK_ONLY; then
    check_environment
    echo -e "${GREEN}${BOLD}环境检查完成${NC}"
    exit 0
fi

if $RUN_MIGRATE_ONLY && $REMOTE_API; then
    echo -e "${YELLOW}远程 API 模式不执行本地数据库迁移${NC}"
    exit 0
fi

# 注册退出清理（必须在任何长时间运行之前）
register_trap

# 启动前清理残留
preflight_cleanup

check_environment

configure_linux_input_method

if $FRONTEND_ONLY; then
    launch_frontend
    exit 0
fi

guard_build_cache

if $RUN_MIGRATE_ONLY; then
    run_migration
    echo -e "${GREEN}${BOLD}数据库迁移完成${NC}"
    exit 0
fi

if $REMOTE_API; then
    echo -e "${CYAN}→${NC} 远程 API 模式：跳过本地数据库迁移"
    echo ""
else
    run_migration
fi
start_dev_sidecars
launch_app
