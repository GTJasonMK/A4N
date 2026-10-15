import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { gzipSync } from "node:zlib";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

assert.ok(process.env.A4N_TEST_DATABASE_URL);
const url = new URL(process.env.A4N_TEST_DATABASE_URL);
const table = "workflow_run_snapshot_projection_probe";
const targetId = 42;
const rows = 100_000;
const rounds = 7;
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? "data/performance-audit-20261015/workflow-run-snapshot-projection");
const containers = execFileSync("docker", ["ps", "--format", "{{.Names}}\t{{.ID}}\t{{.Ports}}"], { encoding: "utf8" })
  .trim().split("\n").filter((line) => {
    const [name, , ports] = line.split("\t");
    return name.startsWith("a4n-perf-workflow-run-snapshot-projection-") && ports?.includes(`127.0.0.1:${url.port}->5432/tcp`);
  });
assert.equal(containers.length, 1);
const container = containers[0].split("\t")[1];

function sql(value) {
  return execFileSync("docker", ["exec", "-i", container, "sh", "-c", 'PGPASSWORD="$POSTGRES_PASSWORD" psql -h 127.0.0.1 -p 5432 -X -q -A -t -v ON_ERROR_STOP=1 -U a4n_test -d a4n_test'], {
    input: value,
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
  }).trim();
}

sql(`
DROP TABLE IF EXISTS ${table};
CREATE TABLE ${table} (
  id bigint PRIMARY KEY, business_key text NOT NULL, workflow_kind text NOT NULL,
  project_id bigint, root_project_id bigint, source_snapshot_id bigint, scope_table text,
  scope_id bigint, chapter_id bigint, mode text, input_fingerprint_json jsonb,
  status text NOT NULL, current_step_key text, progress_json jsonb, active_task_id text,
  can_resume boolean NOT NULL, resume_block_reason text, error_message text,
  created_at timestamp NOT NULL, updated_at timestamp NOT NULL, completed_at timestamp
);
INSERT INTO ${table}
SELECT id, 'workflow-' || id, 'video_canvas_graph', 7, 7, NULL, 'canvas_graph', 42, NULL, NULL,
  jsonb_build_object('graph_version', 9), 'running', 'canvas_node:8',
  CASE WHEN id = ${targetId} THEN (
    SELECT jsonb_build_object(
      'current', 1, 'total', 100, 'message', '正在运行画布', 'percent', 1.0,
      'meta', NULL, 'updated_at', '2026-10-15T10:00:00Z',
      'snapshot', jsonb_build_object('units', jsonb_agg(jsonb_build_object(
        'index', item,
        'content', md5(item::text) || md5((item + 3000)::text) || md5((item + 6000)::text) || md5((item + 9000)::text) ||
          md5((item + 12000)::text) || md5((item + 15000)::text) || md5((item + 18000)::text) || md5((item + 21000)::text)
      )))
    ) FROM generate_series(1, 3000) AS data(item)
  ) ELSE jsonb_build_object(
    'current', 1, 'total', 100, 'message', '正在运行画布', 'percent', 1.0,
    'meta', NULL, 'updated_at', '2026-10-15T10:00:00Z'
  ) END,
  NULL, TRUE, NULL, NULL,
  TIMESTAMP '2026-10-15 10:00:00', TIMESTAMP '2026-10-15 10:00:01', NULL
FROM generate_series(1, ${rows}) AS run(id)
ORDER BY md5(id::text);
ANALYZE ${table};
`);

const columns = `id, business_key, workflow_kind, project_id, root_project_id, source_snapshot_id,
  scope_table, scope_id, chapter_id, mode, input_fingerprint_json, status, current_step_key,
  active_task_id, can_resume, resume_block_reason, error_message, created_at, updated_at, completed_at`;
const fullQuery = `SELECT * FROM ${table} WHERE id = ${targetId}`;
const summaryQuery = `SELECT ${columns}, progress_json FROM ${table} WHERE id = ${targetId}`;
const withoutSnapshotQuery = `SELECT ${columns},
  CASE
    WHEN progress_json IS NULL OR jsonb_typeof(progress_json) <> 'object' THEN progress_json
    ELSE progress_json - 'snapshot' - 'cancellation_active_task_id'
  END AS progress_json,
  (progress_json ? 'snapshot') AS has_snapshot,
  (progress_json ? 'snapshot' AND jsonb_typeof(progress_json->'snapshot') IS DISTINCT FROM 'object') AS invalid_snapshot
  FROM ${table} WHERE id = ${targetId}`;
const publicProgressQuery = `SELECT ${columns},
  CASE
    WHEN progress_json IS NULL OR jsonb_typeof(progress_json) <> 'object' THEN progress_json
    ELSE jsonb_build_object(
      'current', progress_json->'current',
      'total', progress_json->'total',
      'message', progress_json->'message',
      'percent', progress_json->'percent',
      'meta', progress_json->'meta',
      'updated_at', progress_json->'updated_at'
    )
  END AS progress_json,
  (progress_json ? 'snapshot') AS has_snapshot,
  (progress_json ? 'snapshot' AND jsonb_typeof(progress_json->'snapshot') IS DISTINCT FROM 'object') AS invalid_snapshot
  FROM ${table} WHERE id = ${targetId}`;

function publicSummary(row) {
  if (row.invalid_snapshot) throw new Error("progress_json.snapshot must remain an object");
  const progressValue = row.progress_json;
  let progress = null;
  if (progressValue && typeof progressValue === "object" && !Array.isArray(progressValue)) {
    const fields = Object.fromEntries(Object.entries(progressValue).filter(([key]) => key !== "snapshot" && key !== "cancellation_active_task_id"));
    if (Object.keys(fields).length > 0) {
      progress = {
        current: fields.current,
        total: fields.total,
        message: fields.message,
        percent: fields.percent,
        meta: fields.meta,
        updated_at: fields.updated_at,
      };
    }
  }
  return {
    id: row.id,
    business_key: row.business_key,
    workflow_kind: row.workflow_kind,
    project_id: row.project_id,
    root_project_id: row.root_project_id,
    source_snapshot_id: row.source_snapshot_id,
    scope_table: row.scope_table,
    scope_id: row.scope_id,
    chapter_id: row.chapter_id,
    mode: row.mode,
    input_fingerprint_json: row.input_fingerprint_json,
    status: row.status,
    current_step_key: row.current_step_key,
    progress,
    active_task_id: row.active_task_id,
    can_resume: row.can_resume,
    resume_block_reason: row.resume_block_reason,
    error_message: row.error_message,
    created_at: row.created_at,
    updated_at: row.updated_at,
    completed_at: row.completed_at,
  };
}

function planNodes(plan) {
  const nodes = [];
  const visit = (node) => { nodes.push(node["Node Type"]); for (const child of node.Plans ?? []) visit(child); };
  visit(plan);
  return nodes;
}

function sample(kind) {
  const query = kind === "full" ? fullQuery
    : kind === "summary" ? summaryQuery
      : kind === "display" ? withoutSnapshotQuery
        : publicProgressQuery;
  const started = process.hrtime.bigint();
  const raw = sql(`SELECT row_to_json(result)::text FROM (${query}) AS result;`);
  const queryRoundtripMs = Number(process.hrtime.bigint() - started) / 1e6;
  const row = JSON.parse(raw);
  const publicJson = JSON.stringify(publicSummary(row));
  const plan = JSON.parse(sql(`EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON, TIMING OFF) ${query};`))[0];
  const parseSamples = [];
  for (let index = 0; index < 9; index += 1) {
    const parseStarted = process.hrtime.bigint();
    const parsed = JSON.parse(raw);
    const elapsed = Number(process.hrtime.bigint() - parseStarted) / 1e6;
    assert.equal(parsed.id, targetId);
    if (index >= 2) parseSamples.push(elapsed);
  }
  parseSamples.sort((left, right) => left - right);
  return {
    query_roundtrip_ms: queryRoundtripMs,
    sql_ms: plan["Execution Time"],
    nodes: planNodes(plan.Plan),
    db_json_bytes: Buffer.byteLength(raw),
    db_gzip_bytes: gzipSync(raw).length,
    parse_p50_ms: parseSamples[Math.floor(parseSamples.length / 2)],
    public_json: publicJson,
    public_sha256: createHash("sha256").update(publicJson).digest("hex"),
  };
}

const results = { full: [], summary: [], display: [], public_progress: [] };
for (let round = 0; round < rounds; round += 1) {
  const order = round % 2 === 0
    ? ["full", "display", "public_progress", "summary"]
    : ["summary", "public_progress", "display", "full"];
  for (const kind of order) results[kind].push(sample(kind));
}
const expected = results.full[0].public_sha256;
for (const samples of Object.values(results)) {
  for (const result of samples) assert.equal(result.public_sha256, expected);
}
const median = (values) => [...values].sort((left, right) => left - right)[Math.floor(values.length / 2)];
const summarize = (samples) => ({
  query_roundtrip_p50_ms: median(samples.map(({ query_roundtrip_ms }) => query_roundtrip_ms)),
  sql_p50_ms: median(samples.map(({ sql_ms }) => sql_ms)),
  parse_p50_ms: median(samples.map(({ parse_p50_ms }) => parse_p50_ms)),
  db_json_bytes: samples[0].db_json_bytes,
  db_gzip_bytes: samples[0].db_gzip_bytes,
  public_sha256: samples[0].public_sha256,
  plan: samples[0].nodes,
  samples: samples.map(({ query_roundtrip_ms, sql_ms, parse_p50_ms }) => ({ query_roundtrip_ms, sql_ms, parse_p50_ms })),
});
const full = summarize(results.full);
const summary = summarize(results.summary);
const display = summarize(results.display);
const publicProgress = summarize(results.public_progress);
const report = {
  table_rows: rows,
  target_id: targetId,
  target_progress_snapshot_db_json_bytes: results.full[0].db_json_bytes - results.display[0].db_json_bytes,
  rounds,
  parse_warmups: 2,
  parse_samples_per_query: 7,
  full_entity: full,
  progress_snapshot_included: summary,
  progress_snapshot_omitted: display,
  public_progress_fields_only: publicProgress,
  reductions_full_to_omitted: {
    query_roundtrip_percent: Number(((1 - display.query_roundtrip_p50_ms / full.query_roundtrip_p50_ms) * 100).toFixed(2)),
    sql_percent: Number(((1 - display.sql_p50_ms / full.sql_p50_ms) * 100).toFixed(2)),
    parse_percent: Number(((1 - display.parse_p50_ms / full.parse_p50_ms) * 100).toFixed(2)),
    db_json_percent: Number(((1 - display.db_json_bytes / full.db_json_bytes) * 100).toFixed(2)),
    gzip_percent: Number(((1 - display.db_gzip_bytes / full.db_gzip_bytes) * 100).toFixed(2)),
  },
};
await mkdir(artifacts, { recursive: true });
await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
