use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbErr, Statement};
use std::sync::atomic::{AtomicU64, Ordering};

static SCHEMA_COUNTER: AtomicU64 = AtomicU64::new(0);

pub async fn connect_test_database() -> DatabaseConnection {
    connect_test_database_result()
        .await
        .expect("PostgreSQL 测试数据库初始化失败")
}

pub async fn connect_test_database_with_url() -> (DatabaseConnection, String) {
    let base_database_url = std::env::var("A4N_TEST_DATABASE_URL")
        .expect("数据库测试必须显式设置 A4N_TEST_DATABASE_URL");
    let db = connect_test_database().await;
    let schema = db
        .query_one(Statement::from_string(
            db.get_database_backend(),
            "SELECT current_schema() AS schema".to_string(),
        ))
        .await
        .expect("读取 PostgreSQL 测试 schema 失败")
        .expect("PostgreSQL 测试 schema 查询未返回结果")
        .try_get::<String>("", "schema")
        .expect("解析 PostgreSQL 测试 schema 失败");
    let separator = if base_database_url.contains('?') {
        '&'
    } else {
        '?'
    };
    (
        db,
        format!("{base_database_url}{separator}options[search_path]={schema}"),
    )
}

async fn connect_test_database_result() -> Result<DatabaseConnection, DbErr> {
    let database_url = std::env::var("A4N_TEST_DATABASE_URL").map_err(|_| {
        DbErr::Custom("缺少 A4N_TEST_DATABASE_URL，数据库测试必须显式指定 PostgreSQL 测试库".into())
    })?;
    a4n_config::validate_postgres_database_url("A4N_TEST_DATABASE_URL", &database_url)
        .map_err(|error| DbErr::Custom(error.to_string()))?;

    let schema = format!(
        "a4n_test_{}_{}",
        std::process::id(),
        SCHEMA_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let admin_db = Database::connect(&database_url).await?;
    admin_db
        .execute(Statement::from_string(
            admin_db.get_database_backend(),
            format!(r#"CREATE SCHEMA "{schema}""#),
        ))
        .await?;

    let mut opts = ConnectOptions::new(database_url);
    opts.set_schema_search_path(schema);
    let db = Database::connect(opts).await?;
    crate::migrator::run_migrations(&db).await?;
    Ok(db)
}
