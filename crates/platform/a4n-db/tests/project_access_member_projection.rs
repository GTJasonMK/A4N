use a4n_db::collaboration::{
    ProjectAccessInfo, ProjectAccessInfoQuery, ProjectAccessRole, project_access_info_batch,
};
use a4n_db::project::{CreateProjectRecordInput, create_project_record};
use a4n_db_models::models::{project::project_member, user::user};
use chrono::Utc;
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::json;
use std::time::Instant;

const PROJECT_COUNT: usize = 20;
const MEMBER_COUNT: usize = 1_000;
const WARMUPS: usize = 2;
const SAMPLES: usize = 15;

async fn create_fixture(
    db: &DatabaseConnection,
) -> (Vec<ProjectAccessInfoQuery>, i64, Vec<ProjectAccessInfo>) {
    let now = Utc::now().naive_utc();
    let users = (0..=MEMBER_COUNT)
        .map(|index| {
            let username = if index == 0 {
                "project-access-owner".to_string()
            } else {
                format!("project-access-member-{index:04}")
            };
            user::ActiveModel {
                username: Set(username.clone()),
                email: Set(format!("{username}@performance.invalid")),
                password_hash: Set("unused-performance-hash".into()),
                role: Set("user".into()),
                oauth_provider: Set(None),
                oauth_subject: Set(None),
                avatar_url: Set(None),
                page_cli_enabled: Set(false),
                is_active: Set(true),
                lifecycle_gate: Set("active".into()),
                created_at: Set(now),
                updated_at: Set(now),
                last_login_at: Set(None),
                ..Default::default()
            }
        })
        .collect::<Vec<_>>();
    let users =
        user::Entity::insert_many(users.chunks(500).flat_map(|chunk| chunk.iter().cloned()))
            .exec_with_returning_many(db)
            .await
            .expect("benchmark users must be inserted");
    let owner = users
        .iter()
        .find(|user| user.username == "project-access-owner")
        .expect("benchmark owner must be inserted");
    let members = users
        .iter()
        .filter(|user| user.username.starts_with("project-access-member-"))
        .collect::<Vec<_>>();

    let mut queries = Vec::with_capacity(PROJECT_COUNT);
    let mut expected = Vec::with_capacity(PROJECT_COUNT);
    for index in 0..PROJECT_COUNT {
        let project = create_project_record(
            db,
            CreateProjectRecordInput {
                user_id: owner.id,
                title: format!("协作访问性能 {index}"),
                description: String::new(),
                project_type: "novel_long".into(),
            },
        )
        .await
        .expect("benchmark project must be created");
        queries.push(ProjectAccessInfoQuery {
            project_id: project.id,
            root_project_id: None,
        });
        expected.push(ProjectAccessInfo {
            collaboration_root_project_id: project.id,
            owner_username: owner.username.clone(),
            my_role: ProjectAccessRole::Viewer,
            member_count: MEMBER_COUNT as u64,
            pending_invite_count: None,
        });

        let project_members = members
            .iter()
            .take(MEMBER_COUNT)
            .map(|member| project_member::ActiveModel {
                project_id: Set(project.id),
                user_id: Set(member.id),
                role: Set("viewer".into()),
                invited_by_user_id: Set(owner.id),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        for chunk in project_members.chunks(1_000) {
            project_member::Entity::insert_many(chunk.iter().cloned())
                .exec(db)
                .await
                .expect("benchmark memberships must be inserted");
        }
    }
    db.execute_unprepared("ANALYZE project_member")
        .await
        .expect("project member statistics must be refreshed");
    (queries, members[0].id, expected)
}

async fn timed_access_read(
    db: &DatabaseConnection,
    queries: &[ProjectAccessInfoQuery],
    user_id: i64,
) -> (f64, Vec<ProjectAccessInfo>) {
    let started = Instant::now();
    let by_id = project_access_info_batch(db, queries, user_id)
        .await
        .expect("project access summaries must load");
    let rows = queries
        .iter()
        .map(|query| {
            by_id
                .get(&query.project_id)
                .expect("every project must have access information")
                .clone()
        })
        .collect();
    (started.elapsed().as_secs_f64() * 1_000.0, rows)
}

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() - 1) as f64 * percentile).round() as usize;
    sorted[index]
}

#[tokio::test]
#[ignore = "manual PostgreSQL benchmark; requires isolated A4N_TEST_DATABASE_URL"]
async fn benchmark_project_access_member_projection() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let (queries, viewer_id, expected) = create_fixture(&db).await;

    for _ in 0..WARMUPS {
        let (_, result) = timed_access_read(&db, &queries, viewer_id).await;
        assert_eq!(result, expected);
    }
    let mut samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let (elapsed, result) = timed_access_read(&db, &queries, viewer_id).await;
        assert_eq!(result, expected);
        samples.push(elapsed);
    }
    println!(
        "{}",
        json!({
            "projects": PROJECT_COUNT,
            "members_per_project": MEMBER_COUNT,
            "membership_rows_scanned_by_query": PROJECT_COUNT * MEMBER_COUNT,
            "warmups": WARMUPS,
            "samples": SAMPLES,
            "p50_ms": percentile(&samples, 0.50),
            "p95_ms": percentile(&samples, 0.95),
        })
    );
}
