use sea_orm::{ConnectOptions, Database};

const MIGRATION_THREAD_STACK_SIZE: usize = 8 * 1024 * 1024;

fn main() {
    if std::env::args().nth(1).as_deref() == Some("release-data-migrations") {
        match serde_json::to_string(&a4n_migration::migrator::data_migration_names()) {
            Ok(metadata) => println!("{metadata}"),
            Err(error) => {
                eprintln!("生成数据迁移发布元数据失败：{error}");
                std::process::exit(1);
            }
        }
        return;
    }

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

    a4n_config::validate_postgres_database_url("A4N_DATABASE_URL", database_url)
        .map_err(|error| error.to_string())?;

    let db = Database::connect(ConnectOptions::new(database_url))
        .await
        .map_err(|err| format!("连接数据库失败: {err}"))?;
    a4n_migration::migrator::run_migrations(&db)
        .await
        .map_err(|err| format!("数据库迁移失败: {err}"))?;

    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let subcommand = args.get(1).map(String::as_str);
    match args.first().map(String::as_str) {
        Some("apply-data-migrations") => {
            let data_dir = std::env::var_os("A4N_DATA_DIR")
                .map(std::path::PathBuf::from)
                .ok_or_else(|| "数据迁移 apply 需要设置 A4N_DATA_DIR".to_string())?;
            let names = args.iter().skip(1).cloned().collect::<Vec<_>>();
            if names.is_empty() {
                return Err("数据迁移 apply 至少需要一个迁移名称".to_string());
            }
            a4n_migration::migrator::apply_data_migrations(&db, &data_dir, &names)
                .await
                .map_err(|error| format!("应用数据迁移失败：{error}"))?;
        }
        Some("canvas-split") => match subcommand.as_deref() {
            Some("--preflight") => {
                let report = a4n_migration::migrations::canvas_project_split::preflight(&db)
                    .await
                    .map_err(|err| format!("Canvas 拆分预检失败: {err}"))?;
                println!(
                    "Canvas 拆分预检：{} 个旧 freeform 项目，{} 个派生视频含画布内容",
                    report.legacy_freeform_project_ids.len(),
                    report.derived_projects_with_canvas_ids.len()
                );
                println!("旧项目 IDs: {:?}", report.legacy_freeform_project_ids);
                println!(
                    "派生项目 IDs: {:?}",
                    report.derived_projects_with_canvas_ids
                );
            }
            Some("--apply") => {
                let data_dir = std::env::var_os("A4N_DATA_DIR")
                    .map(std::path::PathBuf::from)
                    .ok_or_else(|| "Canvas 拆分 apply 需要设置 A4N_DATA_DIR".to_string())?;
                let report = a4n_migration::migrations::canvas_project_split::apply(&db, &data_dir)
                    .await
                    .map_err(|err| format!("Canvas 拆分执行失败: {err}"))?;
                println!(
                    "Canvas 拆分完成：原位转换 {} 个项目，新建 Canvas 项目 {:?}",
                    report.converted_freeform_projects, report.created_canvas_projects
                );
            }
            Some("--rollback") => {
                let data_dir = std::env::var_os("A4N_DATA_DIR")
                    .map(std::path::PathBuf::from)
                    .ok_or_else(|| "Canvas 拆分 rollback 需要设置 A4N_DATA_DIR".to_string())?;
                let count =
                    a4n_migration::migrations::canvas_project_split::rollback(&db, &data_dir)
                        .await
                        .map_err(|err| format!("Canvas 拆分回滚失败: {err}"))?;
                println!("Canvas 拆分已回滚：处理 {} 条迁移映射", count);
            }
            _ => {
                return Err(
                    "用法：a4n-migration canvas-split --preflight|--apply|--rollback".to_string(),
                );
            }
        },
        Some(command) => return Err(format!("未知迁移命令: {command}")),
        None => println!("数据库迁移完成"),
    }

    Ok(())
}
