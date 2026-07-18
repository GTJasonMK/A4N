#!/usr/bin/env bash
#=============================================================================
# A4N 模块级检查脚本
#
# 用法:
#   ./scripts/dev/check-module.sh host-tauri
#   ./scripts/dev/check-module.sh host-tauri-full
#   ./scripts/dev/check-module.sh host-server
#   ./scripts/dev/check-module.sh host-server-full
#   ./scripts/dev/check-module.sh runtime
#   ./scripts/dev/check-module.sh queue
#   ./scripts/dev/check-module.sh protocol
#   ./scripts/dev/check-module.sh capabilities
#   ./scripts/dev/check-module.sh service-manager
#   ./scripts/dev/check-module.sh demo-sidecar
#   ./scripts/dev/check-module.sh prompt
#   ./scripts/dev/check-module.sh prompt-git
#   ./scripts/dev/check-module.sh llm
#   ./scripts/dev/check-module.sh llm-http
#   ./scripts/dev/check-module.sh embedding
#   ./scripts/dev/check-module.sh rag
#   ./scripts/dev/check-module.sh image
#   ./scripts/dev/check-module.sh image-providers
#   ./scripts/dev/check-module.sh video
#   ./scripts/dev/check-module.sh video-providers
#   ./scripts/dev/check-module.sh media
#   ./scripts/dev/check-module.sh archive
#   ./scripts/dev/check-module.sh import
#   ./scripts/dev/check-module.sh export
#   ./scripts/dev/check-module.sh orchestrator
#   ./scripts/dev/check-module.sh test-support
#   ./scripts/dev/check-module.sh --list
#=============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/common.sh
source "$SCRIPT_DIR/../lib/common.sh"

module_package() {
    case "$1" in
        host-tauri|tauri|host) printf '%s\n' "a4n-tauri" ;;
        host-tauri-full|tauri-full|host-full) printf '%s\n' "a4n-tauri" ;;
        host-server|server|web) printf '%s\n' "a4n-server" ;;
        host-server-full|server-full|web-full) printf '%s\n' "a4n-server" ;;
        runtime) printf '%s\n' "a4n-runtime" ;;
        core) printf '%s\n' "a4n-core" ;;
        migration) printf '%s\n' "a4n-migration" ;;
        queue) printf '%s\n' "a4n-queue" ;;
        protocol|service-protocol) printf '%s\n' "a4n-service-protocol" ;;
        capabilities) printf '%s\n' "a4n-capabilities" ;;
        service-manager|manager) printf '%s\n' "a4n-service-manager" ;;
        demo-sidecar|demo) printf '%s\n' "a4n-demo-sidecar" ;;
        prompt|prompt-core|prompt-lib) printf '%s\n' "a4n-prompt" ;;
        prompt-git) printf '%s\n' "a4n-prompt-git" ;;
        prompt-service) printf '%s\n' "a4n-prompt-service" ;;
        llm|llm-core|llm-lib) printf '%s\n' "a4n-llm" ;;
        llm-http) printf '%s\n' "a4n-llm-http" ;;
        llm-gateway) printf '%s\n' "a4n-llm-gateway" ;;
        embedding) printf '%s\n' "a4n-embedding-service" ;;
        rag) printf '%s\n' "a4n-rag-service" ;;
        image|image-service) printf '%s\n' "a4n-image-service" ;;
        image-core|image-lib) printf '%s\n' "a4n-image" ;;
        image-providers) printf '%s\n' "a4n-image-providers" ;;
        video|video-service) printf '%s\n' "a4n-video-service" ;;
        video-core|video-lib) printf '%s\n' "a4n-video" ;;
        video-providers) printf '%s\n' "a4n-video-providers" ;;
        media|media-core|media-lib) printf '%s\n' "a4n-media" ;;
        archive|archive-core|archive-lib) printf '%s\n' "a4n-archive-core" ;;
        import|import-service) printf '%s\n' "a4n-import-service" ;;
        import-core|import-lib) printf '%s\n' "a4n-import-core" ;;
        export|export-service) printf '%s\n' "a4n-export-service" ;;
        export-core|export-lib) printf '%s\n' "a4n-export-core" ;;
        domain) printf '%s\n' "a4n-domain" ;;
        db) printf '%s\n' "a4n-db" ;;
        orchestrator) printf '%s\n' "a4n-orchestrator" ;;
        test-support) printf '%s\n' "a4n-test-support" ;;
        *)
            return 1
            ;;
    esac
}

module_cargo_args() {
    case "$1" in
        host-tauri|tauri|host|host-server|server|web)
            printf '%s\n' "--no-default-features"
            ;;
        host-tauri-full|tauri-full|host-full|host-server-full|server-full|web-full)
            printf '%s\n' "--features" "embedded-capabilities"
            ;;
    esac
}

print_modules() {
    cat <<'EOF'
可检查模块:
  host-tauri       -> a4n-tauri --no-default-features
  host-tauri-full  -> a4n-tauri --features embedded-capabilities
  host-server      -> a4n-server --no-default-features
  host-server-full -> a4n-server --features embedded-capabilities
  runtime          -> a4n-runtime
  core             -> a4n-core
  migration        -> a4n-migration
  queue            -> a4n-queue
  protocol         -> a4n-service-protocol
  capabilities     -> a4n-capabilities
  service-manager  -> a4n-service-manager
  demo-sidecar     -> a4n-demo-sidecar
  prompt           -> a4n-prompt
  prompt-git       -> a4n-prompt-git
  prompt-service   -> a4n-prompt-service
  llm              -> a4n-llm
  llm-http         -> a4n-llm-http
  llm-gateway      -> a4n-llm-gateway
  embedding        -> a4n-embedding-service
  rag              -> a4n-rag-service
  image            -> a4n-image-service
  image-core       -> a4n-image
  image-providers  -> a4n-image-providers
  video            -> a4n-video-service
  video-core       -> a4n-video
  video-providers  -> a4n-video-providers
  media            -> a4n-media
  archive          -> a4n-archive-core
  import           -> a4n-import-service
  import-core      -> a4n-import-core
  export           -> a4n-export-service
  export-core      -> a4n-export-core
  domain           -> a4n-domain
  db               -> a4n-db
  orchestrator     -> a4n-orchestrator
  test-support     -> a4n-test-support
EOF
}

if [[ $# -ne 1 ]]; then
    print_script_usage "$0" 19
    exit 1
fi

case "$1" in
    --list)
        print_modules
        exit 0
        ;;
    --help|-h)
        print_script_usage "$0" 19
        echo ""
        print_modules
        exit 0
        ;;
esac

module="$1"
if ! package="$(module_package "$module")"; then
    die "未知模块: $module；使用 --list 查看可用模块"
fi

if ! cargo_package_exists "$package"; then
    die "模块 '$module' 映射到 package '$package'，但该 package 尚不存在；请先完成对应重构阶段"
fi

extra_args=()
while IFS= read -r arg; do
    [[ -n "$arg" ]] && extra_args+=("$arg")
done < <(module_cargo_args "$module")

exec "$PROJECT_ROOT/scripts/build.sh" --check -p "$package" "${extra_args[@]}"
