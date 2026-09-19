#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"
cd "$PROJECT_ROOT"

# 06 首个候选：export。它是无数据库写入的叶子计算，Host 仍拥有授权、任务、
# checkpoint、取消、最终提交和恢复；这里检查“实现二进制可单独替换”的边界。
HOST_PACKAGE="${A4N_WORKER_ISOLATION_HOST_PACKAGE:-a4n-tauri}"
WORKER_PACKAGE="${A4N_WORKER_ISOLATION_WORKER_PACKAGE:-a4n-export-service}"
SERVICE_NAME="export"

# Provider implementations are leaf-worker candidates. Keep their contracts,
# adapters, and service entrypoints independent from Host core so changing an
# adapter cannot invalidate database/orchestrator code.
IMAGE_PACKAGE="a4n-image-provider-contract"
IMAGE_PROVIDER_PACKAGE="a4n-image-providers"
IMAGE_SERVICE_PACKAGE="a4n-image-service"
EMBEDDING_PACKAGE="a4n-embedding-provider-contract"
EMBEDDING_PROVIDER_PACKAGE="a4n-embedding-fastembed"
EMBEDDING_SERVICE_PACKAGE="a4n-embedding-service"
VIDEO_PACKAGE="a4n-video"
VIDEO_PROVIDER_PACKAGE="a4n-video-providers"
VIDEO_SERVICE_PACKAGE="a4n-video-service"
RAG_CORE_PACKAGE="a4n-rag-core"
RAG_INDEX_PACKAGE="a4n-rag-index"
RAG_SERVICE_PACKAGE="a4n-rag-service"
STATEFUL_WORKER_PACKAGES=(
    a4n-agent-worker
    a4n-novel-planning-worker
    a4n-artifact-media-worker
    a4n-outline-worker
    a4n-blueprint-worker
    a4n-asset-graph-worker
    a4n-foreshadowing-worker
    a4n-object-profile-worker
    a4n-scene-profile-worker
    a4n-profile-material-worker
    a4n-project-status-worker
    a4n-portrait-worker
    a4n-source-snapshot-worker
)
WORKER_CLIENT_PACKAGES=(
    a4n-novel-planning-worker-client
    a4n-artifact-media-client
    a4n-outline-worker-client
    a4n-blueprint-worker-client
    a4n-asset-graph-worker-client
    a4n-foreshadowing-worker-client
    a4n-object-profile-worker-client
    a4n-scene-profile-worker-client
    a4n-profile-material-worker-client
    a4n-project-status-worker-client
    a4n-portrait-worker-client
    a4n-source-snapshot-worker-client
)

LEAF_PACKAGES=(
    "$IMAGE_PACKAGE" "$IMAGE_PROVIDER_PACKAGE" "$IMAGE_SERVICE_PACKAGE"
    "$EMBEDDING_PACKAGE" "$EMBEDDING_PROVIDER_PACKAGE" "$EMBEDDING_SERVICE_PACKAGE"
    "$VIDEO_PACKAGE" "$VIDEO_PROVIDER_PACKAGE" "$VIDEO_SERVICE_PACKAGE"
    "$RAG_CORE_PACKAGE" "$RAG_INDEX_PACKAGE" "$RAG_SERVICE_PACKAGE"
)

for package in "${LEAF_PACKAGES[@]}"; do
    if rg -n 'a4n-core|a4n_core' "crates/$package/Cargo.toml" "crates/$package/src"; then
        echo "$package leaf must not depend on the removed Host core" >&2
        exit 1
    fi
    image_tree="$(cargo tree --locked --edges normal,build -p "$package")"
    if grep -q 'a4n-core v' <<<"$image_tree"; then
        echo "$package dependency tree unexpectedly contains a4n-core" >&2
        exit 1
    fi
done

host_tree="$(cargo tree --locked -e normal,build -p "$HOST_PACKAGE" --no-default-features)"
if grep -Eq "(^|[[:space:]])${WORKER_PACKAGE}([[:space:]]|v|$)" <<<"$host_tree"; then
    echo "Host dependency tree unexpectedly contains Worker package: $WORKER_PACKAGE" >&2
    exit 1
fi
if grep -q 'a4n-export-core v' <<<"$host_tree"; then
    echo "Host dependency tree unexpectedly contains export Worker implementation" >&2
    exit 1
fi

server_tree="$(cargo tree --locked -e normal,build -p a4n-server --no-default-features)"
if grep -Eq '(^|[[:space:]])(a4n-edge|a4n-control)([[:space:]]|v|$)' <<<"$server_tree"; then
    echo "default Server dependency tree unexpectedly contains supervised process implementation" >&2
    exit 1
fi
for package in "${STATEFUL_WORKER_PACKAGES[@]}"; do
    if grep -Eq "(^|[[:space:]])${package}([[:space:]]|v|$)" <<<"$server_tree"; then
        echo "default Server dependency tree unexpectedly contains Worker implementation: $package" >&2
        exit 1
    fi
done

worker_client_tree="$(cargo tree --locked -e normal,build -p a4n-worker-client)"
if grep -Eq '(^|[[:space:]])(a4n-(db|db-models|runtime|orchestrator)|sea-orm|sqlx)([[:space:]]|v|$)' <<<"$worker_client_tree"; then
    echo "Worker client must remain free of business persistence and orchestrator implementations" >&2
    exit 1
fi

control_tree="$(cargo tree --locked -e normal,build -p a4n-control --no-default-features)"
for package in "${STATEFUL_WORKER_PACKAGES[@]}"; do
    if grep -Eq "(^|[[:space:]])${package}([[:space:]]|v|$)" <<<"$control_tree"; then
        echo "Control dependency tree unexpectedly contains Worker implementation: $package" >&2
        exit 1
    fi
done
for client in "${WORKER_CLIENT_PACKAGES[@]}"; do
    client_tree="$(cargo tree --locked -e normal,build -p "$client")"
    for package in "${STATEFUL_WORKER_PACKAGES[@]}"; do
        if grep -Eq "(^|[[:space:]])${package}([[:space:]]|v|$)" <<<"$client_tree"; then
            echo "$client dependency tree unexpectedly contains Worker implementation: $package" >&2
            exit 1
        fi
    done
done

if rg -n 'a4n[-_]export[-_]core' crates/a4n-test-support/Cargo.toml crates/a4n-test-support/src; then
    echo "shared test support must not pull the export Worker implementation" >&2
    exit 1
fi

cargo check --locked -p "$WORKER_PACKAGE" >/dev/null
cargo check --locked -p a4n-worker-client >/dev/null
cargo test --locked -p a4n-service-protocol --lib >/dev/null

worker_main="crates/$WORKER_PACKAGE/src/main.rs"
[[ -r "$worker_main" ]] || {
    echo "Worker entrypoint is missing: $worker_main" >&2
    exit 1
}
rg -q --fixed-strings 'a4n_sidecar_bootstrap::serve_sidecar' "$worker_main" || {
    echo "Worker must use shared sidecar bootstrap" >&2
    exit 1
}
rg -q --fixed-strings 'EXPORT_SERVICE_NAME' "$worker_main" || {
    echo "Worker must bind the catalog service identity" >&2
    exit 1
}
rg -q --fixed-strings 'sidecar_resource_policy' "$worker_main" || {
    echo "Worker must use the service-protocol resource policy" >&2
    exit 1
}
if rg -n 'DatabaseConnection|sqlx|DATABASE_URL|JWT_SECRET|task_queue|generation_task_checkpoint' "$worker_main"; then
    echo "Worker entrypoint must not own Host database/task/recovery state" >&2
    exit 1
fi

ruby -rjson -e '
  catalog = JSON.parse(File.read("crates/a4n-process-catalog/service-catalog.json"))
  entry = catalog.find { |candidate| candidate.fetch("name") == ARGV.fetch(0) }
  abort "service catalog candidate is missing" unless entry
  abort "candidate binary must match catalog" unless entry.fetch("binary") == ARGV.fetch(1)
  abort "candidate capability version is blank" if entry.fetch("capability_version").strip.empty?
' "$SERVICE_NAME" "$WORKER_PACKAGE"

echo "Worker isolation boundary passed: host=$HOST_PACKAGE worker=$WORKER_PACKAGE service=$SERVICE_NAME"
