#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

ROUTERS=(
    crates/a4n-prompt-service/src/app.rs
    crates/a4n-llm-gateway/src/app.rs
    crates/a4n-embedding-service/src/app.rs
    crates/a4n-rag-service/src/app.rs
    crates/a4n-image-service/src/app.rs
    crates/a4n-video-service/src/main.rs
    crates/a4n-import-service/src/main.rs
    crates/a4n-export-service/src/main.rs
)

for router in "${ROUTERS[@]}"; do
    rg -q --fixed-strings 'a4n_sidecar_bootstrap::require_auth' "$router" || {
        printf 'sidecar router is missing shared authentication middleware: %s\n' "$router" >&2
        exit 1
    }
    rg -q --fixed-strings 'DefaultBodyLimit::max' "$router" || {
        printf 'sidecar router is missing an explicit body limit: %s\n' "$router" >&2
        exit 1
    }
    rg -q --fixed-strings 'a4n_sidecar_bootstrap::concurrency_limit' "$router" || {
        printf 'sidecar router is missing bounded concurrency: %s\n' "$router" >&2
        exit 1
    }
    rg -q --fixed-strings 'sidecar_resource_policy' "$router" || {
        printf 'sidecar router is not using the shared resource policy: %s\n' "$router" >&2
        exit 1
    }
    if rg -n 'DefaultBodyLimit::max\([0-9]' "$router"; then
        printf 'sidecar body limits must use named capability constants: %s\n' "$router" >&2
        exit 1
    fi
done

if rg -n 'const [A-Z0-9_]*(BODY_LIMIT|MAX_CONCURRENT)' \
    crates/a4n-prompt-service/src/app.rs \
    crates/a4n-llm-gateway/src/app.rs \
    crates/a4n-embedding-service/src/app.rs \
    crates/a4n-rag-service/src/app.rs \
    crates/a4n-image-service/src/app.rs \
    crates/a4n-video-service/src/main.rs \
    crates/a4n-import-service/src/main.rs \
    crates/a4n-export-service/src/main.rs; then
    printf 'sidecar resource limits must have one service-protocol owner\n' >&2
    exit 1
fi

rg -q --fixed-strings 'command.env_clear();' crates/a4n-service-manager/src/process.rs || {
    printf 'managed sidecars must start from an empty environment\n' >&2
    exit 1
}
rg -q --fixed-strings 'validate_sidecar_environment' crates/a4n-service-manager/src/config/validation.rs || {
    printf 'managed sidecar config must reject host-secret environment keys\n' >&2
    exit 1
}
if rg -n 'DATABASE_URL|JWT_SECRET|BOOTSTRAP_ADMIN_PASSWORD|PROVIDER_API_KEY' crates/a4n-runtime/src/sidecar_clients/service_manager/environment.rs; then
    printf 'sidecar environment allowlist contains a host database, auth, or provider secret\n' >&2
    exit 1
fi
rg -q --fixed-strings 'SIDECAR_AUTH_HEADER' crates/a4n-capabilities/src/transport.rs || {
    printf 'capability transport must forward sidecar authentication\n' >&2
    exit 1
}
rg -q --fixed-strings 'redact_sensitive_message' crates/a4n-capabilities/src/transport/errors.rs || {
    printf 'capability transport errors must redact URLs and secrets\n' >&2
    exit 1
}
rg -q --fixed-strings 'redact_sensitive_message' crates/a4n-sidecar-bootstrap/src/metrics.rs || {
    printf 'sidecar runtime metrics must redact last-error URLs and secrets\n' >&2
    exit 1
}
rg -q --fixed-strings 'connect_timeout' crates/a4n-capabilities/src/transport.rs || {
    printf 'capability transport must configure a connect timeout\n' >&2
    exit 1
}
rg -q --fixed-strings 'stream_idle_timeout' crates/a4n-capabilities/src/transport.rs || {
    printf 'event stream transport must enforce an idle timeout\n' >&2
    exit 1
}
rg -q --fixed-strings 'GlobalConcurrencyLimitLayer' crates/a4n-sidecar-bootstrap/src/lib.rs || {
    printf 'sidecar concurrency must share one semaphore across routes\n' >&2
    exit 1
}
rg -q --fixed-strings 'must use HTTPS outside loopback' crates/a4n-service-manager/src/config/validation.rs || {
    printf 'non-loopback remote sidecars must require HTTPS\n' >&2
    exit 1
}
rg -q --fixed-strings 'auth_token_env' crates/a4n-service-manager/src/config/contracts.rs || {
    printf 'remote sidecars must declare an independent deployment credential contract\n' >&2
    exit 1
}
rg -q --fixed-strings 'is_host_long_lived_secret_environment_key' crates/a4n-service-manager/src/config/validation.rs || {
    printf 'remote deployment credentials must reject Host long-lived secrets\n' >&2
    exit 1
}
rg -q --fixed-strings 'remote_security_from_env' crates/a4n-runtime/src/sidecar_clients/service_manager/configs.rs || {
    printf 'runtime remote sidecars must load service-scoped deployment credentials\n' >&2
    exit 1
}
rg -q --fixed-strings 'from_pkcs8_pem' crates/a4n-service-manager/src/manager.rs || {
    printf 'remote mTLS client identity loading is missing\n' >&2
    exit 1
}
rg -q --fixed-strings 'notify::recommended_watcher' crates/a4n-service-manager/src/manager.rs || {
    printf 'remote TLS material must have a cross-platform background watcher\n' >&2
    exit 1
}
rg -q --fixed-strings 'validate_egress_scope' crates/a4n-service-protocol/src/egress.rs || {
    printf 'sidecar content egress policy contract is missing\n' >&2
    exit 1
}
rg -q --fixed-strings 'a4n_sidecar_egress_audit' crates/a4n-runtime/src/sidecar_clients/request_context/requests.rs || {
    printf 'trusted-remote content egress must emit a metadata-only audit event\n' >&2
    exit 1
}
rg -q --fixed-strings 'compatibility: Option<ServiceCompatibility>' crates/a4n-service-protocol/src/health/contracts.rs || {
    printf 'health contract must advertise protocol/capability compatibility\n' >&2
    exit 1
}
rg -q --fixed-strings 'negotiate_deployment_version' crates/a4n-service-protocol/src/compatibility.rs || {
    printf 'local atomic and remote rolling version rules are missing\n' >&2
    exit 1
}
rg -q --fixed-strings 'auth_token_configured' crates/a4n-capabilities/src/transport.rs || {
    printf 'sidecar transport debug output must redact authentication material\n' >&2
    exit 1
}

printf 'sidecar authentication, environment, logging, body-limit, timeout, and remote-transport boundaries are present\n'
