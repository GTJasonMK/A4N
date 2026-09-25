//! Upgrade persisted Agent history once; readers only accept the current contract.

use sea_orm::{ConnectionTrait, DbBackend, DbErr};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Agent history migration requires PostgreSQL".into(),
            ));
        }
        manager.get_connection().execute_unprepared(r#"
            UPDATE agent_thread SET context_summary = NULL
            WHERE CASE WHEN pg_input_is_valid(context_summary, 'jsonb') THEN
                jsonb_typeof(context_summary::jsonb) = 'object'
                AND NOT (context_summary::jsonb ? 'schema_version')
            ELSE false END;

            UPDATE agent_message AS message
            SET payload_json = jsonb_set(message.payload_json, '{editor_source,chapter_id}',
                                          thread.scope_json -> 'chapter_id')
            FROM agent_thread AS thread
            WHERE message.thread_id = thread.id
              AND jsonb_typeof(message.payload_json -> 'editor_source') = 'object'
              AND NOT (message.payload_json -> 'editor_source' ? 'chapter_id')
              AND jsonb_typeof(thread.scope_json -> 'chapter_id') = 'number'
              AND thread.scope_json ->> 'chapter_id' ~ '^[1-9][0-9]*$';

            UPDATE agent_event AS event
            SET payload_json = jsonb_set(event.payload_json, '{editor_request,source,chapter_id}',
                                          thread.scope_json -> 'chapter_id')
            FROM agent_thread AS thread
            WHERE event.thread_id = thread.id
              AND jsonb_typeof(event.payload_json #> '{editor_request,source}') = 'object'
              AND NOT (event.payload_json #> '{editor_request,source}' ? 'chapter_id')
              AND jsonb_typeof(thread.scope_json -> 'chapter_id') = 'number'
              AND thread.scope_json ->> 'chapter_id' ~ '^[1-9][0-9]*$';

            UPDATE agent_event AS event
            SET payload_json = jsonb_set(event.payload_json, '{message,metadata,editor_source}',
                                          message.payload_json -> 'editor_source')
            FROM agent_message AS message
            WHERE event.message_id = message.id
              AND jsonb_typeof(event.payload_json #> '{message,metadata,editor_source}') = 'object'
              AND jsonb_typeof(message.payload_json -> 'editor_source') = 'object';

            WITH legacy AS (
                SELECT id, jsonb_build_object(
                    'source_kind', sender_kind,
                    'inbox_kind', payload_json ->> 'inbox_kind',
                    'content', content,
                    'payload', payload_json -> 'payload'
                )::text AS content
                FROM agent_message
                WHERE role = 'user' AND payload_json ->> 'inbox_kind' = 'child_result'
            ), upgraded AS (
                UPDATE agent_message AS message
                SET role = 'assistant', content = legacy.content,
                    content_sha256 = encode(sha256(convert_to(legacy.content, 'UTF8')), 'hex')
                FROM legacy WHERE message.id = legacy.id
                RETURNING message.id, message.content
            )
            UPDATE agent_event AS event
            SET payload_json = jsonb_set(
                jsonb_set(event.payload_json, '{message,role}', '"assistant"'::jsonb),
                '{message,content}', to_jsonb(upgraded.content)
            )
            FROM upgraded
            WHERE event.message_id = upgraded.id
              AND jsonb_typeof(event.payload_json -> 'message') = 'object';

            ALTER TABLE agent_message DROP CONSTRAINT IF EXISTS ck_agent_child_result_role;
            ALTER TABLE agent_message ADD CONSTRAINT ck_agent_child_result_role
                CHECK (role <> 'user' OR payload_json ->> 'inbox_kind' IS DISTINCT FROM 'child_result');
        "#).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom("Agent history canonicalization cannot restore obsolete summaries or user authority for child results".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_db_models::models::agent::{event, message, thread};
    use sea_orm::EntityTrait;
    use serde_json::{Value, json};

    #[tokio::test]
    async fn upgrades_legacy_history_without_hiding_corruption_or_changing_current_summaries() {
        let db = crate::test_support::connect_test_database().await;
        db.execute_unprepared(r#"
            ALTER TABLE agent_message DROP CONSTRAINT ck_agent_child_result_role;
            INSERT INTO agent_thread
                (id, owner_user_id, thread_kind, profile_id, scope_json, status, is_default,
                 revision, next_turn_sequence, next_event_sequence, context_summary, created_at, updated_at)
            VALUES
                (1, 1, 'main', 'main', '{"chapter_id":7}', 'active', false, 1, 0, 2, '{"current_goal":"legacy"}', NOW(), NOW()),
                (2, 1, 'main', 'main', '{}', 'active', false, 1, 0, 0, '{"schema_version":2}', NOW(), NOW()),
                (3, 1, 'main', 'main', '{}', 'active', false, 1, 0, 0, 'not json', NOW(), NOW());
            INSERT INTO agent_message
                (id, thread_id, role, sender_kind, content_sha256, content, status, payload_json, created_at, updated_at)
            VALUES
                (1, 1, 'user', 'subagent', repeat('0',64), '原始子任务结果', 'complete',
                 '{"inbox_kind":"child_result","payload":{"child_run_id":"9"}}', NOW(), NOW()),
                (2, 1, 'user', 'user', repeat('0',64), '真实用户指令', 'complete',
                 '{"editor_source":{"view_instance_id":"v"}}', NOW(), NOW());
            INSERT INTO agent_event
                (thread_id, sequence, message_id, actor_kind, kind, status, public_text, payload_json, created_at)
            VALUES (1, 1, 1, 'subagent', 'message_appended', 'complete', '原始子任务结果',
                    '{"message":{"role":"user","content":"原始子任务结果"}}', NOW()),
                   (1, 2, 2, 'agent', 'run_waiting', 'waiting', '等待原编辑器',
                    '{"editor_request":{"source":{"view_instance_id":"v"}},"message":{"metadata":{"editor_source":{"view_instance_id":"v"}}}}', NOW());
        "#).await.unwrap();
        let manager = SchemaManager::new(&db);
        Migration.up(&manager).await.unwrap();
        Migration.up(&manager).await.unwrap();
        assert_eq!(
            thread::Entity::find_by_id(1)
                .one(&db)
                .await
                .unwrap()
                .unwrap()
                .context_summary,
            None
        );
        assert_eq!(
            thread::Entity::find_by_id(2)
                .one(&db)
                .await
                .unwrap()
                .unwrap()
                .context_summary
                .as_deref(),
            Some("{\"schema_version\":2}")
        );
        assert_eq!(
            thread::Entity::find_by_id(3)
                .one(&db)
                .await
                .unwrap()
                .unwrap()
                .context_summary
                .as_deref(),
            Some("not json")
        );
        let child = message::Entity::find_by_id(1)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(child.role, "assistant");
        assert_eq!(
            serde_json::from_str::<Value>(&child.content).unwrap()["content"],
            "原始子任务结果"
        );
        assert_eq!(
            child.content_sha256,
            a4n_domain::source_text_hash(&child.content)
        );
        let user = message::Entity::find_by_id(2)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user.content, "真实用户指令");
        assert_eq!(user.role, "user");
        assert_eq!(user.payload_json.unwrap()["editor_source"]["chapter_id"], 7);
        let event = event::Entity::find_by_id(1)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .payload_json
            .unwrap();
        assert_eq!(event["message"]["role"], "assistant");
        assert_eq!(event["message"]["content"], child.content);
        let editor = event::Entity::find_by_id(2)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .payload_json
            .unwrap();
        assert_eq!(editor["editor_request"]["source"]["chapter_id"], 7);
        assert_eq!(
            editor["message"]["metadata"]["editor_source"]["chapter_id"],
            7
        );
        assert!(
            db.execute_unprepared("UPDATE agent_message SET role = 'user' WHERE id = 1")
                .await
                .is_err()
        );
        assert_eq!(
            serde_json::from_str::<Value>(&child.content).unwrap()["payload"],
            json!({"child_run_id":"9"})
        );
    }
}
