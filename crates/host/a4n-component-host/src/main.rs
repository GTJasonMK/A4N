#[cfg(test)]
use a4n_process_catalog::{service_catalog, stateful_worker_catalog};
use anyhow::Context;

type Runner = fn() -> anyhow::Result<()>;

fn main() -> anyhow::Result<()> {
    let (kind, name) = parse_selector(std::env::args().skip(1))?;
    let run = match kind.as_str() {
        "--worker" => worker_runner(&name),
        "--service" => service_runner(&name),
        _ => None,
    }
    .with_context(|| format!("未知组件入口: {kind} {name}"))?;
    run()
}

fn parse_selector(mut args: impl Iterator<Item = String>) -> anyhow::Result<(String, String)> {
    let kind = args.next().ok_or_else(usage)?;
    let name = args.next().ok_or_else(usage)?;
    if args.next().is_some() || !matches!(kind.as_str(), "--worker" | "--service") {
        return Err(usage());
    }
    Ok((kind, name))
}

fn usage() -> anyhow::Error {
    anyhow::anyhow!("用法: a4n-component-host --worker <worker-name> | --service <service-name>")
}

#[cfg(feature = "stateful-workers")]
fn worker_runner(name: &str) -> Option<Runner> {
    Some(match name {
        "admin-worker" => a4n_admin_worker::run,
        "agent-worker" => a4n_agent_worker::run,
        "novel-planning-worker" => a4n_novel_planning_worker::run,
        "artifact-media-worker" => a4n_artifact_media_worker::run,
        "outline-worker" => a4n_outline_worker::run,
        "blueprint-worker" => a4n_blueprint_worker::run,
        "asset-graph-worker" => a4n_asset_graph_worker::run,
        "canvas-record-worker" => a4n_canvas_record_worker::run,
        "foreshadowing-worker" => a4n_foreshadowing_worker::run,
        "object-profile-worker" => a4n_object_profile_worker::run,
        "scene-profile-worker" => a4n_scene_profile_worker::run,
        "profile-material-worker" => a4n_profile_material_worker::run,
        "project-status-worker" => a4n_project_status_worker::run,
        "portrait-worker" => a4n_portrait_worker::run,
        "source-snapshot-worker" => a4n_source_snapshot_worker::run,
        "theme-read-worker" => a4n_theme_read_worker::run,
        "project-read-worker" => a4n_project_read_worker::run,
        "import-review-read-worker" => a4n_import_review_worker::run,
        "inspiration-history-worker" => a4n_inspiration_worker::run,
        "project-access-worker" => a4n_project_access_worker::run,
        "project-collaboration-worker" => a4n_project_collaboration_worker::run,
        "project-mutation-worker" => a4n_project_mutation_worker::run,
        "chapter-scene-worker" => a4n_chapter_scene_worker::run,
        "chapter-content-worker" => a4n_chapter_content_worker::run,
        "workflow-read-worker" => a4n_workflow_read_worker::run,
        "workflow-lifecycle-worker" => a4n_workflow_lifecycle_worker::run,
        "config-read-worker" => a4n_config_read_worker::run,
        "video-record-worker" => a4n_video_record_worker::run,
        "manga-record-worker" => a4n_manga_record_worker::run,
        "character-mutation-worker" => a4n_character_mutation_worker::run,
        _ => return None,
    })
}

#[cfg(not(feature = "stateful-workers"))]
fn worker_runner(_: &str) -> Option<Runner> {
    None
}

fn service_runner(name: &str) -> Option<Runner> {
    Some(match name {
        #[cfg(feature = "service_prompt")]
        "prompt" => a4n_prompt_service::run,
        #[cfg(feature = "service_llm")]
        "llm" => a4n_llm_gateway::run,
        #[cfg(feature = "service_embedding")]
        "embedding" => a4n_embedding_service::run,
        #[cfg(feature = "service_rag")]
        "rag" => a4n_rag_service::run,
        #[cfg(feature = "service_image")]
        "image" => a4n_image_service::run,
        #[cfg(feature = "service_video")]
        "video" => a4n_video_service::run,
        #[cfg(feature = "service_import")]
        "import" => a4n_import_service::run,
        #[cfg(feature = "service_export")]
        "export" => a4n_export_service::run,
        #[cfg(feature = "service_media")]
        "media" => a4n_media_service::run,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        parse_selector, service_catalog, service_runner, stateful_worker_catalog, worker_runner,
    };

    #[test]
    fn every_catalog_process_has_a_dispatch_entry() {
        for worker in stateful_worker_catalog().unwrap() {
            assert!(worker_runner(&worker.name).is_some(), "{}", worker.name);
        }
        for service in service_catalog().unwrap() {
            assert!(service_runner(&service.name).is_some(), "{}", service.name);
        }
    }

    #[test]
    fn selector_requires_exactly_one_known_mode_and_name() {
        assert!(parse_selector(Vec::<String>::new().into_iter()).is_err());
        assert!(parse_selector(["--worker".to_string()].into_iter()).is_err());
        assert!(parse_selector(["--other".to_string(), "x".to_string()].into_iter()).is_err());
        assert!(
            parse_selector(
                [
                    "--worker".to_string(),
                    "agent-worker".to_string(),
                    "extra".to_string()
                ]
                .into_iter()
            )
            .is_err()
        );
    }
}
