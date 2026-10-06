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
worker_packages="$(stateful_worker_packages)" || {
    echo "Unable to read stateful worker catalog" >&2
    exit 1
}
worker_clients="$(worker_client_packages)" || {
    echo "Unable to read Cargo worker-client package list" >&2
    exit 1
}
[[ -n "$worker_packages" ]] || {
    echo "Stateful worker catalog is empty" >&2
    exit 1
}
[[ -n "$worker_clients" ]] || {
    echo "Cargo worker-client package list is empty" >&2
    exit 1
}
STATEFUL_WORKER_PACKAGES=()
while IFS= read -r package; do
    [[ -n "$package" ]] && STATEFUL_WORKER_PACKAGES+=("$package")
done <<< "$worker_packages"
WORKER_CLIENT_PACKAGES=()
while IFS= read -r package; do
    [[ -n "$package" ]] && WORKER_CLIENT_PACKAGES+=("$package")
done <<< "$worker_clients"

LEAF_PACKAGES=(
    "$IMAGE_PACKAGE" "$IMAGE_PROVIDER_PACKAGE" "$IMAGE_SERVICE_PACKAGE"
    "$EMBEDDING_PACKAGE" "$EMBEDDING_PROVIDER_PACKAGE" "$EMBEDDING_SERVICE_PACKAGE"
    "$VIDEO_PACKAGE" "$VIDEO_PROVIDER_PACKAGE" "$VIDEO_SERVICE_PACKAGE"
    "$RAG_CORE_PACKAGE" "$RAG_INDEX_PACKAGE" "$RAG_SERVICE_PACKAGE"
)

for package in "${LEAF_PACKAGES[@]}"; do
    package_dir="$(cargo_package_dir "$package")"
    if rg -n 'a4n-core|a4n_core' "$package_dir/Cargo.toml" "$package_dir/src"; then
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

duplicate_runtime_tree="$(cargo tree --workspace --locked --duplicates)"
if grep -Eq '^(axum|tokio|tower) v' <<<"$duplicate_runtime_tree"; then
    echo "workspace must keep a single Axum/Tokio/Tower version for independently packaged components" >&2
    grep -E '^(axum|tokio|tower) v' <<<"$duplicate_runtime_tree" >&2
    exit 1
fi

control_tree="$(cargo tree --locked -e normal,build -p a4n-control --no-default-features)"
for tree in "$control_tree" "$host_tree"; do
    if grep -Eq 'a4n-(import|export|document-export|media-processing|media-render)-core v' <<<"$tree"; then
        echo "default Host must use transform clients, not embedded implementations" >&2
        exit 1
    fi
done
for worker in "${STATEFUL_WORKER_PACKAGES[@]}"; do
    worker_tree="$(cargo tree --locked -e normal -p "$worker")"
    if grep -Eq 'a4n-(sidecar-bootstrap|worker-client|runtime|orchestrator) v' <<<"$worker_tree"; then
        echo "$worker must not depend on the Sidecar runtime helper or Host/client implementations" >&2
        exit 1
    fi
    grep -q 'a4n-db v' <<<"$worker_tree" || {
        echo "$worker must link the shared database runtime" >&2
        exit 1
    }
    if [[ "$worker" == a4n-import-review-worker ]] && grep -q 'a4n-import-core v' <<<"$worker_tree"; then
        echo "import review must not pull document/archive parsers for SimHash" >&2
        exit 1
    fi
done
for package in "${STATEFUL_WORKER_PACKAGES[@]}"; do
    if grep -Eq "(^|[[:space:]])${package}([[:space:]]|v|$)" <<<"$control_tree"; then
        echo "Control dependency tree unexpectedly contains Worker implementation: $package" >&2
        exit 1
    fi
done
for client in "${WORKER_CLIENT_PACKAGES[@]}"; do
    client_tree="$(cargo tree --locked -e normal,build -p "$client")"
    if grep -Eq '(^|[[:space:]])(a4n-(db|db-models|migration|orchestrator)|sea-orm(-migration)?|sqlx)([[:space:]]|v|$)' <<<"$client_tree"; then
        echo "$client dependency tree unexpectedly contains business persistence or orchestrator" >&2
        exit 1
    fi
    for package in "${STATEFUL_WORKER_PACKAGES[@]}"; do
        if grep -Eq "(^|[[:space:]])${package}([[:space:]]|v|$)" <<<"$client_tree"; then
            echo "$client dependency tree unexpectedly contains Worker implementation: $package" >&2
            exit 1
        fi
    done
done

if rg -n 'a4n[-_]export[-_]core' crates/platform/a4n-test-support/Cargo.toml crates/platform/a4n-test-support/src; then
    echo "shared test support must not pull the export Worker implementation" >&2
    exit 1
fi

cargo check --locked -p "$WORKER_PACKAGE" >/dev/null
cargo check --locked -p a4n-worker-client >/dev/null
cargo test --locked -p a4n-service-protocol --lib >/dev/null

worker_source="$(cargo_package_dir "$WORKER_PACKAGE")/src"
[[ -d "$worker_source" ]] || {
    echo "Worker source is missing: $worker_source" >&2
    exit 1
}
rg -q --fixed-strings 'a4n_sidecar_bootstrap::serve_sidecar' "$worker_source" || {
    echo "Worker must use shared sidecar bootstrap" >&2
    exit 1
}
rg -q --fixed-strings 'EXPORT_SERVICE_NAME' "$worker_source" || {
    echo "Worker must bind the catalog service identity" >&2
    exit 1
}
rg -q --fixed-strings 'sidecar_resource_policy' "$worker_source" || {
    echo "Worker must use the service-protocol resource policy" >&2
    exit 1
}
if rg -n 'DatabaseConnection|sqlx|DATABASE_URL|JWT_SECRET|task_queue|generation_task_checkpoint' "$worker_source"; then
    echo "Worker entrypoint must not own Host database/task/recovery state" >&2
    exit 1
fi

ruby -rjson -e '
  catalog = JSON.parse(File.read("crates/process/a4n-process-catalog/service-catalog.json"))
  entry = catalog.find { |candidate| candidate.fetch("name") == ARGV.fetch(0) }
  abort "service catalog candidate is missing" unless entry
  abort "candidate binary must match catalog" unless entry.fetch("binary") == ARGV.fetch(1)
  abort "candidate capability version is blank" if entry.fetch("capability_version").strip.empty?
' "$SERVICE_NAME" "$WORKER_PACKAGE"

echo "Worker isolation boundary passed: host=$HOST_PACKAGE worker=$WORKER_PACKAGE service=$SERVICE_NAME"
