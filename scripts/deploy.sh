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
SERVICE_CATALOG="$PROJECT_ROOT/crates/a4n-service-protocol/service-catalog.json"
SIDECAR_PACKAGES=(
    "a4n-prompt-service"
    "a4n-llm-gateway"
    "a4n-embedding-service"
    "a4n-rag-service"
    "a4n-image-service"
    "a4n-video-service"
    "a4n-import-service"
    "a4n-export-service"
)
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

BINARY_EXT=""
if [[ "$TARGET_PLATFORM" == windows-* ]]; then
    BINARY_EXT=".exe"
fi

# 输出目录
DEPLOY_DIR="$PROJECT_ROOT/deploy"
RELEASE_BINARY="$PROJECT_ROOT/target/release/${PACKAGE_NAME}${BINARY_EXT}"
FRONTEND_DIST="$FRONTEND_DIR/dist"
PACKAGE_DIR="$DEPLOY_DIR/${PACKAGE_NAME}-${VERSION}-${TARGET_PLATFORM}"
ARCHIVE_NAME="${PACKAGE_NAME}-${VERSION}-${TARGET_PLATFORM}.tar.gz"

release_binary_path() {
    local package="$1"
    printf '%s/target/release/%s%s\n' "$PROJECT_ROOT" "$package" "$BINARY_EXT"
}

sidecar_module_name() {
    case "$1" in
        a4n-prompt-service) printf '%s\n' "prompt" ;;
        a4n-llm-gateway) printf '%s\n' "llm" ;;
        a4n-embedding-service) printf '%s\n' "embedding" ;;
        a4n-rag-service) printf '%s\n' "rag" ;;
        a4n-image-service) printf '%s\n' "image" ;;
        a4n-video-service) printf '%s\n' "video" ;;
        a4n-import-service) printf '%s\n' "import" ;;
        a4n-export-service) printf '%s\n' "export" ;;
        *) printf '%s\n' "$1" ;;
    esac
}

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
        require_onnxruntime
        echo -e "${GREEN}✓ ONNX Runtime 1.24.2 ($A4N_ONNXRUNTIME_ROOT)${NC}"
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
        for package in "${SIDECAR_PACKAGES[@]}"; do
            local sidecar_binary
            sidecar_binary="$(release_binary_path "$package")"
            if [[ ! -f "$sidecar_binary" ]]; then
                echo -e "${RED}✗ 未找到 sidecar Release 二进制: ${sidecar_binary}${NC}"
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
    cargo build --release -p "$PACKAGE_NAME" 2>&1
    for package in "${SIDECAR_PACKAGES[@]}"; do
        echo -e "${CYAN}→ 构建 sidecar: ${package}${NC}"
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

    for package in "${SIDECAR_PACKAGES[@]}"; do
        local sidecar_binary module
        sidecar_binary="$(release_binary_path "$package")"
        module="$(sidecar_module_name "$package")"
        if [[ ! -f "$sidecar_binary" ]]; then
            echo -e "  ${RED}✗${NC} 缺少 sidecar 二进制: ${sidecar_binary}"
            exit 1
        fi
        cp "$sidecar_binary" "$PACKAGE_DIR/bin/services/"
        echo -e "  ${GREEN}✓${NC} sidecar: bin/services/$(basename "$sidecar_binary") (${module})"
    done

    require_onnxruntime
    local runtime_source runtime_name
    case "$TARGET_PLATFORM" in
        windows-*) runtime_source="$A4N_ONNXRUNTIME_ROOT/lib/onnxruntime.dll"; runtime_name="onnxruntime.dll" ;;
        macos-*) runtime_source="$A4N_ONNXRUNTIME_ROOT/lib/libonnxruntime.dylib"; runtime_name="libonnxruntime.dylib" ;;
        *) runtime_source="$A4N_ONNXRUNTIME_ROOT/lib/libonnxruntime.so.1"; runtime_name="libonnxruntime.so.1" ;;
    esac
    [[ -r "$runtime_source" ]] || die "缺少 ONNX Runtime 发布文件: $runtime_source"
    cp -L "$runtime_source" "$PACKAGE_DIR/bin/services/$runtime_name"
    echo -e "  ${GREEN}✓${NC} runtime: bin/services/${runtime_name}"

    require_command ruby
    ruby "$PROJECT_ROOT/scripts/release_manifest.rb" generate \
        --package-root "$PACKAGE_DIR" \
        --target "$TARGET_PLATFORM" \
        --package "$PACKAGE_NAME" \
        --host "$PACKAGE_NAME" \
        --catalog "$SERVICE_CATALOG" \
        --output "$PACKAGE_DIR/release-manifest.json"
    echo -e "  ${GREEN}✓${NC} release-manifest.json（host + ${#SIDECAR_PACKAGES[@]} 个 sidecar）"

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
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%start.ps1"
exit /b %ERRORLEVEL%
BATEOF
        printf '\xEF\xBB\xBF' > "$PACKAGE_DIR/start.ps1"
        cat >> "$PACKAGE_DIR/start.ps1" << 'PSEOF'
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $ScriptDir

function Assert-ReleaseManifest {
    $manifestPath = Join-Path $ScriptDir "release-manifest.json"
    if (-not (Test-Path $manifestPath)) {
        throw "缺少 release-manifest.json"
    }
    $manifest = Get-Content -Raw -Path $manifestPath | ConvertFrom-Json
    if ($manifest.schema_version -ne 2 -or $manifest.binaries.Count -ne 9 -or $manifest.runtime_files.Count -ne 1) {
        throw "release manifest 必须包含 schema v2 的 host + 8 个 sidecar + runtime"
    }
    $seen = @{}
    foreach ($entry in @($manifest.binaries) + @($manifest.runtime_files)) {
        if ($seen.ContainsKey($entry.path)) {
            throw "release manifest 存在重复路径: $($entry.path)"
        }
        $seen[$entry.path] = $true
        if ([IO.Path]::IsPathRooted($entry.path) -or $entry.path -match '(^|[\\/])\.\.([\\/]|$)') {
            throw "release manifest 路径越界: $($entry.path)"
        }
        $relative = $entry.path -replace '/', [IO.Path]::DirectorySeparatorChar
        $binary = Join-Path $ScriptDir $relative
        if (-not (Test-Path $binary)) {
            throw "release manifest 缺少二进制: $relative"
        }
        $actual = (Get-FileHash -Algorithm SHA256 -Path $binary).Hash.ToLowerInvariant()
        if ($actual -ne $entry.sha256.ToLowerInvariant()) {
            throw "release manifest SHA-256 不匹配: $($entry.path)"
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
Set-DefaultEnv "A4N_SIDECAR_BIN_DIR" (Join-Path $ScriptDir "bin\services")
Set-DefaultEnv "A4N_PROMPT_SERVICE_PROMPTS_DIR" (Join-Path $ScriptDir "prompts")
Set-DefaultEnv "A4N_RAG_SERVICE_DATA_DIR" $env:A4N_DATA_DIR
Set-DefaultEnv "A4N_EMBEDDING_CACHE_DIR" (Join-Path $env:A4N_CACHE_DIR "embedding-models")

Set-DefaultEnv "A4N_PROMPT_SERVICE_ADDR" "127.0.0.1:47641"
Set-DefaultEnv "A4N_LLM_GATEWAY_ADDR" "127.0.0.1:47642"
Set-DefaultEnv "A4N_EMBEDDING_SERVICE_ADDR" "127.0.0.1:47643"
Set-DefaultEnv "A4N_RAG_SERVICE_ADDR" "127.0.0.1:47644"
Set-DefaultEnv "A4N_IMAGE_SERVICE_ADDR" "127.0.0.1:47645"
Set-DefaultEnv "A4N_VIDEO_SERVICE_ADDR" "127.0.0.1:47646"
Set-DefaultEnv "A4N_IMPORT_SERVICE_ADDR" "127.0.0.1:47647"
Set-DefaultEnv "A4N_EXPORT_SERVICE_ADDR" "127.0.0.1:47648"

Set-DefaultEnv "A4N_PROMPT_SERVICE_URL" "http://$env:A4N_PROMPT_SERVICE_ADDR"
Set-DefaultEnv "A4N_LLM_GATEWAY_URL" "http://$env:A4N_LLM_GATEWAY_ADDR"
Set-DefaultEnv "A4N_EMBEDDING_SERVICE_URL" "http://$env:A4N_EMBEDDING_SERVICE_ADDR"
Set-DefaultEnv "A4N_RAG_SERVICE_URL" "http://$env:A4N_RAG_SERVICE_ADDR"
Set-DefaultEnv "A4N_IMAGE_SERVICE_URL" "http://$env:A4N_IMAGE_SERVICE_ADDR"
Set-DefaultEnv "A4N_VIDEO_SERVICE_URL" "http://$env:A4N_VIDEO_SERVICE_ADDR"
Set-DefaultEnv "A4N_IMPORT_SERVICE_URL" "http://$env:A4N_IMPORT_SERVICE_ADDR"
Set-DefaultEnv "A4N_EXPORT_SERVICE_URL" "http://$env:A4N_EXPORT_SERVICE_ADDR"

$ServicePidDir = Join-Path $env:A4N_DATA_DIR "services"
$ServiceLogDir = Join-Path $env:A4N_LOG_DIR "services"
New-Item -ItemType Directory -Force -Path $ServicePidDir, $ServiceLogDir, $env:A4N_CACHE_DIR, $env:A4N_TEMP_DIR | Out-Null

$tokenFile = Join-Path $ServicePidDir "auth-token"
if ([string]::IsNullOrWhiteSpace($env:A4N_SIDECAR_AUTH_TOKEN)) {
    if (Test-Path $tokenFile) {
        $env:A4N_SIDECAR_AUTH_TOKEN = (Get-Content -Raw -Path $tokenFile).Trim()
    } else {
        $bytes = New-Object byte[] 32
        $rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
        try {
            $rng.GetBytes($bytes)
        } finally {
            $rng.Dispose()
        }
        $env:A4N_SIDECAR_AUTH_TOKEN = -join ($bytes | ForEach-Object { $_.ToString("x2") })
        Set-Content -Path $tokenFile -Value $env:A4N_SIDECAR_AUTH_TOKEN -Encoding ascii
    }
}
if ([string]::IsNullOrWhiteSpace($env:A4N_SIDECAR_AUTH_TOKEN)) {
    throw "sidecar 认证 token 为空"
}

$managedPidFiles = New-Object System.Collections.Generic.List[string]
$hostProcess = $null

function Test-ProcessAlive {
    param([Parameter(Mandatory = $true)][int]$ProcessId)
    try {
        $process = Get-Process -Id $ProcessId -ErrorAction Stop
        return -not $process.HasExited
    } catch {
        return $false
    }
}

function Stop-ManagedProcess {
    param([Parameter(Mandatory = $true)][int]$ProcessId)

    try {
        $process = Get-Process -Id $ProcessId -ErrorAction Stop
    } catch {
        return
    }

    try {
        Stop-Process -Id $ProcessId -ErrorAction SilentlyContinue
        $process.WaitForExit(2000) | Out-Null
    } catch {
    }

    if (Test-ProcessAlive $ProcessId) {
        Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
    }
}

function Test-ServiceHealth {
    param(
        [Parameter(Mandatory = $true)][string]$Module,
        [Parameter(Mandatory = $true)][string]$Url
    )

    try {
        $response = Invoke-WebRequest -Uri "$($Url.TrimEnd('/'))/healthz" -UseBasicParsing -TimeoutSec 2
        $health = $response.Content | ConvertFrom-Json
        return $health.service -eq $Module -and $health.status -eq "healthy"
    } catch {
        return $false
    }
}

function Invoke-Cleanup {
    if ($script:hostProcess -ne $null -and -not ($script:hostProcess).HasExited) {
        Stop-ManagedProcess ($script:hostProcess).Id
    }

    foreach ($pidFile in $script:managedPidFiles) {
        if (-not (Test-Path $pidFile)) {
            continue
        }
        $rawPid = (Get-Content -Path $pidFile -ErrorAction SilentlyContinue | Select-Object -First 1)
        $pidValue = 0
        if ([int]::TryParse($rawPid, [ref]$pidValue) -and (Test-ProcessAlive $pidValue)) {
            Stop-ManagedProcess $pidValue
        }
        Remove-Item -Path $pidFile -Force -ErrorAction SilentlyContinue
    }
}

function Wait-ForHealth {
    param(
        [Parameter(Mandatory = $true)][string]$Module,
        [Parameter(Mandatory = $true)][System.Diagnostics.Process]$Process,
        [Parameter(Mandatory = $true)][string]$Url
    )

    $healthUrl = "$($Url.TrimEnd('/'))/healthz"
    for ($i = 0; $i -lt 50; $i++) {
        if (Test-ServiceHealth -Module $Module -Url $Url) {
            return
        }
        if ($Process.HasExited) {
            throw "sidecar $Module 启动后已退出，日志: $(Join-Path $ServiceLogDir "$Module.log")"
        }
        Start-Sleep -Milliseconds 100
    }
    throw "sidecar $Module 健康检查超时: $healthUrl"
}

function Start-A4nService {
    param(
        [Parameter(Mandatory = $true)][string]$Module,
        [Parameter(Mandatory = $true)][string]$BinaryName,
        [Parameter(Mandatory = $true)][string]$AddrEnv,
        [Parameter(Mandatory = $true)][string]$Addr,
        [Parameter(Mandatory = $true)][string]$UrlEnv,
        [Parameter(Mandatory = $true)][string]$Url
    )

    [Environment]::SetEnvironmentVariable($AddrEnv, $Addr, "Process")
    [Environment]::SetEnvironmentVariable($UrlEnv, $Url, "Process")

    $binary = Join-Path $env:A4N_SIDECAR_BIN_DIR $BinaryName
    $pidFile = Join-Path $ServicePidDir "$Module.pid"
    $logFile = Join-Path $ServiceLogDir "$Module.log"
    $errFile = Join-Path $ServiceLogDir "$Module.err.log"

    if (-not (Test-Path $binary)) {
        throw "缺少 sidecar 二进制: $binary"
    }

    if (Test-Path $pidFile) {
        $rawPid = (Get-Content -Path $pidFile -ErrorAction SilentlyContinue | Select-Object -First 1)
        $pidValue = 0
        if ([int]::TryParse($rawPid, [ref]$pidValue) -and (Test-ProcessAlive $pidValue)) {
            if (Test-ServiceHealth -Module $Module -Url $Url) {
                Write-Host "sidecar $Module 已在运行，PID: $pidValue"
                return
            }
            Write-Warning "忽略无法通过 $Module health 身份校验的旧 PID: $pidValue"
        }
        Remove-Item -Path $pidFile -Force -ErrorAction SilentlyContinue
    }

    $process = Start-Process `
        -FilePath $binary `
        -WorkingDirectory $ScriptDir `
        -RedirectStandardOutput $logFile `
        -RedirectStandardError $errFile `
        -WindowStyle Hidden `
        -PassThru
    Set-Content -Path $pidFile -Value $process.Id -Encoding ascii
    $script:managedPidFiles.Add($pidFile) | Out-Null
    Wait-ForHealth -Module $Module -Process $process -Url $Url
    Write-Host "sidecar $Module 已启动，PID: $($process.Id)，日志: $logFile"
}

$services = @(
    @{ Module = "prompt"; Binary = "a4n-prompt-service.exe"; AddrEnv = "A4N_PROMPT_SERVICE_ADDR"; Addr = $env:A4N_PROMPT_SERVICE_ADDR; UrlEnv = "A4N_PROMPT_SERVICE_URL"; Url = $env:A4N_PROMPT_SERVICE_URL },
    @{ Module = "llm"; Binary = "a4n-llm-gateway.exe"; AddrEnv = "A4N_LLM_GATEWAY_ADDR"; Addr = $env:A4N_LLM_GATEWAY_ADDR; UrlEnv = "A4N_LLM_GATEWAY_URL"; Url = $env:A4N_LLM_GATEWAY_URL },
    @{ Module = "embedding"; Binary = "a4n-embedding-service.exe"; AddrEnv = "A4N_EMBEDDING_SERVICE_ADDR"; Addr = $env:A4N_EMBEDDING_SERVICE_ADDR; UrlEnv = "A4N_EMBEDDING_SERVICE_URL"; Url = $env:A4N_EMBEDDING_SERVICE_URL },
    @{ Module = "rag"; Binary = "a4n-rag-service.exe"; AddrEnv = "A4N_RAG_SERVICE_ADDR"; Addr = $env:A4N_RAG_SERVICE_ADDR; UrlEnv = "A4N_RAG_SERVICE_URL"; Url = $env:A4N_RAG_SERVICE_URL },
    @{ Module = "image"; Binary = "a4n-image-service.exe"; AddrEnv = "A4N_IMAGE_SERVICE_ADDR"; Addr = $env:A4N_IMAGE_SERVICE_ADDR; UrlEnv = "A4N_IMAGE_SERVICE_URL"; Url = $env:A4N_IMAGE_SERVICE_URL },
    @{ Module = "video"; Binary = "a4n-video-service.exe"; AddrEnv = "A4N_VIDEO_SERVICE_ADDR"; Addr = $env:A4N_VIDEO_SERVICE_ADDR; UrlEnv = "A4N_VIDEO_SERVICE_URL"; Url = $env:A4N_VIDEO_SERVICE_URL },
    @{ Module = "import"; Binary = "a4n-import-service.exe"; AddrEnv = "A4N_IMPORT_SERVICE_ADDR"; Addr = $env:A4N_IMPORT_SERVICE_ADDR; UrlEnv = "A4N_IMPORT_SERVICE_URL"; Url = $env:A4N_IMPORT_SERVICE_URL },
    @{ Module = "export"; Binary = "a4n-export-service.exe"; AddrEnv = "A4N_EXPORT_SERVICE_ADDR"; Addr = $env:A4N_EXPORT_SERVICE_ADDR; UrlEnv = "A4N_EXPORT_SERVICE_URL"; Url = $env:A4N_EXPORT_SERVICE_URL }
)

$exitCode = 0
try {
    Write-Host "正在启动 A4N sidecar 服务..."
    foreach ($service in $services) {
        Start-A4nService `
            -Module $service.Module `
            -BinaryName $service.Binary `
            -AddrEnv $service.AddrEnv `
            -Addr $service.Addr `
            -UrlEnv $service.UrlEnv `
            -Url $service.Url
    }

    Write-Host "正在启动 A4N..."
    $hostBinary = Join-Path $ScriptDir "bin\a4n-tauri.exe"
    $hostProcess = Start-Process -FilePath $hostBinary -WorkingDirectory $ScriptDir -PassThru
    $hostProcess.WaitForExit()
    $exitCode = $hostProcess.ExitCode
} finally {
    Invoke-Cleanup
}

exit $exitCode
PSEOF
        echo -e "  ${GREEN}✓${NC} 启动脚本: start.bat / start.ps1"
    else
        cat > "$PACKAGE_DIR/start.sh" << 'SHEOF'
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

verify_release_manifest() {
    local checksum_file="$SCRIPT_DIR/release-manifest.sha256"
    [[ -r "$SCRIPT_DIR/release-manifest.json" ]] || {
        echo "缺少 release-manifest.json" >&2
        return 1
    }
    [[ -r "$checksum_file" ]] || {
        echo "缺少 release-manifest.sha256" >&2
        return 1
    }
    while read -r expected relative; do
        [[ -n "$expected" && -n "$relative" ]] || continue
        case "$relative" in
            /*|../*|*/../*|\\..\\*|..\\*)
                echo "release manifest 路径越界: $relative" >&2
                return 1
                ;;
        esac
        [[ -f "$SCRIPT_DIR/$relative" ]] || {
            echo "release manifest 缺少二进制: $relative" >&2
            return 1
        }
        if command -v sha256sum >/dev/null 2>&1; then
            printf '%s  %s\n' "$expected" "$relative" | (cd "$SCRIPT_DIR" && sha256sum -c -) >/dev/null || return 1
        elif command -v shasum >/dev/null 2>&1; then
            actual="$(shasum -a 256 "$SCRIPT_DIR/$relative" | awk '{print $1}')"
            [[ "$actual" == "$expected" ]] || return 1
        else
            echo "缺少 sha256sum 或 shasum，拒绝启动" >&2
            return 1
        fi
    done <"$checksum_file"
}

verify_release_manifest

export A4N_PORTABLE_HOME="${A4N_PORTABLE_HOME:-$SCRIPT_DIR}"
export A4N_DATA_DIR="${A4N_DATA_DIR:-$A4N_PORTABLE_HOME/data}"
export A4N_CACHE_DIR="${A4N_CACHE_DIR:-$A4N_DATA_DIR/cache}"
export A4N_LOG_DIR="${A4N_LOG_DIR:-$A4N_DATA_DIR/logs}"
export A4N_TEMP_DIR="${A4N_TEMP_DIR:-$A4N_DATA_DIR/tmp}"
export A4N_RUNTIME_PROFILE="${A4N_RUNTIME_PROFILE:-sidecar}"
export A4N_RELEASE_MANIFEST_PATH="${A4N_RELEASE_MANIFEST_PATH:-$SCRIPT_DIR/release-manifest.json}"
export A4N_SIDECAR_BIN_DIR="${A4N_SIDECAR_BIN_DIR:-$SCRIPT_DIR/bin/services}"
export LD_LIBRARY_PATH="$A4N_SIDECAR_BIN_DIR${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export DYLD_LIBRARY_PATH="$A4N_SIDECAR_BIN_DIR${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
export A4N_PROMPT_SERVICE_PROMPTS_DIR="${A4N_PROMPT_SERVICE_PROMPTS_DIR:-$SCRIPT_DIR/prompts}"
export A4N_RAG_SERVICE_DATA_DIR="${A4N_RAG_SERVICE_DATA_DIR:-$A4N_DATA_DIR}"
export A4N_EMBEDDING_CACHE_DIR="${A4N_EMBEDDING_CACHE_DIR:-$A4N_CACHE_DIR/embedding-models}"

export A4N_PROMPT_SERVICE_ADDR="${A4N_PROMPT_SERVICE_ADDR:-127.0.0.1:47641}"
export A4N_LLM_GATEWAY_ADDR="${A4N_LLM_GATEWAY_ADDR:-127.0.0.1:47642}"
export A4N_EMBEDDING_SERVICE_ADDR="${A4N_EMBEDDING_SERVICE_ADDR:-127.0.0.1:47643}"
export A4N_RAG_SERVICE_ADDR="${A4N_RAG_SERVICE_ADDR:-127.0.0.1:47644}"
export A4N_IMAGE_SERVICE_ADDR="${A4N_IMAGE_SERVICE_ADDR:-127.0.0.1:47645}"
export A4N_VIDEO_SERVICE_ADDR="${A4N_VIDEO_SERVICE_ADDR:-127.0.0.1:47646}"
export A4N_IMPORT_SERVICE_ADDR="${A4N_IMPORT_SERVICE_ADDR:-127.0.0.1:47647}"
export A4N_EXPORT_SERVICE_ADDR="${A4N_EXPORT_SERVICE_ADDR:-127.0.0.1:47648}"

export A4N_PROMPT_SERVICE_URL="${A4N_PROMPT_SERVICE_URL:-http://$A4N_PROMPT_SERVICE_ADDR}"
export A4N_LLM_GATEWAY_URL="${A4N_LLM_GATEWAY_URL:-http://$A4N_LLM_GATEWAY_ADDR}"
export A4N_EMBEDDING_SERVICE_URL="${A4N_EMBEDDING_SERVICE_URL:-http://$A4N_EMBEDDING_SERVICE_ADDR}"
export A4N_RAG_SERVICE_URL="${A4N_RAG_SERVICE_URL:-http://$A4N_RAG_SERVICE_ADDR}"
export A4N_IMAGE_SERVICE_URL="${A4N_IMAGE_SERVICE_URL:-http://$A4N_IMAGE_SERVICE_ADDR}"
export A4N_VIDEO_SERVICE_URL="${A4N_VIDEO_SERVICE_URL:-http://$A4N_VIDEO_SERVICE_ADDR}"
export A4N_IMPORT_SERVICE_URL="${A4N_IMPORT_SERVICE_URL:-http://$A4N_IMPORT_SERVICE_ADDR}"
export A4N_EXPORT_SERVICE_URL="${A4N_EXPORT_SERVICE_URL:-http://$A4N_EXPORT_SERVICE_ADDR}"

SERVICE_PID_DIR="$A4N_DATA_DIR/services"
SERVICE_LOG_DIR="$A4N_LOG_DIR/services"
mkdir -p "$SERVICE_PID_DIR" "$SERVICE_LOG_DIR" "$A4N_CACHE_DIR" "$A4N_TEMP_DIR"

token_file="$SERVICE_PID_DIR/auth-token"
if [[ -z "${A4N_SIDECAR_AUTH_TOKEN:-}" ]]; then
    if [[ -s "$token_file" ]]; then
        A4N_SIDECAR_AUTH_TOKEN="$(tr -d '[:space:]' <"$token_file")"
    else
        A4N_SIDECAR_AUTH_TOKEN="$(od -An -N32 -tx1 /dev/urandom | tr -d '[:space:]')"
        umask 077
        printf '%s\n' "$A4N_SIDECAR_AUTH_TOKEN" >"$token_file"
    fi
fi
[[ -n "$A4N_SIDECAR_AUTH_TOKEN" ]] || {
    echo "sidecar 认证 token 为空" >&2
    exit 1
}
export A4N_SIDECAR_AUTH_TOKEN

managed_pid_files=()
host_pid=""

stop_managed_pid() {
    local pid="$1"
    [[ "$pid" =~ ^[0-9]+$ ]] || return 0
    kill -0 "$pid" 2>/dev/null || return 0
    kill -TERM "$pid" 2>/dev/null || true
    for _ in {1..20}; do
        if ! kill -0 "$pid" 2>/dev/null; then
            wait "$pid" 2>/dev/null || true
            return 0
        fi
        sleep 0.1
    done
    kill -KILL "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
}

cleanup() {
    local pid_file pid
    [[ -z "$host_pid" ]] || stop_managed_pid "$host_pid"
    for pid_file in "${managed_pid_files[@]}"; do
        [[ -f "$pid_file" ]] || continue
        pid="$(cat "$pid_file" 2>/dev/null || true)"
        stop_managed_pid "$pid"
        rm -f "$pid_file"
    done
}

trap cleanup EXIT INT TERM

wait_for_health() {
    local module="$1"
    local pid="$2"
    local url="$3"

    command -v curl >/dev/null 2>&1 || {
        echo "缺少 curl，无法验证 sidecar $module health" >&2
        return 1
    }
    for _ in {1..50}; do
        if service_is_healthy "$module" "$url"; then
            return 0
        fi
        if ! kill -0 "$pid" 2>/dev/null; then
            echo "sidecar $module 启动后已退出，日志: $SERVICE_LOG_DIR/$module.log" >&2
            return 1
        fi
        sleep 0.1
    done
    echo "sidecar $module 健康检查超时或身份不匹配: $url/healthz" >&2
    return 1
}

service_is_healthy() {
    local module="$1"
    local url="$2"
    local payload
    payload="$(curl -fsS --max-time 2 "$url/healthz" 2>/dev/null)" || return 1
    printf '%s\n' "$payload" | grep -Eq '"service"[[:space:]]*:[[:space:]]*"'"$module"'"' || return 1
    printf '%s\n' "$payload" | grep -Eq '"status"[[:space:]]*:[[:space:]]*"healthy"'
}

start_service() {
    local module="$1"
    local binary_name="$2"
    local addr_env="$3"
    local addr="$4"
    local url_env="$5"
    local url="$6"
    local binary="$A4N_SIDECAR_BIN_DIR/$binary_name"
    local pid_file="$SERVICE_PID_DIR/$module.pid"
    local log_file="$SERVICE_LOG_DIR/$module.log"
    local old_pid pid

    export "$addr_env=$addr"
    export "$url_env=$url"

    if [[ ! -x "$binary" ]]; then
        echo "缺少 sidecar 二进制或不可执行: $binary" >&2
        exit 1
    fi

    if [[ -f "$pid_file" ]]; then
        old_pid="$(cat "$pid_file" 2>/dev/null || true)"
        if [[ -n "$old_pid" ]] && kill -0 "$old_pid" 2>/dev/null; then
            if service_is_healthy "$module" "$url"; then
                echo "sidecar $module 已在运行，PID: $old_pid"
                return 0
            fi
            echo "忽略无法通过 $module health 身份校验的旧 PID: $old_pid" >&2
        fi
        rm -f "$pid_file"
    fi

    (
        cd "$SCRIPT_DIR"
        exec "$binary"
    ) >>"$log_file" 2>&1 &
    pid=$!
    printf '%s\n' "$pid" >"$pid_file"
    managed_pid_files+=("$pid_file")
    wait_for_health "$module" "$pid" "$url"
    echo "sidecar $module 已启动，PID: $pid，日志: $log_file"
}

echo "正在启动 A4N sidecar 服务..."
start_service "prompt" "a4n-prompt-service" "A4N_PROMPT_SERVICE_ADDR" "$A4N_PROMPT_SERVICE_ADDR" "A4N_PROMPT_SERVICE_URL" "$A4N_PROMPT_SERVICE_URL"
start_service "llm" "a4n-llm-gateway" "A4N_LLM_GATEWAY_ADDR" "$A4N_LLM_GATEWAY_ADDR" "A4N_LLM_GATEWAY_URL" "$A4N_LLM_GATEWAY_URL"
start_service "embedding" "a4n-embedding-service" "A4N_EMBEDDING_SERVICE_ADDR" "$A4N_EMBEDDING_SERVICE_ADDR" "A4N_EMBEDDING_SERVICE_URL" "$A4N_EMBEDDING_SERVICE_URL"
start_service "rag" "a4n-rag-service" "A4N_RAG_SERVICE_ADDR" "$A4N_RAG_SERVICE_ADDR" "A4N_RAG_SERVICE_URL" "$A4N_RAG_SERVICE_URL"
start_service "image" "a4n-image-service" "A4N_IMAGE_SERVICE_ADDR" "$A4N_IMAGE_SERVICE_ADDR" "A4N_IMAGE_SERVICE_URL" "$A4N_IMAGE_SERVICE_URL"
start_service "video" "a4n-video-service" "A4N_VIDEO_SERVICE_ADDR" "$A4N_VIDEO_SERVICE_ADDR" "A4N_VIDEO_SERVICE_URL" "$A4N_VIDEO_SERVICE_URL"
start_service "import" "a4n-import-service" "A4N_IMPORT_SERVICE_ADDR" "$A4N_IMPORT_SERVICE_ADDR" "A4N_IMPORT_SERVICE_URL" "$A4N_IMPORT_SERVICE_URL"
start_service "export" "a4n-export-service" "A4N_EXPORT_SERVICE_ADDR" "$A4N_EXPORT_SERVICE_ADDR" "A4N_EXPORT_SERVICE_URL" "$A4N_EXPORT_SERVICE_URL"

echo "正在启动 A4N..."
bin/a4n-tauri &
host_pid=$!
set +e
wait "$host_pid"
status=$?
set -e
host_pid=""
exit "$status"
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
  Windows:      双击 start.bat（内部调用 start.ps1 管理 sidecar 生命周期）

默认运行模式:
  启动脚本默认设置 A4N_RUNTIME_PROFILE=sidecar，并从 bin/services/ 启动 prompt、
  LLM、embedding、RAG、image、video、import、export 八个 sidecar。替换某个
  sidecar 二进制后，重启对应 sidecar 或重新运行启动脚本即可生效。

首次启动前必须在 data/config.toml 填写 database_url，并配置 JWT secret：
  database_url = "postgres://user:password@host:5432/database"
  [security]
  jwt_secret = "replace-with-a-long-random-secret"

目录结构:
  bin/         可执行文件
  bin/services/ prompt/LLM/embedding/RAG/image/video/import/export sidecar
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
        sudo mkdir -p "${install_dir}/lib/a4n/services"
        for package in "${SIDECAR_PACKAGES[@]}"; do
            sudo cp "$(release_binary_path "$package")" "${install_dir}/lib/a4n/services/"
        done
        sudo mkdir -p "${install_dir}/share/a4n/static"
        sudo mkdir -p "${install_dir}/share/a4n/prompts"
        sudo cp -r "$FRONTEND_DIST"/* "${install_dir}/share/a4n/static/" 2>/dev/null || true
        sudo cp -r "$PROJECT_ROOT/prompts/"* "${install_dir}/share/a4n/prompts/" 2>/dev/null || true
    else
        cp "$RELEASE_BINARY" "${install_dir}/bin/"
        mkdir -p "${install_dir}/lib/a4n/services"
        for package in "${SIDECAR_PACKAGES[@]}"; do
            cp "$(release_binary_path "$package")" "${install_dir}/lib/a4n/services/"
        done
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
    echo "  - bin/services/ (${#SIDECAR_PACKAGES[@]} 个 sidecar 二进制)"
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

guard_build_cache

if $DO_INSTALL; then
    do_build
    install_to_system
    exit 0
fi

do_build
prepare_package
create_archive
