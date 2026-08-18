use sea_orm::{ConnectOptions, Database};

const MIGRATION_THREAD_STACK_SIZE: usize = 8 * 1024 * 1024;

fn main() {
    // Windows main threads default to a smaller stack than the baseline migration future needs.
    let result = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .thread_stack_size(MIGRATION_THREAD_STACK_SIZE)
        .enable_all()
        .build()
        .map_err(|err| format!("初始化数据库迁移运行时失败: {err}"))
        .and_then(|runtime| {
            runtime.block_on(async {
                tokio::spawn(run())
                    .await
                    .map_err(|err| format!("数据库迁移任务失败: {err}"))?
            })
        });
    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let database_url = std::env::var("A4N_DATABASE_URL")
        .map_err(|error| format!("读取 A4N_DATABASE_URL 失败：{error}"))?;
    let database_url = database_url.trim();
    if database_url.is_empty() {
        return Err("数据库连接 URL 不能为空".to_string());
    }

    a4n_core::config::env::validate_postgres_database_url("A4N_DATABASE_URL", database_url)
        .map_err(|error| error.to_string())?;

    let db = Database::connect(ConnectOptions::new(database_url))
        .await
        .map_err(|err| format!("连接数据库失败: {err}"))?;
    a4n_migration::migrator::run_migrations(&db)
        .await
        .map_err(|err| format!("数据库迁移失败: {err}"))?;

    println!("数据库迁移完成");
    Ok(())
}
