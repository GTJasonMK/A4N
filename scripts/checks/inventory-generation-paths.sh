#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../.." && pwd)"
manifest_path="${repo_root}/scripts/checks/generation-path-inventory.txt"
mode="report"

usage() {
  cat <<'EOF'
Usage: scripts/checks/inventory-generation-paths.sh [--check] [--manifest PATH]

Without --check, prints the current production generation-path inventory as TSV.
With --check, compares discovered category/path keys with the manifest machine block.
EOF
}

while (($# > 0)); do
  case "$1" in
    --check)
      mode="check"
      shift
      ;;
    --manifest)
      if (($# < 2)); then
        echo "error: --manifest requires a path" >&2
        exit 2
      fi
      manifest_path="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if ! command -v rg >/dev/null 2>&1; then
  echo "error: rg is required" >&2
  exit 2
fi

tmp_dir="$(mktemp -d)"
cleanup() {
  rm -rf -- "${tmp_dir}"
}
trap cleanup EXIT

raw_matches="${tmp_dir}/matches.tsv"
inventory="${tmp_dir}/inventory.tsv"
: >"${raw_matches}"

common_globs=(
  --glob '*.rs'
  --glob '*.ts'
  --glob '*.tsx'
  --glob '!**/tests/**'
  --glob '!**/fixtures/**'
  --glob '!**/*test*.rs'
  --glob '!**/*.test.ts'
  --glob '!**/*.test.tsx'
  --glob '!crates/a4n-test-support/**'
  --glob '!frontend/src/api/generated/**'
)

rust_source_roots=()
for source_root in "${repo_root}"/crates/a4n-*/src; do
  [[ -d "${source_root}" ]] || continue
  rust_source_roots+=("${source_root#"${repo_root}/"}")
done

scan() {
  local category="$1"
  local pattern="$2"
  shift 2

  local output
  local status
  set +e
  output="$(cd "${repo_root}" && rg -n --no-heading --color never "${common_globs[@]}" "${pattern}" "$@")"
  status=$?
  set -e

  if ((status > 1)); then
    echo "error: rg failed for category ${category}" >&2
    exit "$status"
  fi
  if ((status == 1)); then
    return
  fi

  awk -F ':' -v category="${category}" '
    NF >= 2 {
      print category "\t" $1 "\t" $2
    }
  ' <<<"${output}" >>"${raw_matches}"
}

scan "llm_stream" '\bchat_stream\s*\(' "${rust_source_roots[@]}"
scan "llm_full_response" '\bchat_llm\s*\(|\.chat\s*\(' "${rust_source_roots[@]}"
scan "structured_stream" 'structured_stream|chat_json_stream(_result)?_with_(progress|sink)' "${rust_source_roots[@]}"
scan "manual_stream_collector" 'collect_stream|while[[:space:]]+let[[:space:]]+Some\(.*\.next\(\)\.await' "${rust_source_roots[@]}"
scan "service_task_event" 'ServiceTaskEvent' "${rust_source_roots[@]}"
scan "legacy_sse_progress" '\bSseEvent\b|\bProgressTaskKind\b|emit_chunk' "${rust_source_roots[@]}"
scan "frontend_workflow_event" '\bWorkflowEvent\b|workflow:\$\{|createProgressTaskId' frontend/src
scan "media_provider_job" 'provider_job_id|\bjob_id\b|polling|poll_[a-zA-Z0-9_]*\(' \
  crates/a4n-orchestrator/src/video \
  crates/a4n-orchestrator/src/portrait.rs \
  crates/a4n-orchestrator/src/manga \
  crates/a4n-orchestrator/src/galgame \
  crates/a4n-runtime/src/sidecar_clients \
  crates/a4n-runtime/src/actions

{
  printf 'category\tpath\tmatch_count\tfirst_lines\n'
  sort -t $'\t' -k1,1 -k2,2 -k3,3n "${raw_matches}" | awk -F '\t' '
    function flush() {
      if (key == "") {
        return
      }
      print category "\t" path "\t" count "\t" lines
    }
    {
      current = $1 SUBSEP $2
      if (current != key) {
        flush()
        key = current
        category = $1
        path = $2
        count = 0
        lines = ""
      }
      count++
      if (count <= 8) {
        lines = lines (lines == "" ? "" : ",") $3
      }
    }
    END {
      flush()
    }
  '
} >"${inventory}"

if [[ "${mode}" == "report" ]]; then
  cat "${inventory}"
  exit 0
fi

if [[ ! -f "${manifest_path}" ]]; then
  echo "error: generation inventory manifest not found: ${manifest_path}" >&2
  exit 2
fi

actual_keys="${tmp_dir}/actual.keys"
declared_keys="${tmp_dir}/declared.keys"
missing_keys="${tmp_dir}/missing.keys"
stale_keys="${tmp_dir}/stale.keys"

awk -F '\t' 'NR > 1 { print $1 "|" $2 }' "${inventory}" | sort -u >"${actual_keys}"
awk '/^[a-z_]+\|[^|]+$/ { print }' "${manifest_path}" | sort -u >"${declared_keys}"

comm -23 "${actual_keys}" "${declared_keys}" >"${missing_keys}"
comm -13 "${actual_keys}" "${declared_keys}" >"${stale_keys}"

actual_count="$(wc -l <"${actual_keys}" | tr -d ' ')"
declared_count="$(wc -l <"${declared_keys}" | tr -d ' ')"
missing_count="$(wc -l <"${missing_keys}" | tr -d ' ')"
stale_count="$(wc -l <"${stale_keys}" | tr -d ' ')"

printf 'generation inventory: actual=%s declared=%s missing=%s stale=%s\n' \
  "${actual_count}" "${declared_count}" "${missing_count}" "${stale_count}"

if ((missing_count > 0)); then
  echo "unclassified generation paths:" >&2
  sed 's/^/  /' "${missing_keys}" >&2
fi
if ((stale_count > 0)); then
  echo "stale generation inventory declarations:" >&2
  sed 's/^/  /' "${stale_keys}" >&2
fi

if ((missing_count > 0 || stale_count > 0)); then
  exit 1
fi
