#!/usr/bin/env python3
"""Check the real migration on a disposable clone of a Director CLI fixture DB."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import uuid

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--template-database", required=True)
parser.add_argument("--postgres-container", required=True)
parser.add_argument("--migration-binary", type=Path, required=True)
parser.add_argument("--port", type=int, required=True)
parser.add_argument("--report", type=Path, required=True)
args = parser.parse_args()
assert re.fullmatch(r"a4n_agent_[0-9a-f]+", args.template_database), "Use an isolated CLI fixture only"
database = "a4n_agent_migration_" + uuid.uuid4().hex

def docker(*command, **kwargs):
    result = subprocess.run(["docker", "exec", "-i", args.postgres_container, *command], text=True, capture_output=True, timeout=60, **kwargs)
    if result.returncode:
        raise RuntimeError(result.stderr.strip())
    return result

def sql(statement):
    return docker("psql", "-XqAt", "-U", "postgres", "-d", database, "-v", "ON_ERROR_STOP=1", input=statement).stdout

docker("createdb", "-U", "postgres", "--template", args.template_database, database)
try:
    sql("""
DROP TABLE visual_plan_edit_receipt;
DELETE FROM seaql_migrations WHERE version = 'agent_project_binding';
CREATE TABLE migration_expected(thread_id bigint, project_id bigint, chapter_id bigint,
  video boolean, scope jsonb, message_id bigint, message jsonb, run_id bigint, turn_id bigint,
  execution_id bigint, arguments jsonb, result jsonb);
UPDATE agent_thread SET archived_at=now(), status='archived';
DO $$
#variable_conflict use_variable
DECLARE root_id bigint; video_id bigint; chapter_id bigint; target_id bigint;
  thread_id bigint; message_id bigint; turn_id bigint; run_id bigint; execution_id bigint;
  original_thread jsonb; original_message jsonb; original_turn jsonb;
  original_run jsonb; original_execution jsonb; scope jsonb;
BEGIN
  SELECT id INTO STRICT root_id FROM novel_project WHERE project_kind='text' ORDER BY id LIMIT 1;
  SELECT id INTO STRICT video_id FROM novel_project WHERE root_project_id=root_id AND project_kind='video' ORDER BY id LIMIT 1;
  SELECT to_jsonb(t) INTO STRICT original_thread FROM agent_thread t WHERE profile_id='director' ORDER BY id LIMIT 1;
  SELECT to_jsonb(t) INTO STRICT original_message FROM agent_message t WHERE role='user' ORDER BY id LIMIT 1;
  SELECT to_jsonb(t) INTO STRICT original_turn FROM agent_turn t ORDER BY id LIMIT 1;
  SELECT to_jsonb(t) INTO STRICT original_run FROM agent_run t ORDER BY id LIMIT 1;
  SELECT to_jsonb(t) INTO STRICT original_execution FROM agent_execution t WHERE execution_kind='tool_call' ORDER BY id LIMIT 1;
  FOREACH target_id IN ARRAY ARRAY[root_id, video_id] LOOP
    SELECT id INTO STRICT chapter_id FROM chapter WHERE project_id=root_id
      AND chapter_number=CASE WHEN target_id=root_id THEN 2 ELSE 1 END;
    thread_id := nextval(pg_get_serial_sequence('agent_thread','id'));
    message_id := nextval(pg_get_serial_sequence('agent_message','id'));
    turn_id := nextval(pg_get_serial_sequence('agent_turn','id'));
    run_id := nextval(pg_get_serial_sequence('agent_run','id'));
    execution_id := nextval(pg_get_serial_sequence('agent_execution','id'));
    scope := jsonb_build_object('project_id',target_id,'chapter_id',chapter_id,
      'allowed_tools',jsonb_build_array('read_chapter_visual_plan'));
    INSERT INTO agent_thread SELECT (jsonb_populate_record(NULL::agent_thread, original_thread ||
      jsonb_build_object('id',thread_id,'project_id',target_id,'resource_type','chapter',
        'resource_id',chapter_id::text,'canonical_binding_key','subagent:director:chapter:'||chapter_id,
        'scope_json',scope,'status','active','archived_at',NULL,'is_default',false))).*;
    INSERT INTO agent_message SELECT (jsonb_populate_record(NULL::agent_message, original_message ||
      jsonb_build_object('id',message_id,'thread_id',thread_id,'turn_id',NULL,'run_id',NULL,
        'content','Preserve this legacy message'))).*;
    INSERT INTO agent_turn SELECT (jsonb_populate_record(NULL::agent_turn, original_turn ||
      jsonb_build_object('id',turn_id,'thread_id',thread_id,'input_message_id',message_id,
        'final_message_id',NULL,'status','active','client_message_id','migration-'||thread_id))).*;
    INSERT INTO agent_run SELECT (jsonb_populate_record(NULL::agent_run, original_run ||
      jsonb_build_object('id',run_id,'thread_id',thread_id,'turn_id',turn_id,
        'root_run_id',NULL,'parent_run_id',NULL,'status','waiting','waiting_reason','external',
        'is_active',true,'completed_at',NULL,'worker_id',NULL,'lease_token',NULL,'lease_expires_at',NULL))).*;
    INSERT INTO agent_execution SELECT (jsonb_populate_record(NULL::agent_execution, original_execution ||
      jsonb_build_object('id',execution_id,'run_id',run_id,'status','running','execution_key','legacy-pending'))).*;
    UPDATE agent_message SET turn_id=turn_id, run_id=run_id WHERE id=message_id;
    INSERT INTO migration_expected SELECT thread_id,target_id,chapter_id,target_id=video_id,scope,
      m.id,to_jsonb(m),run_id,turn_id,e.id,e.arguments_json,e.result_json
      FROM agent_message m, agent_execution e WHERE m.id=message_id AND e.id=execution_id;
  END LOOP;
END $$;
""")
    env = dict(os.environ, A4N_DATABASE_URL=f"postgresql://postgres@127.0.0.1:{args.port}/{database}")
    subprocess.run([str(args.migration_binary.resolve())], env=env, check=True, capture_output=True, timeout=60)
    result = json.loads(sql("""
SELECT json_build_object(
 'count',count(*),
 'messages_preserved',bool_and(to_jsonb(m)=x.message),
 'payloads_preserved',bool_and(e.arguments_json IS NOT DISTINCT FROM x.arguments AND e.result_json IS NOT DISTINCT FROM x.result),
 'restrictions_preserved',bool_and(t.scope_json=x.scope),
 'pending_stopped',bool_and(r.status='interrupted' AND NOT r.is_active AND u.status='failed' AND e.status='outcome_unknown'),
 'identity',bool_and(CASE WHEN x.video THEN t.canonical_binding_key='subagent:v2:director:project:'||x.project_id||':chapter:'||x.chapter_id AND t.archived_at IS NULL
   ELSE t.status='archived' AND t.archived_at IS NOT NULL END))
FROM migration_expected x JOIN agent_thread t ON t.id=x.thread_id
JOIN agent_message m ON m.id=x.message_id JOIN agent_run r ON r.id=x.run_id
JOIN agent_turn u ON u.id=x.turn_id JOIN agent_execution e ON e.id=x.execution_id;
"""))
    assert result.pop("count") == 2 and all(result.values()), result
    args.report.write_text(json.dumps({"passed": True, "checks": result}, ensure_ascii=False, indent=2))
    print(json.dumps(result))
finally:
    docker("dropdb", "-U", "postgres", database)
