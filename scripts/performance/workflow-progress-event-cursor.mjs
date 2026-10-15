import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { gzipSync } from "node:zlib";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

assert.ok(process.env.A4N_TEST_DATABASE_URL);
const url = new URL(process.env.A4N_TEST_DATABASE_URL);
const table = "workflow_progress_event_cursor_probe";
const workflowRunId = 42;
const persistedLimit = 200;
const afterEventId = 200;
const rows = 100_000;
const rounds = 7;
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? "data/performance-audit-20261015/workflow-progress-event-cursor");
const containers = execFileSync("docker", ["ps", "--format", "{{.Names}}\t{{.ID}}\t{{.Ports}}"], { encoding: "utf8" })
  .trim().split("\n").filter((line) => {
    const [name, , ports] = line.split("\t");
    return name.startsWith("a4n-perf-workflow-progress-event-cursor-") && ports?.includes(`127.0.0.1:${url.port}->5432/tcp`);
  });
assert.equal(containers.length, 1);
const container = containers[0].split("\t")[1];

function sql(value) {
  return execFileSync("docker", ["exec", "-i", container, "sh", "-c", 'PGPASSWORD="$POSTGRES_PASSWORD" psql -h 127.0.0.1 -p 5432 -X -q -A -t -v ON_ERROR_STOP=1 -U a4n_test -d a4n_test'], {
    input: value,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  }).trim();
}

sql(`
DROP TABLE IF EXISTS ${table};
CREATE TABLE ${table} (
  id bigint PRIMARY KEY, workflow_run_id bigint NOT NULL, task_id text, event_index integer NOT NULL,
  current integer NOT NULL, total integer NOT NULL, message text NOT NULL, percent real NOT NULL,
  meta_json jsonb, created_at timestamp NOT NULL
);
INSERT INTO ${table}
SELECT id,
  CASE WHEN id BETWEEN 6 AND 205 THEN ${workflowRunId} ELSE (id % 499) + 100 END,
  NULL,
  CASE WHEN id BETWEEN 6 AND 205 THEN id - 5 ELSE ((id - 1) % 200) + 1 END,
  id % 100, 100, 'Progress event ' || id || repeat('m', 128), (id % 100)::real,
  jsonb_build_object('phase_kind', 'canvas_node', 'task_kind', 'workflow', 'detail', repeat('d', 512), 'event_id', id),
  TIMESTAMP '2026-01-01' + id * INTERVAL '1 second'
FROM generate_series(1, ${rows}) AS series(id)
ORDER BY md5(id::text);
CREATE INDEX idx_${table}_run_index ON ${table} (workflow_run_id, event_index);
ANALYZE ${table};
`);

function statement(afterCursor, cursor = afterEventId) {
  const cursorFilter = afterCursor ? ` AND id > ${cursor}` : "";
  const order = afterCursor ? "ASC" : "DESC";
  return `SELECT event_index, id::text AS id, task_id, current, total, message, percent, meta_json AS meta,
    to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.US"Z"') AS timestamp
    FROM ${table} WHERE workflow_run_id = ${workflowRunId}${cursorFilter}
    ORDER BY event_index ${order} LIMIT ${persistedLimit}`;
}

function planNodes(plan) {
  const nodes = [];
  const visit = (node) => { nodes.push(node["Node Type"]); for (const child of node.Plans ?? []) visit(child); };
  visit(plan);
  return nodes;
}

function sample(afterCursor, cursor = afterEventId) {
  const query = statement(afterCursor, cursor);
  const plan = JSON.parse(sql(`SET max_parallel_workers_per_gather = 0; EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON, TIMING OFF) ${query};`))[0];
  const json = sql(`
    WITH latest AS (${query})
    SELECT jsonb_build_object(
      'events', COALESCE(jsonb_agg(jsonb_build_object(
        'id', latest.id, 'current', latest.current, 'total', latest.total,
        'message', latest.message, 'meta', latest.meta, 'timestamp', latest.timestamp
      ) ORDER BY latest.event_index ASC), '[]'::jsonb),
      'stream_text', NULL
    )::text FROM latest;
  `);
  const events = JSON.parse(json).events;
  const parseTimes = [];
  for (let index = 0; index < 9; index += 1) {
    const started = process.hrtime.bigint();
    const parsed = JSON.parse(json);
    const elapsed = Number(process.hrtime.bigint() - started) / 1e6;
    assert.equal(parsed.events.length, events.length);
    if (index >= 2) parseTimes.push(elapsed);
  }
  return {
    sql_ms: plan["Execution Time"],
    nodes: planNodes(plan.Plan),
    event_count: events.length,
    events,
    json_bytes: Buffer.byteLength(json),
    gzip_bytes: gzipSync(json).length,
    parse_p50_ms: [...parseTimes].sort((left, right) => left - right)[Math.floor(parseTimes.length / 2)],
  };
}

const fullSamples = [];
const incrementalSamples = [];
for (let round = 0; round < rounds; round += 1) {
  for (const afterCursor of (round % 2 === 0 ? [false, true] : [true, false])) {
    const result = sample(afterCursor);
    assert.equal(result.event_count, afterCursor ? 5 : 200);
    (afterCursor ? incrementalSamples : fullSamples).push(result);
  }
}

const baselineEvents = fullSamples[0].events;
const unseen = baselineEvents.filter((event) => Number(event.id) > afterEventId);
for (const result of incrementalSamples) {
  assert.deepEqual(result.events, unseen);
}
const median = (values) => [...values].sort((left, right) => left - right)[Math.floor(values.length / 2)];
const summarize = (samples) => ({
  sql_p50_ms: median(samples.map(({ sql_ms }) => sql_ms)),
  parse_p50_ms: median(samples.map(({ parse_p50_ms }) => parse_p50_ms)),
  event_count: samples[0].event_count,
  json_bytes: samples[0].json_bytes,
  gzip_bytes: samples[0].gzip_bytes,
  plan: samples[0].nodes,
  sql_samples_ms: samples.map(({ sql_ms }) => sql_ms),
  parse_samples_ms: samples.map(({ parse_p50_ms }) => parse_p50_ms),
});
const full = summarize(fullSamples);
const incremental = summarize(incrementalSamples);
const idle = sample(true, 205);
const visibleSha256 = createHash("sha256").update(JSON.stringify(unseen)).digest("hex");
const report = {
  table_rows: rows,
  events_retained_for_run: persistedLimit,
  after_event_id: afterEventId,
  new_events: unseen.length,
  rounds,
  parse_warmups: 2,
  parse_samples_per_query: 7,
  full,
  incremental,
  idle: {
    event_count: idle.event_count,
    json_bytes: idle.json_bytes,
    sql_ms: idle.sql_ms,
  },
  reductions: {
    sql_percent: Number(((1 - incremental.sql_p50_ms / full.sql_p50_ms) * 100).toFixed(2)),
    parse_percent: Number(((1 - incremental.parse_p50_ms / full.parse_p50_ms) * 100).toFixed(2)),
    json_percent: Number(((1 - incremental.json_bytes / full.json_bytes) * 100).toFixed(2)),
    gzip_percent: Number(((1 - incremental.gzip_bytes / full.gzip_bytes) * 100).toFixed(2)),
  },
  unseen_sha256: visibleSha256,
};
await mkdir(artifacts, { recursive: true });
await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
