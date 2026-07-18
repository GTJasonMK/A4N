use sea_orm::{ConnectOptions, Database};

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let database_url = match std::env::args().nth(1) {
        Some(value) => value,
        None => match std::env::var("DATABASE_URL") {
            Ok(value) => value,
            Err(std::env::VarError::NotPresent) => {
                return Err("缺少数据库连接 URL：请传入参数或设置 DATABASE_URL".to_string());
            }
            Err(error) => return Err(format!("读取 DATABASE_URL 失败：{error}")),
        },
    };
    let database_url = database_url.trim();
    if database_url.is_empty() {
        return Err("数据库连接 URL 不能为空".to_string());
    }

    if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
        return Err("数据库连接 URL 必须是 postgres:// 或 postgresql://".to_string());
    }

    let db = Database::connect(ConnectOptions::new(database_url))
        .await
        .map_err(|err| format!("连接数据库失败: {err}"))?;
    a4n_migration::migrator::run_migrations(&db)
        .await
        .map_err(|err| format!("数据库迁移失败: {err}"))?;

    println!("数据库迁移完成");
    Ok(())
}
