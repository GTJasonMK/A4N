#!/usr/bin/env bash
#=============================================================================
# A4N 项目部署脚本
#
# 用法:
#   ./scripts/deploy.sh                 # 构建并打包（生成 tar.gz）
#   ./scripts/deploy.sh --target linux  # 指定目标平台
#   ./scripts/deploy.sh --no-build      # 跳过构建（使用已有二进制）
#   ./scripts/deploy.sh --dry-run       # 预演（仅显示将打包的文件）
#   ./scripts/deploy.sh --install       # 安装到系统路径
#   ./scripts/deploy.sh --help          # 显示帮助
#
# 产物: deploy/a4n-{version}-{platform}.tar.gz
#
# 跨平台：Linux / macOS / Windows(Git Bash / WSL)
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# —— 配置 ——
FRONTEND_DIR="$PROJECT_ROOT/frontend"
PACKAGE_NAME="a4n-tauri"
VERSION=$(grep '^version' "$PROJECT_ROOT/crates/a4n-tauri/Cargo.toml" | head -1 | sed 's/.*"\(.*\)"/\1/')
SKIP_BUILD=false
DRY_RUN=false
DO_INSTALL=false
TARGET_PLATFORM=""

# —— 检测平台 ——
detect_platform() {
    local os arch
    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)

    case "$arch" in
        x86_64|amd64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *) ;;
    esac

    case "$os" in
        linux)   os="linux" ;;
        darwin)  os="macos" ;;
        mingw*|msys*|cygwin*) os="windows"; arch="x86_64" ;;
        *)       os="unknown" ;;
    esac

    echo "${os}-${arch}"
}

# —— 解析参数 ——
while [[ $# -gt 0 ]]; do
    case "$1" in
        --target)
            TARGET_PLATFORM="$2"
            shift 2
            ;;
        --no-build)
            SKIP_BUILD=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --install|-i)
            DO_INSTALL=true
            shift
            ;;
        --help|-h)
            print_script_usage "$0" 15
            exit 0
            ;;
        *)
            die "未知参数: $1"
            ;;
    esac
done

# 确定目标平台
if [[ -z "$TARGET_PLATFORM" ]]; then
    TARGET_PLATFORM=$(detect_platform)
fi

# 输出目录
DEPLOY_DIR="$PROJECT_ROOT/deploy"
RELEASE_BINARY="$PROJECT_ROOT/target/release/$PACKAGE_NAME"
FRONTEND_DIST="$FRONTEND_DIR/dist"
PACKAGE_DIR="$DEPLOY_DIR/${PACKAGE_NAME}-${VERSION}-${TARGET_PLATFORM}"
ARCHIVE_NAME="${PACKAGE_NAME}-${VERSION}-${TARGET_PLATFORM}.tar.gz"

echo -e "${BOLD}${CYAN}╔══════════════════════════════════╗${NC}"
echo -e "${BOLD}${CYAN}║     A4N 项目部署脚本            ║${NC}"
echo -e "${BOLD}${CYAN}╚══════════════════════════════════╝${NC}"
echo ""
echo -e "  版本:   ${CYAN}${VERSION}${NC}"
echo -e "  平台:   ${CYAN}${TARGET_PLATFORM}${NC}"
echo -e "  产物:   ${CYAN}${ARCHIVE_NAME}${NC}"
echo ""

# —— 检查 Node.js ——
check_node() {
    require_command node
    require_command npm
}

# —— 前置检查 ——
check_prerequisites() {
    if ! $SKIP_BUILD; then
        require_command cargo
        check_node
        echo -e "${GREEN}✓ cargo $(cargo --version | cut -d' ' -f2)${NC}"
        echo -e "${GREEN}✓ node $(node --version)${NC}"
        echo -e "${GREEN}✓ npm $(npm --version)${NC}"
    else
        echo -e "${YELLOW}⚠ 跳过构建，前置检查不要求 cargo/node/npm${NC}"
    fi
}

# —— 安装前端依赖 ——
install_frontend_deps() {
    if [[ ! -d "$FRONTEND_DIR/node_modules" ]]; then
        echo -e "${YELLOW}⚠ 前端依赖未安装，正在安装...${NC}"
        cd "$FRONTEND_DIR"
        npm install --silent 2>&1
        cd "$PROJECT_ROOT"
        echo -e "${GREEN}✓ 前端依赖安装完成${NC}"
    fi
}

# —— 构建 Release ——
do_build() {
    if $SKIP_BUILD; then
        echo -e "${YELLOW}⚠ 跳过构建（--no-build）${NC}"
        if [[ ! -f "$RELEASE_BINARY" ]]; then
            echo -e "${RED}✗ 未找到 Release 二进制: ${RELEASE_BINARY}${NC}"
            echo "  请先运行 ./scripts/build.sh --release"
            exit 1
        fi
        if [[ ! -d "$FRONTEND_DIST" ]]; then
            echo -e "${RED}✗ 未找到前端产物: ${FRONTEND_DIST}${NC}"
            echo "  请先运行 ./scripts/build.sh --frontend"
            exit 1
        fi
        echo ""
        return
    fi

    # 安装前端依赖
    install_frontend_deps

    # 构建前端
    echo -e "${CYAN}━━━ 构建前端 ━━━${NC}"
    cd "$FRONTEND_DIR"
    npm run build 2>&1
    cd "$PROJECT_ROOT"
    echo -e "${GREEN}✓ 前端构建完成${NC}"
    echo ""

    # 构建后端
    echo -e "${CYAN}━━━ 构建 Release ━━━${NC}"
    cd "$PROJECT_ROOT"
    cargo build --release 2>&1
    echo ""

    if [[ ! -f "$RELEASE_BINARY" ]]; then
        echo -e "${RED}✗ 构建失败，未生成二进制${NC}"
        exit 1
    fi

    local size
    size=$(du -h "$RELEASE_BINARY" | cut -f1)
    echo -e "${GREEN}✓ 二进制已生成 (${size})${NC}"
    echo ""
}

# —— 准备打包目录 ——
prepare_package() {
    echo -e "${CYAN}━━━ 准备打包 ━━━${NC}"

    # 清理旧产物
    rm -rf "$PACKAGE_DIR"
    mkdir -p "$PACKAGE_DIR/bin"
    mkdir -p "$PACKAGE_DIR/static"    # 前端静态文件
    mkdir -p "$PACKAGE_DIR/data"
    mkdir -p "$PACKAGE_DIR/prompts"

    # 复制二进制
    cp "$RELEASE_BINARY" "$PACKAGE_DIR/bin/"
    echo -e "  ${GREEN}✓${NC} 二进制: bin/${PACKAGE_NAME}"

    # 复制前端产物
    if [[ -d "$FRONTEND_DIST" ]]; then
        cp -r "$FRONTEND_DIST"/* "$PACKAGE_DIR/static/"
        local static_count
        static_count=$(find "$PACKAGE_DIR/static" -type f | wc -l)
        echo -e "  ${GREEN}✓${NC} 前端静态文件: ${static_count} 个文件"
    else
        echo -e "  ${YELLOW}⚠${NC} 未找到前端产物，跳过"
    fi

    # 复制提示词模板
    if [[ -d "$PROJECT_ROOT/prompts" ]]; then
        cp -r "$PROJECT_ROOT/prompts/"* "$PACKAGE_DIR/prompts/" 2>/dev/null || true
        local prompt_count
        prompt_count=$(find "$PACKAGE_DIR/prompts" -type f | wc -l)
        echo -e "  ${GREEN}✓${NC} 提示词模板: ${prompt_count} 个文件"
    fi

    # 生成便携式配置模板；不要复制开发态 data/config.toml，
    # 避免把开发机绝对路径、本地数据库连接或密钥写进发布包。
    cat > "$PACKAGE_DIR/data/config.toml" << 'CONFIGEOF'
# A4N 配置文件
# 程序会把这些相对路径按当前应用目录解析。
database_url = ""
data_dir = "data"
cache_dir = "data/cache"
temp_dir = "data/tmp"
log_dir = "data/logs"
CONFIGEOF
    echo -e "  ${CYAN}→${NC} 已生成便携式配置模板: data/config.toml"

    # 生成启动脚本
    if [[ "$TARGET_PLATFORM" == windows-* ]]; then
        cat > "$PACKAGE_DIR/start.bat" << 'BATEOF'
@echo off
set "SCRIPT_DIR=%~dp0"
cd /d "%SCRIPT_DIR%"
if not defined A4N_PORTABLE_HOME set "A4N_PORTABLE_HOME=%SCRIPT_DIR%"
echo 正在启动 A4N...
bin\a4n-tauri.exe
BATEOF
        echo -e "  ${GREEN}✓${NC} 启动脚本: start.bat"
    else
        cat > "$PACKAGE_DIR/start.sh" << 'SHEOF'
#!/usr/bin/env bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"
export A4N_PORTABLE_HOME="${A4N_PORTABLE_HOME:-$SCRIPT_DIR}"
echo "正在启动 A4N..."
exec bin/a4n-tauri
SHEOF
        chmod +x "$PACKAGE_DIR/start.sh"
        echo -e "  ${GREEN}✓${NC} 启动脚本: start.sh"
    fi

    # 生成 README
    cat > "$PACKAGE_DIR/README.txt" << EOF
A4N - AI 辅助小说创作工具
━━━━━━━━━━━━━━━━━━━━━━━━
版本: ${VERSION}
平台: ${TARGET_PLATFORM}

使用方法:
  Linux/macOS:  ./start.sh
  Windows:      双击 start.bat

首次启动前必须在 data/config.toml 填写 database_url，并配置 JWT secret：
  database_url = "postgres://user:password@host:5432/database"
  [security]
  jwt_secret = "replace-with-a-long-random-secret"

目录结构:
  bin/         可执行文件
  static/      前端页面（React SPA）
  prompts/     提示词模板（46 个）
  data/        数据目录、配置、数据库、日志、缓存、生成素材

系统要求:
  - Linux: webkit2gtk-4.1, gtk3
  - macOS: 无额外依赖
  - Windows: WebView2 运行时（通常已预装）
EOF
    echo -e "  ${GREEN}✓${NC} README"

    echo ""
}

# —— 打包 ——
create_archive() {
    echo -e "${CYAN}━━━ 打包 ━━━${NC}"

    mkdir -p "$DEPLOY_DIR"
    cd "$DEPLOY_DIR"

    tar czf "$ARCHIVE_NAME" "$(basename "$PACKAGE_DIR")" 2>&1
    local archive_size
    archive_size=$(du -h "$ARCHIVE_NAME" | cut -f1)
    echo -e "  ${GREEN}✓${NC} ${DEPLOY_DIR}/${ARCHIVE_NAME}"
    echo -e "  大小: ${CYAN}${archive_size}${NC}"

    # 生成校验和
    if command -v sha256sum &>/dev/null; then
        sha256sum "$ARCHIVE_NAME" > "${ARCHIVE_NAME}.sha256"
        echo -e "  ${GREEN}✓${NC} SHA256: ${ARCHIVE_NAME}.sha256"
    elif command -v shasum &>/dev/null; then
        shasum -a 256 "$ARCHIVE_NAME" > "${ARCHIVE_NAME}.sha256"
        echo -e "  ${GREEN}✓${NC} SHA256: ${ARCHIVE_NAME}.sha256"
    fi

    # 清理临时目录
    rm -rf "$PACKAGE_DIR"

    echo ""
    echo -e "${GREEN}${BOLD}━━━ 部署包已生成 ━━━${NC}"
    echo -e "  ${DEPLOY_DIR}/${ARCHIVE_NAME}"
    echo ""
}

# —— 安装到系统 ——
install_to_system() {
    local install_dir="/usr/local"
    if [[ "$(uname -s)" == "Darwin" ]]; then
        install_dir="/usr/local"
    fi

    echo -e "${CYAN}━━━ 安装到系统 ━━━${NC}"
    echo -e "  目标: ${install_dir}/bin/${PACKAGE_NAME}"

    if [[ "$(id -u)" -ne 0 ]]; then
        echo -e "${YELLOW}⚠ 需要 sudo 权限${NC}"
        sudo cp "$RELEASE_BINARY" "${install_dir}/bin/"
        sudo mkdir -p "${install_dir}/share/a4n/static"
        sudo mkdir -p "${install_dir}/share/a4n/prompts"
        sudo cp -r "$FRONTEND_DIST"/* "${install_dir}/share/a4n/static/" 2>/dev/null || true
        sudo cp -r "$PROJECT_ROOT/prompts/"* "${install_dir}/share/a4n/prompts/" 2>/dev/null || true
    else
        cp "$RELEASE_BINARY" "${install_dir}/bin/"
        mkdir -p "${install_dir}/share/a4n/static"
        mkdir -p "${install_dir}/share/a4n/prompts"
        cp -r "$FRONTEND_DIST"/* "${install_dir}/share/a4n/static/" 2>/dev/null || true
        cp -r "$PROJECT_ROOT/prompts/"* "${install_dir}/share/a4n/prompts/" 2>/dev/null || true
    fi

    echo -e "${GREEN}✓ 安装完成${NC}"
    echo -e "  运行: ${CYAN}${PACKAGE_NAME}${NC}"
    echo ""
}

# —— 主流程 ——
check_prerequisites

if $DRY_RUN; then
    echo -e "${CYAN}━━━ 预演模式（不执行实际操作）━━━${NC}"
    echo ""
    echo "将打包以下内容:"
    echo "  - bin/${PACKAGE_NAME}"
    if [[ -d "$FRONTEND_DIST" ]]; then
        static_count=0
        static_count=$(find "$FRONTEND_DIST" -type f | wc -l)
        echo "  - static/ (${static_count} 个前端文件)"
    fi
    if [[ -d "$PROJECT_ROOT/prompts" ]]; then
        count=0
        count=$(find "$PROJECT_ROOT/prompts" -type f | wc -l)
        echo "  - prompts/ (${count} 个文件)"
    fi
    echo "  - data/config.toml"
    echo "  - start.sh / start.bat"
    echo "  - README.txt"
    echo ""
    echo "产物: ${DEPLOY_DIR}/${ARCHIVE_NAME}"
    exit 0
fi

if $DO_INSTALL; then
    do_build
    install_to_system
    exit 0
fi

do_build
prepare_package
create_archive
