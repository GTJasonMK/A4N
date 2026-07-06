use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbErr, Statement};
use std::sync::atomic::{AtomicU64, Ordering};

static SCHEMA_COUNTER: AtomicU64 = AtomicU64::new(0);

pub async fn connect_test_database() -> DatabaseConnection {
    connect_test_database_result()
        .await
        .expect("PostgreSQL 测试数据库初始化失败")
}

async fn connect_test_database_result() -> Result<DatabaseConnection, DbErr> {
    let database_url = std::env::var("A4N_TEST_DATABASE_URL").map_err(|_| {
        DbErr::Custom("缺少 A4N_TEST_DATABASE_URL，数据库测试必须显式指定 PostgreSQL 测试库".into())
    })?;
    if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
        return Err(DbErr::Custom(
            "A4N_TEST_DATABASE_URL 必须是 postgres:// 或 postgresql:// 连接串".into(),
        ));
    }

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
