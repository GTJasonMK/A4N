#!/usr/bin/env bash
#=============================================================================
# A4N 项目部署脚本
#
# 用法:
#   ./scripts/deploy.sh                 # 构建并打包（生成 tar.gz）
#   ./scripts/deploy.sh --target linux-x86_64  # 指定目标平台
#   ./scripts/deploy.sh --no-build      # 跳过构建（使用已有二进制）
#   ./scripts/deploy.sh --services export,video # 只分发选定 sidecar
#   ./scripts/deploy.sh --dry-run       # 预演（仅显示将打包的文件）
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
VERSION=$(grep '^version' "$PROJECT_ROOT/crates/host/a4n-tauri/Cargo.toml" | head -1 | sed 's/.*"\(.*\)"/\1/')
TOOL_SPECS=()
COMPONENT_SERVICE_FEATURES=()
SELECTED_SERVICES=()
SKIP_BUILD=false
DRY_RUN=false
TARGET_PLATFORM=""
SERVICES_FILTER=""

load_release_inventory() {
    local lines package binary service
    lines="$(release_tool_rows)"
    while IFS=$'\t' read -r package binary; do
        TOOL_SPECS+=("$package:$binary")
    done <<<"$lines"
    while IFS=$'\t' read -r service _; do
        SELECTED_SERVICES+=("$service")
    done <<<"$(sidecar_catalog_rows)"
}

select_sidecar_inventory() {
    [[ -z "$SERVICES_FILTER" ]] && return
    local requested service rows selected_services
    rows="$(sidecar_catalog_rows)"
    selected_services=()
    SELECTED_SERVICES=()
    IFS=',' read -ra requested <<<"$SERVICES_FILTER"
    for service in "${requested[@]}"; do
        service="$(echo "$service" | tr -d '[:space:]')"
        [[ -n "$service" ]] || die "--services 不能包含空 service"
        found_service=false
        while IFS=$'\t' read -r catalog_service _; do
            if [[ "$catalog_service" == "$service" ]]; then
                found_service=true
                break
            fi
        done <<<"$rows"
        $found_service || die "--services 包含未知 service: $service"
        [[ " ${selected_services[*]} " != *" $service "* ]] || die "--services 包含重复 service: $service"
        selected_services+=("$service")
        COMPONENT_SERVICE_FEATURES+=("service_${service//-/_}")
    done
    SELECTED_SERVICES=("${selected_services[@]}")
}

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
            [[ $# -ge 2 ]] || die "--target 缺少值"
            TARGET_PLATFORM="$2"
            shift 2
            ;;
        --no-build)
            SKIP_BUILD=true
            shift
            ;;
        --services)
            [[ $# -ge 2 ]] || die "--services 缺少值"
            SERVICES_FILTER="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
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
[[ "$TARGET_PLATFORM" =~ ^[a-z0-9][a-z0-9._-]*$ ]] || die "--target 只能包含小写字母、数字、点、下划线和连字符"

BINARY_EXT=""
if [[ "$TARGET_PLATFORM" == *windows* ]]; then
    BINARY_EXT=".exe"
fi

# 输出目录
DEPLOY_DIR="$PROJECT_ROOT/deploy"
RELEASE_BINARY="$CARGO_TARGET_DIR/release/${PACKAGE_NAME}${BINARY_EXT}"
FRONTEND_DIST="$FRONTEND_DIR/dist"
PACKAGE_DIR="$DEPLOY_DIR/${PACKAGE_NAME}-${VERSION}-${TARGET_PLATFORM}"
ARCHIVE_NAME="${PACKAGE_NAME}-${VERSION}-${TARGET_PLATFORM}.tar.gz"

release_binary_path() {
    local binary="$1"
    printf '%s/release/%s%s\n' "$CARGO_TARGET_DIR" "$binary" "$BINARY_EXT"
}

onnxruntime_release_name() {
    case "$TARGET_PLATFORM" in
        *windows*) printf '%s\n' "onnxruntime.dll" ;;
        *darwin*|macos-*) printf '%s\n' "libonnxruntime.dylib" ;;
        *) printf '%s\n' "libonnxruntime.so.1" ;;
    esac
}

onnxruntime_release_source() {
    printf '%s/lib/%s\n' "$A4N_ONNXRUNTIME_ROOT" "$(onnxruntime_release_name)"
}

echo -e "${BOLD}${CYAN}╔══════════════════════════════════╗${NC}"
echo -e "${BOLD}${CYAN}║     A4N 项目部署脚本            ║${NC}"
echo -e "${BOLD}${CYAN}╚══════════════════════════════════╝${NC}"
echo ""
echo -e "  版本:   ${CYAN}${VERSION}${NC}"
echo -e "  平台:   ${CYAN}${TARGET_PLATFORM}${NC}"
echo -e "  产物:   ${CYAN}${ARCHIVE_NAME}${NC}"
echo ""

# —— 前置检查 ——
check_prerequisites() {
    if ! $SKIP_BUILD; then
        require_command cargo
        require_command node
        require_command npm
        echo -e "${GREEN}✓ cargo $(cargo --version | cut -d' ' -f2)${NC}"
        echo -e "${GREEN}✓ node $(node --version)${NC}"
        echo -e "${GREEN}✓ npm $(npm --version)${NC}"
        require_onnxruntime
        echo -e "${GREEN}✓ ONNX Runtime $(onnxruntime_version) ($A4N_ONNXRUNTIME_ROOT)${NC}"
    else
        echo -e "${YELLOW}⚠ 跳过构建，前置检查不要求 cargo/node/npm${NC}"
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
        for spec in "${TOOL_SPECS[@]}"; do
            local package="${spec%%:*}" binary="${spec#*:}" tool_binary
            tool_binary="$(release_binary_path "$binary")"
            if [[ ! -f "$tool_binary" ]]; then
                echo -e "${RED}✗ 未找到工具 Release 二进制: ${tool_binary}${NC}"
                echo "  请先运行 cargo build --release -p ${package}"
                exit 1
            fi
        done
        if [[ ! -d "$FRONTEND_DIST" ]]; then
            echo -e "${RED}✗ 未找到前端产物: ${FRONTEND_DIST}${NC}"
            echo "  请先运行 ./scripts/build.sh --frontend"
            exit 1
        fi
        echo ""
        return
    fi

    ensure_frontend_dependencies

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
    cargo build --release -p "$PACKAGE_NAME" 2>&1
    local component_host_features
    if [[ ${#COMPONENT_SERVICE_FEATURES[@]} -gt 0 ]]; then
        component_host_features="stateful-workers,$(IFS=,; printf '%s' "${COMPONENT_SERVICE_FEATURES[*]}")"
        echo -e "${CYAN}→ 构建共享组件宿主（选定 services）${NC}"
        cargo build --release --no-default-features --features "$component_host_features" -p a4n-component-host 2>&1
    else
        echo -e "${CYAN}→ 构建共享组件宿主（全部角色）${NC}"
        cargo build --release -p a4n-component-host 2>&1
    fi
    for spec in "${TOOL_SPECS[@]}"; do
        local package="${spec%%:*}"
        [[ "$package" == "a4n-component-host" ]] && continue
        echo -e "${CYAN}→ 构建工具: ${package}${NC}"
        cargo build --release -p "$package" 2>&1
    done
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
    local release_manifest_schema expected_checksum_entries
    release_manifest_schema="$(release_manifest_schema_version)"
    echo -e "${CYAN}━━━ 准备打包 ━━━${NC}"

    # 清理旧产物
    rm -rf "$PACKAGE_DIR"
    mkdir -p "$PACKAGE_DIR/bin"
    mkdir -p "$PACKAGE_DIR/bin/services"
    mkdir -p "$PACKAGE_DIR/static"    # 前端静态文件
    mkdir -p "$PACKAGE_DIR/data"
    mkdir -p "$PACKAGE_DIR/prompts"

    # 复制二进制
    cp "$RELEASE_BINARY" "$PACKAGE_DIR/bin/"
    echo -e "  ${GREEN}✓${NC} 二进制: bin/${PACKAGE_NAME}"

    for spec in "${TOOL_SPECS[@]}"; do
        local tool_name="${spec#*:}" tool_binary
        tool_binary="$(release_binary_path "$tool_name")"
        [[ -f "$tool_binary" ]] || die "缺少工具二进制: $tool_binary"
        cp "$tool_binary" "$PACKAGE_DIR/bin/"
        echo -e "  ${GREEN}✓${NC} 工具: bin/$(basename "$tool_binary")"
    done

    require_onnxruntime
    local runtime_source runtime_name
    runtime_name="$(onnxruntime_release_name)"
    runtime_source="$(onnxruntime_release_source)"
    [[ -r "$runtime_source" ]] || die "缺少 ONNX Runtime 发布文件: $runtime_source"
    cp -L "$runtime_source" "$PACKAGE_DIR/bin/services/$runtime_name"
    echo -e "  ${GREEN}✓${NC} runtime: bin/services/${runtime_name}"

    local manifest_args=(
        generate
        --package-root "$PACKAGE_DIR"
        --target "$TARGET_PLATFORM"
        --package "$PACKAGE_NAME"
        --output "$PACKAGE_DIR/release-manifest.json"
    )
    if [[ -n "$SERVICES_FILTER" ]]; then
        manifest_args+=(--services "$SERVICES_FILTER")
    fi
    ruby "$PROJECT_ROOT/scripts/release_manifest.rb" "${manifest_args[@]}"
    ruby "$PROJECT_ROOT/scripts/release_manifest.rb" verify \
        --package-root "$PACKAGE_DIR" \
        --manifest "$PACKAGE_DIR/release-manifest.json"
    expected_checksum_entries="$(wc -l < "$PACKAGE_DIR/release-manifest.sha256")"
    echo -e "  ${GREEN}✓${NC} release-manifest.json（host + ${#SELECTED_SERVICES[@]} 个 service + ${#TOOL_SPECS[@]} 个工具）"

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
        cp -R "$PROJECT_ROOT/prompts/." "$PACKAGE_DIR/prompts/"
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
    if [[ "$TARGET_PLATFORM" == *windows* ]]; then
        cat > "$PACKAGE_DIR/start.bat" << 'BATEOF'
@echo off
set "SCRIPT_DIR=%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%start.ps1"
exit /b %ERRORLEVEL%
BATEOF
        printf '\xEF\xBB\xBF' > "$PACKAGE_DIR/start.ps1"
        printf '$ReleaseManifestSchemaVersion = %s\n\n' "$release_manifest_schema" >> "$PACKAGE_DIR/start.ps1"
        cat >> "$PACKAGE_DIR/start.ps1" << 'PSEOF'
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $ScriptDir

function Assert-ReleaseManifest {
    $manifestPath = Join-Path $ScriptDir "release-manifest.json"
    $checksumPath = Join-Path $ScriptDir "release-manifest.sha256"
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        throw "缺少 release-manifest.json"
    }
    if (-not (Test-Path -LiteralPath $checksumPath -PathType Leaf)) {
        throw "缺少 release-manifest.sha256"
    }
    $manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
    if ($manifest.schema_version -ne $ReleaseManifestSchemaVersion) {
        throw "release manifest schema 不兼容"
    }
    $seen = @{}
    $manifestEntries = @($manifest.binaries) + @($manifest.tools) + @($manifest.runtime_files)
    if ($manifestEntries.Count -eq 0) {
        throw "release manifest 没有文件条目"
    }
    foreach ($entry in $manifestEntries) {
        $relative = [string]$entry.path
        $expectedHash = [string]$entry.sha256
        if ([string]::IsNullOrWhiteSpace($relative) -or
            $expectedHash -notmatch '^[0-9a-fA-F]{64}$' -or
            [IO.Path]::IsPathRooted($relative) -or
            $relative -match '^[A-Za-z]:[\\/]' -or
            ($relative -split "[/\\]") -contains "..") {
            throw "release manifest 文件条目无效: $relative"
        }
        $binary = Join-Path $ScriptDir ($relative -replace '/', [IO.Path]::DirectorySeparatorChar)
        if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
            throw "release manifest 缺少二进制: $relative"
        }
        if ($seen.ContainsKey($relative)) {
            if ($seen[$relative] -ne $expectedHash.ToLowerInvariant()) {
                throw "release manifest 共享路径的 SHA-256 不一致: $relative"
            }
            continue
        }
        $seen[$relative] = $expectedHash.ToLowerInvariant()
        $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $binary).Hash.ToLowerInvariant()
        if ($actual -ne $expectedHash.ToLowerInvariant()) {
            throw "release manifest SHA-256 不匹配: $relative"
        }
    }
    $checksumEntries = @{}
    $checksumLines = @(Get-Content -LiteralPath $checksumPath)
    if ($checksumLines.Count -eq 0) {
        throw "release manifest checksum 为空"
    }
    foreach ($line in $checksumLines) {
        if ([string]::IsNullOrWhiteSpace($line) -or $line -notmatch '^(?<hash>[0-9a-fA-F]{64})  (?<path>\S+)$') {
            throw "release manifest checksum 格式无效"
        }
        $hash = $Matches.hash.ToLowerInvariant()
        $relative = $Matches.path
        if ([IO.Path]::IsPathRooted($relative) -or
            $relative -match '^[A-Za-z]:[\\/]' -or
            ($relative -split "[/\\]") -contains ".." -or
            $relative -match '\\') {
            throw "release manifest checksum 路径越界: $relative"
        }
        if ($checksumEntries.ContainsKey($relative)) {
            throw "release manifest checksum 存在重复路径: $relative"
        }
        $checksumEntries[$relative] = $hash
        $binary = Join-Path $ScriptDir ($relative -replace '/', [IO.Path]::DirectorySeparatorChar)
        if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
            throw "release manifest checksum 缺少文件: $relative"
        }
        $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $binary).Hash.ToLowerInvariant()
        if ($actual -ne $hash) {
            throw "release manifest checksum 不匹配: $relative"
        }
    }
    $seen["release-manifest.json"] = $null
    if ($checksumEntries.Count -ne $seen.Count) {
        throw "release manifest checksum 条目数量不匹配"
    }
    foreach ($relative in $seen.Keys) {
        if (-not $checksumEntries.ContainsKey($relative)) {
            throw "release manifest checksum 缺少条目: $relative"
        }
        if ($null -ne $seen[$relative] -and $checksumEntries[$relative] -ne $seen[$relative]) {
            throw "release manifest checksum 与 manifest 不一致: $relative"
        }
    }
}

Assert-ReleaseManifest

function Set-DefaultEnv {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Value
    )

    if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($Name, "Process"))) {
        [Environment]::SetEnvironmentVariable($Name, $Value, "Process")
    }
}

Set-DefaultEnv "A4N_PORTABLE_HOME" $ScriptDir
Set-DefaultEnv "A4N_DATA_DIR" (Join-Path $env:A4N_PORTABLE_HOME "data")
Set-DefaultEnv "A4N_CACHE_DIR" (Join-Path $env:A4N_DATA_DIR "cache")
Set-DefaultEnv "A4N_LOG_DIR" (Join-Path $env:A4N_DATA_DIR "logs")
Set-DefaultEnv "A4N_TEMP_DIR" (Join-Path $env:A4N_DATA_DIR "tmp")
Set-DefaultEnv "A4N_RUNTIME_PROFILE" "sidecar"
Set-DefaultEnv "A4N_RELEASE_MANIFEST_PATH" (Join-Path $ScriptDir "release-manifest.json")
Set-DefaultEnv "A4N_SIDECAR_BIN_DIR" (Join-Path $ScriptDir "bin")
Set-DefaultEnv "A4N_ONNXRUNTIME_ROOT" (Join-Path $ScriptDir "bin\services")
Set-DefaultEnv "A4N_MANAGED_SIDECARS" "1"
$env:PATH = (Join-Path $ScriptDir "bin\services") + ";" + $env:PATH

& (Join-Path $ScriptDir "bin\a4n-tauri.exe")
exit $LASTEXITCODE
PSEOF
        echo -e "  ${GREEN}✓${NC} 启动脚本: start.bat / start.ps1"
    else
        printf '%s\n' '#!/usr/bin/env bash' "EXPECTED_RELEASE_CHECKSUM_ENTRIES=$expected_checksum_entries" > "$PACKAGE_DIR/start.sh"
        cat >> "$PACKAGE_DIR/start.sh" << 'SHEOF'
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

verify_release_manifest() {
    local checksum_file="$SCRIPT_DIR/release-manifest.sha256"
    local line expected relative count=0
    declare -A seen=()
    [[ -r "$SCRIPT_DIR/release-manifest.json" ]] || {
        echo "缺少 release-manifest.json" >&2
        return 1
    }
    [[ -r "$checksum_file" ]] || {
        echo "缺少 release-manifest.sha256" >&2
        return 1
    }
    while IFS= read -r line || [[ -n "$line" ]]; do
        [[ "$line" =~ ^([[:xdigit:]]{64})[[:space:]][[:space:]]([^[:space:]]+)$ ]] || {
            echo "release manifest checksum 格式无效" >&2
            return 1
        }
        expected="${BASH_REMATCH[1],,}"
        relative="${BASH_REMATCH[2]}"
        case "/$relative/" in
            *"/../"*|//*|*\\*)
                echo "release manifest 路径越界: $relative" >&2
                return 1
                ;;
        esac
        [[ -z "${seen[$relative]+x}" ]] || {
            echo "release manifest checksum 存在重复路径: $relative" >&2
            return 1
        }
        seen["$relative"]=1
        [[ -f "$SCRIPT_DIR/$relative" ]] || {
            echo "release manifest 缺少二进制: $relative" >&2
            return 1
        }
        if command -v sha256sum >/dev/null 2>&1; then
            printf '%s  %s\n' "$expected" "$relative" | (cd "$SCRIPT_DIR" && sha256sum -c -) >/dev/null || return 1
        elif command -v shasum >/dev/null 2>&1; then
            actual="$(shasum -a 256 "$SCRIPT_DIR/$relative" | awk '{print $1}')"
            [[ "${actual,,}" == "$expected" ]] || return 1
        else
            echo "缺少 sha256sum 或 shasum，拒绝启动" >&2
            return 1
        fi
        count=$((count + 1))
    done <"$checksum_file"
    [[ "$count" -eq "$EXPECTED_RELEASE_CHECKSUM_ENTRIES" ]] || {
        echo "release manifest checksum 条目数量不匹配" >&2
        return 1
    }
    [[ -n "${seen[release-manifest.json]+x}" ]] || {
        echo "release manifest checksum 缺少 release-manifest.json" >&2
        return 1
    }
}

verify_release_manifest

export A4N_PORTABLE_HOME="${A4N_PORTABLE_HOME:-$SCRIPT_DIR}"
export A4N_DATA_DIR="${A4N_DATA_DIR:-$A4N_PORTABLE_HOME/data}"
export A4N_CACHE_DIR="${A4N_CACHE_DIR:-$A4N_DATA_DIR/cache}"
export A4N_LOG_DIR="${A4N_LOG_DIR:-$A4N_DATA_DIR/logs}"
export A4N_TEMP_DIR="${A4N_TEMP_DIR:-$A4N_DATA_DIR/tmp}"
export A4N_RUNTIME_PROFILE="${A4N_RUNTIME_PROFILE:-sidecar}"
export A4N_RELEASE_MANIFEST_PATH="${A4N_RELEASE_MANIFEST_PATH:-$SCRIPT_DIR/release-manifest.json}"
export A4N_SIDECAR_BIN_DIR="${A4N_SIDECAR_BIN_DIR:-$SCRIPT_DIR/bin}"
export A4N_ONNXRUNTIME_ROOT="${A4N_ONNXRUNTIME_ROOT:-$SCRIPT_DIR/bin/services}"
export A4N_MANAGED_SIDECARS="${A4N_MANAGED_SIDECARS:-1}"
export LD_LIBRARY_PATH="$SCRIPT_DIR/bin/services${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export DYLD_LIBRARY_PATH="$SCRIPT_DIR/bin/services${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"

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

默认运行模式:
  启动脚本启用 managed sidecars，由桌面 Host 使用 bin/a4n-component-host
  为每个 service 启动独立进程。bin/services/ 保存 ONNX Runtime。

首次启动前必须在 data/config.toml 填写 database_url，并配置 JWT secret：
  database_url = "postgres://user:password@host:5432/database"
  [security]
  jwt_secret = "replace-with-a-long-random-secret"

目录结构:
  bin/         桌面 Host 与 release tool catalog 声明的工具
  bin/a4n-component-host worker/service launcher，bin/services/ 保存 ONNX Runtime
  static/      前端页面（React SPA）
  prompts/     提示词模板
  data/        数据目录、配置、数据库、日志、缓存、生成素材

系统要求:
  - Linux: webkit2gtk-4.1, gtk3
  - macOS: 无额外依赖
  - Windows: WebView2 运行时（通常已预装）
  - ONNX Runtime 由发布包携带，不依赖开发机安装目录
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

# —— 主流程 ——
load_release_inventory
select_sidecar_inventory
check_prerequisites

if $DRY_RUN; then
    echo -e "${CYAN}━━━ 预演模式（不执行实际操作）━━━${NC}"
    echo ""
    echo "将打包以下内容:"
    echo "  - bin/${PACKAGE_NAME}"
    for spec in "${TOOL_SPECS[@]}"; do
        if [[ "${spec#*:}" != "a4n-component-host" ]]; then
            echo "  - bin/${spec#*:}${BINARY_EXT}"
        fi
    done
    echo "  - bin/a4n-component-host (worker/service shared launcher)"
    echo "  - bin/services/ (ONNX Runtime)"
    echo "  - start.sh / start.bat / Windows start.ps1 (默认 sidecar profile，启动包内 sidecar)"
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
    echo "  - README.txt"
    echo ""
    echo "产物: ${DEPLOY_DIR}/${ARCHIVE_NAME}"
    exit 0
fi

do_build
prepare_package
create_archive
