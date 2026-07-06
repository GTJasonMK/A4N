use super::*;
use a4n_core::state::validator::StateValidator;
use a4n_services::canvas::{
    CanvasGraphDocument, validate_canvas_document_edge_contract, validate_canvas_document_is_dag,
    validate_canvas_viewport_json,
};
use a4n_services::media::{
    MEDIA_STATUS_COMPLETED, is_valid_media_binding_role, is_valid_media_binding_target_kind,
    is_valid_media_kind, is_valid_media_origin, is_valid_media_status,
};
use a4n_services::project::normalize_project_type;

mod media;
mod provenance;

#[cfg(test)]
use media::sha256_bytes;
use media::{require_media_payload, validate_optional_file_size};
pub(super) use media::{validate_media_entry_bytes, validate_media_entry_owner};
use provenance::{
    MediaProvenanceSnapshotRefs, validate_media_provenance_snapshot,
    validate_media_provenance_source_external_path,
};

pub(super) fn validate_package_manifest(
    package: &VideoStudioProjectPackage,
    media_payload: &ImportMediaPayload,
) -> Result<(), String> {
    if package.version != VIDEO_STUDIO_PACKAGE_VERSION {
        return Err(format!("视频画布项目包版本无效: {}", package.version));
    }
    if package.mode != PACKAGE_MODE {
        return Err(format!("视频画布项目包模式无效: {}", package.mode));
    }
    validate_freeform_video_project(&package.project, "视频画布项目包")?;
    normalize_project_type(&package.project.project_type).map_err(|err| err.to_string())?;
    StateValidator::parse_status(&package.project.status).map_err(|err| err.to_string())?;

    validate_unique_ids(
        package.canvas_graphs.iter().map(|record| record.id),
        "canvas_graph.id",
    )?;
    validate_unique_ids(
        package.canvas_nodes.iter().map(|record| record.id),
        "canvas_node.id",
    )?;
    validate_unique_ids(
        package.canvas_edges.iter().map(|record| record.id),
        "canvas_edge.id",
    )?;
    validate_unique_ids(
        package.canvas_text_assets.iter().map(|record| record.id),
        "canvas_text_asset.id",
    )?;
    validate_unique_ids(
        package.generation_specs.iter().map(|record| record.id),
        "generation_spec.id",
    )?;
    validate_unique_ids(
        package.prompt_artifacts.iter().map(|record| record.id),
        "prompt_artifact.id",
    )?;
    validate_unique_ids(
        package.generated_images.iter().map(|record| record.id),
        "generated_image.id",
    )?;
    validate_unique_ids(
        package.generated_videos.iter().map(|record| record.id),
        "generated_video.id",
    )?;
    validate_unique_ids(
        package.media_assets.iter().map(|record| record.id),
        "media_asset.id",
    )?;
    validate_unique_ids(
        package.media_bindings.iter().map(|record| record.id),
        "media_binding.id",
    )?;
    validate_unique_ids(
        package.video_segments.iter().map(|record| record.id),
        "video_segment.id",
    )?;
    validate_unique_ids(
        package.assembly_timelines.iter().map(|record| record.id),
        "assembly_timeline.id",
    )?;
    validate_unique_ids(
        package.assembly_clips.iter().map(|record| record.id),
        "assembly_clip.id",
    )?;

    let graph_ids = package
        .canvas_graphs
        .iter()
        .map(|record| record.id)
        .collect::<HashSet<_>>();
    let node_ids = package
        .canvas_nodes
        .iter()
        .map(|record| record.id)
        .collect::<HashSet<_>>();
    let text_asset_ids = package
        .canvas_text_assets
        .iter()
        .map(|record| record.id)
        .collect::<HashSet<_>>();
    let generation_spec_ids = package
        .generation_specs
        .iter()
        .map(|record| record.id)
        .collect::<HashSet<_>>();
    let prompt_artifact_ids = package
        .prompt_artifacts
        .iter()
        .map(|record| record.id)
        .collect::<HashSet<_>>();
    let image_ids = package
        .generated_images
        .iter()
        .map(|record| record.id)
        .collect::<HashSet<_>>();
    let video_ids = package
        .generated_videos
        .iter()
        .map(|record| record.id)
        .collect::<HashSet<_>>();
    let media_asset_ids = package
        .media_assets
        .iter()
        .map(|record| record.id)
        .collect::<HashSet<_>>();
    let video_segment_ids = package
        .video_segments
        .iter()
        .map(|record| record.id)
        .collect::<HashSet<_>>();
    let assembly_timeline_ids = package
        .assembly_timelines
        .iter()
        .map(|record| record.id)
        .collect::<HashSet<_>>();
    let assembly_clip_ids = package
        .assembly_clips
        .iter()
        .map(|record| record.id)
        .collect::<HashSet<_>>();

    for graph in &package.canvas_graphs {
        if graph.project_id != package.project.id {
            return Err(format!(
                "画布图不属于导入包项目: canvas_graph_id={}",
                graph.id
            ));
        }
        if let Err(err) = validate_canvas_viewport_json(
            graph.viewport_json.as_ref(),
            "canvas_graph.viewport_json",
        ) {
            return Err(format!(
                "视频画布项目包画布视口无效: canvas_graph_id={}, {}",
                graph.id, err
            ));
        }
    }
    let node_ref_ids = CanvasNodeRefIds {
        text_asset_ids: &text_asset_ids,
        generation_spec_ids: &generation_spec_ids,
        prompt_artifact_ids: &prompt_artifact_ids,
        image_ids: &image_ids,
        video_ids: &video_ids,
        media_asset_ids: &media_asset_ids,
        video_segment_ids: &video_segment_ids,
        assembly_timeline_ids: &assembly_timeline_ids,
    };
    for node in &package.canvas_nodes {
        if !graph_ids.contains(&node.graph_id) {
            return Err(format!(
                "画布节点引用了不存在的图: canvas_node_id={}",
                node.id
            ));
        }
        validate_canvas_node_ref(node, &node_ref_ids)?;
    }
    for edge in &package.canvas_edges {
        if !graph_ids.contains(&edge.graph_id) {
            return Err(format!(
                "画布连线引用了不存在的图: canvas_edge_id={}",
                edge.id
            ));
        }
        if !node_ids.contains(&edge.source_node_id) || !node_ids.contains(&edge.target_node_id) {
            return Err(format!(
                "画布连线引用了不存在的节点: canvas_edge_id={}",
                edge.id
            ));
        }
    }
    validate_canvas_edge_contracts(package)?;
    for asset in &package.canvas_text_assets {
        if asset.project_id != package.project.id {
            return Err(format!(
                "画布文本资产不属于导入包项目: canvas_text_asset_id={}",
                asset.id
            ));
        }
    }
    for spec in &package.generation_specs {
        if spec.project_id != package.project.id || spec.root_project_id != package.project.id {
            return Err(format!(
                "生成意图不属于导入包项目: generation_spec_id={}",
                spec.id
            ));
        }
        if spec
            .source_context_kind
            .as_deref()
            .unwrap_or("snapshot_backed")
            != SOURCE_MODE_FREEFORM
            || spec.source_snapshot_id.is_some()
            || spec.source_chapter_id.is_some()
            || spec.source_snapshot_chapter_id.is_some()
        {
            return Err(format!(
                "视频画布项目包只允许 freeform 生成意图: generation_spec_id={}",
                spec.id
            ));
        }
        if let Some(asset_id) = spec.canvas_text_asset_id
            && !text_asset_ids.contains(&asset_id)
        {
            return Err(format!(
                "生成意图引用了不存在的画布文本资产: generation_spec_id={}, canvas_text_asset_id={asset_id}",
                spec.id
            ));
        }
        validate_generation_spec_target(
            spec,
            &node_ids,
            &media_asset_ids,
            &video_segment_ids,
            &assembly_timeline_ids,
        )?;
    }
    for artifact in &package.prompt_artifacts {
        if !generation_spec_ids.contains(&artifact.generation_spec_id) {
            return Err(format!(
                "提示词产物引用了不存在的生成意图: prompt_artifact_id={}, generation_spec_id={}",
                artifact.id, artifact.generation_spec_id
            ));
        }
    }
    for image in &package.generated_images {
        if image.project_id != package.project.id || image.chapter_id.is_some() {
            return Err(format!(
                "生成图片不是项目级画布资产: generated_image_id={}",
                image.id
            ));
        }
        validate_optional_file_size(image.file_size, "generated_image", image.id)?;
        require_media_payload(media_payload, MEDIA_OWNER_IMAGE, image.id, MEDIA_FIELD_FILE)?;
    }
    for video in &package.generated_videos {
        if video.project_id != package.project.id || video.chapter_id.is_some() {
            return Err(format!(
                "生成视频不是项目级画布资产: generated_video_id={}",
                video.id
            ));
        }
        validate_optional_file_size(video.file_size, "generated_video", video.id)?;
        if let Some(source_video_id) = video.source_video_id
            && !video_ids.contains(&source_video_id)
        {
            return Err(format!(
                "生成视频引用了不存在的源视频: generated_video_id={}, source_video_id={source_video_id}",
                video.id
            ));
        }
        if let Some(source_image_id) = video.source_image_id
            && !image_ids.contains(&source_image_id)
        {
            return Err(format!(
                "生成视频引用了不存在的源图片: generated_video_id={}, source_image_id={source_image_id}",
                video.id
            ));
        }
        if video.file_path.is_some() {
            require_media_payload(media_payload, MEDIA_OWNER_VIDEO, video.id, MEDIA_FIELD_FILE)?;
        }
        if video.thumbnail_path.is_some() {
            require_media_payload(
                media_payload,
                MEDIA_OWNER_VIDEO,
                video.id,
                MEDIA_FIELD_THUMBNAIL,
            )?;
        }
    }
    for asset in &package.media_assets {
        if asset.project_id != package.project.id || asset.root_project_id != package.project.id {
            return Err(format!(
                "媒体资产不属于导入包 freeform 项目: media_asset_id={}",
                asset.id
            ));
        }
        validate_optional_file_size(asset.file_size, "media_asset", asset.id)?;
        if asset.source_chapter_id.is_some() || asset.source_snapshot_chapter_id.is_some() {
            return Err(format!(
                "视频画布媒体资产不能绑定文本章节: media_asset_id={}",
                asset.id
            ));
        }
        if !is_valid_media_kind(&asset.media_kind) {
            return Err(format!(
                "媒体资产类型无效: media_asset_id={}, media_kind={}",
                asset.id, asset.media_kind
            ));
        }
        if !is_valid_media_origin(&asset.origin) {
            return Err(format!(
                "媒体资产来源无效: media_asset_id={}, origin={}",
                asset.id, asset.origin
            ));
        }
        if !is_valid_media_status(&asset.status) {
            return Err(format!(
                "媒体资产状态无效: media_asset_id={}, status={}",
                asset.id, asset.status
            ));
        }
        if asset.status == MEDIA_STATUS_COMPLETED && asset.file_path.is_none() {
            return Err(format!("completed 媒体资产缺少 file_path: {}", asset.id));
        }
        if asset.file_path.is_some() {
            require_media_payload(
                media_payload,
                MEDIA_OWNER_MEDIA_ASSET,
                asset.id,
                MEDIA_FIELD_FILE,
            )?;
        }
        if asset.thumbnail_path.is_some() {
            require_media_payload(
                media_payload,
                MEDIA_OWNER_MEDIA_ASSET,
                asset.id,
                MEDIA_FIELD_THUMBNAIL,
            )?;
        }
    }
    for provenance in &package.media_provenance {
        if !media_asset_ids.contains(&provenance.media_asset_id) {
            return Err(format!(
                "媒体来源引用了不存在的媒体资产: media_asset_id={}",
                provenance.media_asset_id
            ));
        }
        if let Some(spec_id) = provenance.generation_spec_id
            && !generation_spec_ids.contains(&spec_id)
        {
            return Err(format!(
                "媒体来源引用了不存在的生成意图: media_asset_id={}, generation_spec_id={spec_id}",
                provenance.media_asset_id
            ));
        }
        if let Some(artifact_id) = provenance.prompt_artifact_id
            && !prompt_artifact_ids.contains(&artifact_id)
        {
            return Err(format!(
                "媒体来源引用了不存在的提示词产物: media_asset_id={}, prompt_artifact_id={artifact_id}",
                provenance.media_asset_id
            ));
        }
        if let Some(source_asset_id) = provenance.source_media_asset_id
            && !media_asset_ids.contains(&source_asset_id)
        {
            return Err(format!(
                "媒体来源引用了不存在的源媒体资产: media_asset_id={}, source_media_asset_id={source_asset_id}",
                provenance.media_asset_id
            ));
        }
        validate_media_provenance_source_external_path(provenance, &image_ids, &video_ids)?;
        validate_media_provenance_snapshot(
            provenance,
            &MediaProvenanceSnapshotRefs {
                graph_ids: &graph_ids,
                node_ids: &node_ids,
                text_asset_ids: &text_asset_ids,
                generation_spec_ids: &generation_spec_ids,
                prompt_artifact_ids: &prompt_artifact_ids,
                image_ids: &image_ids,
                video_ids: &video_ids,
                media_asset_ids: &media_asset_ids,
                video_segment_ids: &video_segment_ids,
                assembly_timeline_ids: &assembly_timeline_ids,
                assembly_clip_ids: &assembly_clip_ids,
            },
        )?;
    }
    for segment in &package.video_segments {
        if segment.project_id != package.project.id || segment.root_project_id != package.project.id
        {
            return Err(format!(
                "视频片段不属于导入包 freeform 项目: video_segment_id={}",
                segment.id
            ));
        }
        if segment.source_chapter_id.is_some() || segment.source_snapshot_chapter_id.is_some() {
            return Err(format!(
                "视频画布片段不能绑定文本章节: video_segment_id={}",
                segment.id
            ));
        }
        if let Some(spec_id) = segment.source_generation_spec_id
            && !generation_spec_ids.contains(&spec_id)
        {
            return Err(format!(
                "视频片段引用了不存在的生成意图: video_segment_id={}, generation_spec_id={spec_id}",
                segment.id
            ));
        }
        if let Some(asset_id) = segment.result_media_asset_id
            && !media_asset_ids.contains(&asset_id)
        {
            return Err(format!(
                "视频片段引用了不存在的媒体资产: video_segment_id={}, media_asset_id={asset_id}",
                segment.id
            ));
        }
        if let Some(previous_id) = segment.previous_segment_id
            && !video_segment_ids.contains(&previous_id)
        {
            return Err(format!(
                "视频片段引用了不存在的上一片段: video_segment_id={}, previous_segment_id={previous_id}",
                segment.id
            ));
        }
        if let Some(next_id) = segment.next_segment_id
            && !video_segment_ids.contains(&next_id)
        {
            return Err(format!(
                "视频片段引用了不存在的下一片段: video_segment_id={}, next_segment_id={next_id}",
                segment.id
            ));
        }
    }
    for timeline in &package.assembly_timelines {
        if timeline.project_id != package.project.id {
            return Err(format!(
                "成片时间线不属于导入包项目: assembly_timeline_id={}",
                timeline.id
            ));
        }
        if timeline.source_chapter_id.is_some() || timeline.source_snapshot_chapter_id.is_some() {
            return Err(format!(
                "视频画布成片时间线不能绑定文本章节: assembly_timeline_id={}",
                timeline.id
            ));
        }
        if let Some(asset_id) = timeline.preview_media_asset_id
            && !media_asset_ids.contains(&asset_id)
        {
            return Err(format!(
                "成片时间线引用了不存在的预览媒体资产: assembly_timeline_id={}, media_asset_id={asset_id}",
                timeline.id
            ));
        }
        if let Some(cache_key) = timeline.preview_cache_key.as_deref()
            && let Some((graph_id, node_id)) = parse_canvas_cache_key(cache_key)
        {
            if !graph_ids.contains(&graph_id) {
                return Err(format!(
                    "成片时间线缓存键引用了不存在的画布图: assembly_timeline_id={}, graph_id={graph_id}",
                    timeline.id
                ));
            }
            if !node_ids.contains(&node_id) {
                return Err(format!(
                    "成片时间线缓存键引用了不存在的画布节点: assembly_timeline_id={}, node_id={node_id}",
                    timeline.id
                ));
            }
        }
    }
    for clip in &package.assembly_clips {
        if !assembly_timeline_ids.contains(&clip.timeline_id) {
            return Err(format!(
                "成片 clip 引用了不存在的时间线: assembly_clip_id={}",
                clip.id
            ));
        }
        if !media_asset_ids.contains(&clip.media_asset_id) {
            return Err(format!(
                "成片 clip 引用了不存在的媒体资产: assembly_clip_id={}, media_asset_id={}",
                clip.id, clip.media_asset_id
            ));
        }
        if let Some(segment_id) = clip.source_segment_id
            && !video_segment_ids.contains(&segment_id)
        {
            return Err(format!(
                "成片 clip 引用了不存在的视频片段: assembly_clip_id={}, source_segment_id={segment_id}",
                clip.id
            ));
        }
    }
    for binding in &package.media_bindings {
        if !media_asset_ids.contains(&binding.media_asset_id) {
            return Err(format!(
                "媒体绑定引用了不存在的媒体资产: media_binding_id={}, media_asset_id={}",
                binding.id, binding.media_asset_id
            ));
        }
        if !is_valid_media_binding_target_kind(&binding.target_kind) {
            return Err(format!(
                "媒体绑定目标类型无效: media_binding_id={}, target_kind={}",
                binding.id, binding.target_kind
            ));
        }
        if !is_valid_media_binding_role(&binding.role) {
            return Err(format!(
                "媒体绑定角色无效: media_binding_id={}, role={}",
                binding.id, binding.role
            ));
        }
        match binding.target_kind.as_str() {
            "generated_video" if !video_ids.contains(&binding.target_id) => {
                return Err(format!(
                    "媒体绑定引用了不存在的生成视频: media_binding_id={}, target_id={}",
                    binding.id, binding.target_id
                ));
            }
            "assembly_timeline" if !assembly_timeline_ids.contains(&binding.target_id) => {
                return Err(format!(
                    "媒体绑定引用了不存在的成片时间线: media_binding_id={}, target_id={}",
                    binding.id, binding.target_id
                ));
            }
            "video_segment" if !video_segment_ids.contains(&binding.target_id) => {
                return Err(format!(
                    "媒体绑定引用了不存在的视频片段: media_binding_id={}, target_id={}",
                    binding.id, binding.target_id
                ));
            }
            "generated_video" | "assembly_timeline" | "video_segment" => {}
            other => {
                return Err(format!(
                    "视频画布项目包暂不支持媒体绑定目标类型: media_binding_id={}, target_kind={other}",
                    binding.id
                ));
            }
        }
    }
    for entry in media_payload.entries.values() {
        match entry.owner_kind.as_str() {
            MEDIA_OWNER_IMAGE if !image_ids.contains(&entry.owner_id) => {
                return Err(format!(
                    "媒体文件条目引用了不存在的生成图片: {}",
                    entry.owner_id
                ));
            }
            MEDIA_OWNER_VIDEO if !video_ids.contains(&entry.owner_id) => {
                return Err(format!(
                    "媒体文件条目引用了不存在的生成视频: {}",
                    entry.owner_id
                ));
            }
            MEDIA_OWNER_MEDIA_ASSET if !media_asset_ids.contains(&entry.owner_id) => {
                return Err(format!(
                    "媒体文件条目引用了不存在的媒体资产: {}",
                    entry.owner_id
                ));
            }
            MEDIA_OWNER_IMAGE | MEDIA_OWNER_VIDEO | MEDIA_OWNER_MEDIA_ASSET => {}
            other => return Err(format!("媒体文件条目 owner_kind 无效: {other}")),
        }
    }
    Ok(())
}

fn validate_canvas_edge_contracts(package: &VideoStudioProjectPackage) -> Result<(), String> {
    for graph in &package.canvas_graphs {
        let document = CanvasGraphDocument {
            graph: graph.clone(),
            nodes: package
                .canvas_nodes
                .iter()
                .filter(|node| node.graph_id == graph.id)
                .cloned()
                .collect(),
            edges: package
                .canvas_edges
                .iter()
                .filter(|edge| edge.graph_id == graph.id)
                .cloned()
                .collect(),
            text_assets: package.canvas_text_assets.clone(),
        };
        validate_canvas_document_edge_contract(&document).map_err(|err| {
            format!(
                "视频画布项目包画布连线合同无效: canvas_graph_id={}, {}",
                graph.id, err
            )
        })?;
        validate_canvas_document_is_dag(&document).map_err(|err| {
            format!(
                "视频画布项目包画布结构无效: canvas_graph_id={}, {}",
                graph.id, err
            )
        })?;
    }
    Ok(())
}

struct CanvasNodeRefIds<'a> {
    text_asset_ids: &'a HashSet<i64>,
    generation_spec_ids: &'a HashSet<i64>,
    prompt_artifact_ids: &'a HashSet<i64>,
    image_ids: &'a HashSet<i64>,
    video_ids: &'a HashSet<i64>,
    media_asset_ids: &'a HashSet<i64>,
    video_segment_ids: &'a HashSet<i64>,
    assembly_timeline_ids: &'a HashSet<i64>,
}

fn validate_canvas_node_ref(
    node: &canvas_node::Model,
    refs: &CanvasNodeRefIds<'_>,
) -> Result<(), String> {
    let Some(ref_type) = node
        .ref_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        if node.ref_id.is_some() {
            return Err(format!(
                "画布节点 ref_type 为空时不能设置 ref_id: canvas_node_id={}",
                node.id
            ));
        }
        return Ok(());
    };
    match ref_type {
        "inline" => {
            if node.ref_id.is_some() {
                return Err(format!(
                    "inline 画布节点不能设置 ref_id: canvas_node_id={}",
                    node.id
                ));
            }
            Ok(())
        }
        "config" => {
            if let Some(ref_id) = node.ref_id
                && ref_id <= 0
            {
                return Err(format!(
                    "config 画布节点 ref_id 必须是正整数: canvas_node_id={}",
                    node.id
                ));
            }
            Ok(())
        }
        "legacy_generated_image" => {
            require_canvas_node_ref_id(node, ref_type, refs.image_ids, "生成图片")
        }
        "legacy_generated_video" => {
            require_canvas_node_ref_id(node, ref_type, refs.video_ids, "生成视频")
        }
        "canvas_text_asset" => {
            require_canvas_node_ref_id(node, ref_type, refs.text_asset_ids, "画布文本资产")
        }
        "generation_spec" => {
            require_canvas_node_ref_id(node, ref_type, refs.generation_spec_ids, "生成意图")
        }
        "prompt_artifact" => {
            require_canvas_node_ref_id(node, ref_type, refs.prompt_artifact_ids, "提示词产物")
        }
        "media_asset" => {
            require_canvas_node_ref_id(node, ref_type, refs.media_asset_ids, "媒体资产")
        }
        "video_segment" => {
            require_canvas_node_ref_id(node, ref_type, refs.video_segment_ids, "视频片段")
        }
        "assembly_timeline" => {
            require_canvas_node_ref_id(node, ref_type, refs.assembly_timeline_ids, "成片时间线")
        }
        "legacy_chapter_video_prompt" => Err(format!(
            "视频画布项目包不能包含章节视频提示词引用: canvas_node_id={}",
            node.id
        )),
        other => Err(format!(
            "视频画布项目包包含未知画布节点引用类型: canvas_node_id={}, ref_type={other}",
            node.id
        )),
    }
}

fn require_canvas_node_ref_id(
    node: &canvas_node::Model,
    ref_type: &str,
    allowed_ids: &HashSet<i64>,
    label: &str,
) -> Result<(), String> {
    let ref_id = node.ref_id.ok_or_else(|| {
        format!(
            "{ref_type} 画布节点必须设置 ref_id: canvas_node_id={}",
            node.id
        )
    })?;
    if ref_id <= 0 {
        return Err(format!(
            "{ref_type} 画布节点 ref_id 必须是正整数: canvas_node_id={}",
            node.id
        ));
    }
    if !allowed_ids.contains(&ref_id) {
        return Err(format!(
            "画布节点引用了不存在的{label}: canvas_node_id={}, ref_id={ref_id}",
            node.id
        ));
    }
    Ok(())
}

fn validate_generation_spec_target(
    spec: &generation_spec::Model,
    node_ids: &HashSet<i64>,
    media_asset_ids: &HashSet<i64>,
    video_segment_ids: &HashSet<i64>,
    assembly_timeline_ids: &HashSet<i64>,
) -> Result<(), String> {
    let Some(target_id) = spec.target_entity_id else {
        return Ok(());
    };
    if target_id <= 0 {
        return Err(format!(
            "生成意图 target_entity_id 必须是正整数: generation_spec_id={}",
            spec.id
        ));
    }
    let (allowed_ids, label) = match spec.target_entity_kind.as_str() {
        "canvas_node" => (node_ids, "画布节点"),
        "media_asset" => (media_asset_ids, "媒体资产"),
        "video_segment" => (video_segment_ids, "视频片段"),
        "assembly_timeline" => (assembly_timeline_ids, "成片时间线"),
        other => {
            return Err(format!(
                "视频画布项目包暂不支持生成意图目标类型: generation_spec_id={}, target_entity_kind={other}",
                spec.id
            ));
        }
    };
    if !allowed_ids.contains(&target_id) {
        return Err(format!(
            "生成意图引用了不存在的{label}: generation_spec_id={}, target_entity_id={target_id}",
            spec.id
        ));
    }
    Ok(())
}

pub(super) fn validate_freeform_video_project(
    project: &novel_project::Model,
    label: &str,
) -> Result<(), String> {
    if project.project_kind != PROJECT_KIND_VIDEO || project.source_mode != SOURCE_MODE_FREEFORM {
        return Err(format!("{label}必须是 source_mode=freeform 的 video 项目"));
    }
    if project.root_project_id.is_some()
        || project.parent_project_id.is_some()
        || project.source_snapshot_id.is_some()
        || project.derived_project_status.is_some()
    {
        return Err(format!("{label}不能包含文本派生项目引用字段"));
    }
    Ok(())
}

fn validate_unique_ids<I>(ids: I, label: &str) -> Result<(), String>
where
    I: IntoIterator<Item = i64>,
{
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(format!("视频画布项目包 ID 重复: {label}={id}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> chrono::NaiveDateTime {
        chrono::Utc::now().naive_utc()
    }

    fn freeform_project() -> novel_project::Model {
        novel_project::Model {
            id: 1,
            user_id: 1,
            title: "自由视频".to_string(),
            description: String::new(),
            project_type: "script".to_string(),
            project_kind: PROJECT_KIND_VIDEO.to_string(),
            source_mode: SOURCE_MODE_FREEFORM.to_string(),
            root_project_id: None,
            parent_project_id: None,
            source_snapshot_id: None,
            source_sync_status: SOURCE_SYNC_CURRENT.to_string(),
            derived_project_status: None,
            status: "draft".to_string(),
            target_chapters: 1,
            current_chapter: 1,
            total_chapters: None,
            chapters_per_part: None,
            avatar_svg: None,
            avatar_animal: None,
            raw_blueprint: None,
            import_analysis_progress: None,
            export_config: None,
            created_at: now(),
            updated_at: now(),
        }
    }

    fn package() -> VideoStudioProjectPackage {
        VideoStudioProjectPackage {
            version: VIDEO_STUDIO_PACKAGE_VERSION.to_string(),
            exported_at: now().to_string(),
            mode: PACKAGE_MODE.to_string(),
            project: freeform_project(),
            canvas_graphs: Vec::new(),
            canvas_nodes: Vec::new(),
            canvas_edges: Vec::new(),
            canvas_text_assets: Vec::new(),
            generation_specs: Vec::new(),
            prompt_artifacts: Vec::new(),
            generated_images: Vec::new(),
            generated_videos: Vec::new(),
            media_assets: Vec::new(),
            media_provenance: Vec::new(),
            media_bindings: Vec::new(),
            video_segments: Vec::new(),
            assembly_timelines: Vec::new(),
            assembly_clips: Vec::new(),
            media_files: Vec::new(),
        }
    }

    fn video_with_file() -> generated_video::Model {
        generated_video::Model {
            id: 10,
            project_id: 1,
            chapter_id: None,
            video_type: "canvas_video".to_string(),
            workflow_mode: Some("canvas_video".to_string()),
            file_path: Some("projects/1/videos/video.mp4".to_string()),
            thumbnail_path: Some("projects/1/videos/video.jpg".to_string()),
            prompt: "prompt".to_string(),
            negative_prompt: None,
            model: "test".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 3.0,
            fps: 24,
            file_size: Some(10),
            source_video_id: None,
            source_image_id: None,
            status: MEDIA_STATUS_COMPLETED.to_string(),
            provider_job_id: None,
            error_message: None,
            metadata_json: None,
            is_favorite: false,
            created_at: now(),
            updated_at: now(),
        }
    }

    fn empty_payload() -> ImportMediaPayload {
        ImportMediaPayload {
            entries: HashMap::new(),
            bytes: HashMap::new(),
        }
    }

    fn graph() -> canvas_graph::Model {
        canvas_graph::Model {
            id: 20,
            project_id: 1,
            name: "默认画布".to_string(),
            graph_kind: "video_studio".to_string(),
            version: 1,
            viewport_json: None,
            created_at: now(),
            updated_at: now(),
        }
    }

    fn node_with_ref(ref_type: &str, ref_id: i64) -> canvas_node::Model {
        canvas_node::Model {
            id: 30,
            graph_id: 20,
            node_kind: "VideoNode".to_string(),
            ref_type: Some(ref_type.to_string()),
            ref_id: Some(ref_id),
            config_json: serde_json::json!({}),
            position_json: serde_json::json!({ "x": 0, "y": 0 }),
            state: "idle".to_string(),
            created_at: now(),
            updated_at: now(),
        }
    }

    fn canvas_node(id: i64, node_kind: &str) -> canvas_node::Model {
        canvas_node::Model {
            id,
            graph_id: 20,
            node_kind: node_kind.to_string(),
            ref_type: Some("inline".to_string()),
            ref_id: None,
            config_json: serde_json::json!({}),
            position_json: serde_json::json!({ "x": 0, "y": 0 }),
            state: "idle".to_string(),
            created_at: now(),
            updated_at: now(),
        }
    }

    fn canvas_edge(
        id: i64,
        source_node_id: i64,
        target_node_id: i64,
        source_port: &str,
        target_port: &str,
        role: &str,
        config_json: serde_json::Value,
    ) -> canvas_edge::Model {
        canvas_edge::Model {
            id,
            graph_id: 20,
            source_node_id,
            source_port: source_port.to_string(),
            target_node_id,
            target_port: target_port.to_string(),
            role: role.to_string(),
            config_json,
        }
    }

    fn assembly_timeline_with_cache_key(cache_key: &str) -> assembly_timeline::Model {
        assembly_timeline::Model {
            id: 40,
            project_id: 1,
            source_chapter_id: None,
            source_snapshot_chapter_id: None,
            version: 1,
            status: MEDIA_STATUS_COMPLETED.to_string(),
            preview_media_asset_id: None,
            preview_cache_key: Some(cache_key.to_string()),
            created_at: now(),
            updated_at: now(),
        }
    }

    fn media_asset_with_file(bytes: &[u8]) -> media_asset::Model {
        media_asset::Model {
            id: 60,
            project_id: 1,
            root_project_id: 1,
            source_snapshot_id: None,
            source_chapter_id: None,
            source_snapshot_chapter_id: None,
            media_kind: "video".to_string(),
            origin: "imported".to_string(),
            file_path: Some("projects/1/videos/asset.mp4".to_string()),
            thumbnail_path: None,
            width: Some(1280),
            height: Some(720),
            duration_secs: Some(3.0),
            fps: Some(24.0),
            file_size: Some(bytes.len() as i64),
            checksum: Some(sha256_bytes(bytes)),
            status: MEDIA_STATUS_COMPLETED.to_string(),
            error_message: None,
            created_at: now(),
            updated_at: now(),
        }
    }

    fn media_asset_without_file() -> media_asset::Model {
        let mut asset = media_asset_with_file(b"unused");
        asset.file_path = None;
        asset.file_size = None;
        asset.checksum = None;
        asset.status = "failed".to_string();
        asset.error_message = Some("导入包测试媒体资产".to_string());
        asset
    }

    fn media_provenance(
        source_external_path: Option<&str>,
        request_snapshot_json: Option<serde_json::Value>,
    ) -> media_provenance::Model {
        media_provenance::Model {
            media_asset_id: 60,
            generation_spec_id: None,
            prompt_artifact_id: None,
            provider_job_id: None,
            source_media_asset_id: None,
            source_external_path: source_external_path.map(str::to_string),
            imported_from_user: false,
            request_snapshot_json,
            created_at: now(),
        }
    }

    fn generation_spec_with_target(
        target_entity_kind: &str,
        target_entity_id: Option<i64>,
    ) -> generation_spec::Model {
        generation_spec::Model {
            id: 50,
            project_id: 1,
            root_project_id: 1,
            source_context_kind: Some(SOURCE_MODE_FREEFORM.to_string()),
            source_snapshot_id: None,
            source_chapter_id: None,
            source_snapshot_chapter_id: None,
            canvas_text_asset_id: None,
            target_kind: "video".to_string(),
            target_entity_kind: target_entity_kind.to_string(),
            target_entity_id,
            status: "completed".to_string(),
            source_version_hash: "hash".to_string(),
            params_json: serde_json::json!({}),
            created_at: now(),
            updated_at: now(),
        }
    }

    #[test]
    fn validate_video_studio_manifest_rejects_missing_generated_video_file_payload() {
        let mut package = package();
        package.generated_videos.push(video_with_file());

        let err = validate_package_manifest(&package, &empty_payload())
            .expect_err("generated video file path must require package payload");

        assert!(err.contains("owner_kind=generated_video"));
        assert!(err.contains("field=file_path"));
    }

    #[test]
    fn validate_video_studio_manifest_rejects_missing_generated_video_thumbnail_payload() {
        let mut package = package();
        let mut video = video_with_file();
        video.file_path = None;
        package.generated_videos.push(video);

        let err = validate_package_manifest(&package, &empty_payload())
            .expect_err("generated video thumbnail path must require package payload");

        assert!(err.contains("owner_kind=generated_video"));
        assert!(err.contains("field=thumbnail_path"));
    }

    #[test]
    fn validate_video_studio_manifest_accepts_generated_video_media_payloads() {
        let mut package = package();
        let video = video_with_file();
        package.generated_videos.push(video.clone());
        let mut payload = empty_payload();
        for (field, path) in [
            (MEDIA_FIELD_FILE, video.file_path.clone().unwrap()),
            (MEDIA_FIELD_THUMBNAIL, video.thumbnail_path.clone().unwrap()),
        ] {
            let entry = VideoStudioMediaFileEntry {
                owner_kind: MEDIA_OWNER_VIDEO.to_string(),
                owner_id: video.id,
                field: field.to_string(),
                original_path: path,
                package_path: format!("media/generated_video/{}/{}.bin", video.id, field),
            };
            payload.entries.insert(
                media_file_key(&entry.owner_kind, entry.owner_id, &entry.field),
                entry,
            );
        }

        validate_package_manifest(&package, &payload)
            .expect("generated video media payloads should satisfy manifest validation");
    }

    #[test]
    fn validate_video_studio_manifest_rejects_missing_video_source_ref() {
        let mut package = package();
        let mut video = video_with_file();
        video.file_path = None;
        video.thumbnail_path = None;
        video.source_video_id = Some(999);
        package.generated_videos.push(video);

        let err = validate_package_manifest(&package, &empty_payload())
            .expect_err("missing source video ref must fail");

        assert!(err.contains("source_video_id=999"));
    }

    #[test]
    fn validate_video_studio_manifest_rejects_missing_canvas_node_media_ref() {
        let mut package = package();
        package.canvas_graphs.push(graph());
        package.canvas_nodes.push(node_with_ref("media_asset", 999));

        let err = validate_package_manifest(&package, &empty_payload())
            .expect_err("missing media asset node ref must fail");

        assert!(err.contains("画布节点引用了不存在的媒体资产"));
        assert!(err.contains("ref_id=999"));
    }

    #[test]
    fn validate_video_studio_manifest_rejects_non_object_graph_viewport() {
        let mut package = package();
        let mut graph = graph();
        graph.viewport_json = Some(serde_json::json!([0, 0, 1]));
        package.canvas_graphs.push(graph);

        let err = validate_package_manifest(&package, &empty_payload())
            .expect_err("graph viewport must be a JSON object when present");

        assert!(err.contains("画布视口无效"));
        assert!(err.contains("viewport_json 必须是 JSON 对象"));
    }

    #[test]
    fn validate_video_studio_manifest_rejects_invalid_canvas_edge_role() {
        let mut package = package();
        package.canvas_graphs.push(graph());
        package.canvas_nodes.push(canvas_node(31, "VideoInputNode"));
        package
            .canvas_nodes
            .push(canvas_node(32, "AssembleVideoNode"));
        package.canvas_edges.push(canvas_edge(
            60,
            31,
            32,
            "video",
            "clip",
            "source_video",
            serde_json::json!({ "order_index": 1 }),
        ));

        let err = validate_package_manifest(&package, &empty_payload())
            .expect_err("import package must reject invalid canvas edge role");

        assert!(err.contains("画布连线合同无效"));
        assert!(err.contains("role 无效"));
        assert!(err.contains("source_video"));
    }

    #[test]
    fn validate_video_studio_manifest_rejects_assembly_clip_without_order_index() {
        let mut package = package();
        package.canvas_graphs.push(graph());
        package.canvas_nodes.push(canvas_node(31, "VideoInputNode"));
        package
            .canvas_nodes
            .push(canvas_node(32, "AssembleVideoNode"));
        package.canvas_edges.push(canvas_edge(
            60,
            31,
            32,
            "video",
            "clip",
            "assembly_clip",
            serde_json::json!({}),
        ));

        let err = validate_package_manifest(&package, &empty_payload())
            .expect_err("assembly clip edge must keep explicit order_index");

        assert!(err.contains("画布连线合同无效"));
        assert!(err.contains("order_index"));
    }

    #[test]
    fn validate_video_studio_manifest_rejects_cyclic_canvas_graph() {
        let mut package = package();
        package.canvas_graphs.push(graph());
        package.canvas_nodes.push(canvas_node(31, "LlmTextNode"));
        package.canvas_nodes.push(canvas_node(32, "LlmTextNode"));
        package.canvas_edges.push(canvas_edge(
            60,
            31,
            32,
            "text",
            "input",
            "context",
            serde_json::json!({}),
        ));
        package.canvas_edges.push(canvas_edge(
            61,
            32,
            31,
            "text",
            "input",
            "context",
            serde_json::json!({}),
        ));

        let err = validate_package_manifest(&package, &empty_payload())
            .expect_err("import package must reject cyclic canvas graph");

        assert!(err.contains("画布结构无效"));
        assert!(err.contains("有向无环图"));
    }

    #[test]
    fn validate_video_studio_manifest_rejects_timeline_cache_key_missing_node() {
        let mut package = package();
        package.canvas_graphs.push(graph());
        package
            .assembly_timelines
            .push(assembly_timeline_with_cache_key("canvas_graph:20:node:999"));

        let err = validate_package_manifest(&package, &empty_payload())
            .expect_err("timeline cache key must reference a package canvas node");

        assert!(err.contains("成片时间线缓存键引用了不存在的画布节点"));
        assert!(err.contains("node_id=999"));
    }

    #[test]
    fn validate_video_studio_manifest_rejects_missing_source_external_generated_video() {
        let mut package = package();
        package.media_assets.push(media_asset_without_file());
        package
            .media_provenance
            .push(media_provenance(Some("generated_video:999"), None));

        let err = validate_package_manifest(&package, &empty_payload())
            .expect_err("standard generated video source path must reference package video");

        assert!(err.contains("source_external_path"));
        assert!(err.contains("generated_video_id=999"));
    }

    #[test]
    fn validate_video_studio_manifest_rejects_non_object_request_snapshot() {
        let mut package = package();
        package.media_assets.push(media_asset_without_file());
        package
            .media_provenance
            .push(media_provenance(None, Some(serde_json::json!("bad"))));

        let err = validate_package_manifest(&package, &empty_payload())
            .expect_err("media provenance request snapshot must be object");

        assert!(err.contains("request_snapshot_json 必须是 JSON 对象"));
        assert!(err.contains("media_asset_id=60"));
    }

    #[test]
    fn validate_video_studio_manifest_rejects_missing_request_snapshot_generated_video() {
        let mut package = package();
        package.media_assets.push(media_asset_without_file());
        package.media_provenance.push(media_provenance(
            None,
            Some(serde_json::json!({
                "source": "video_canvas",
                "generated_video_id": 999
            })),
        ));

        let err = validate_package_manifest(&package, &empty_payload())
            .expect_err("request snapshot generated video id must be package-local");

        assert!(err.contains("request_snapshot_json.generated_video_id"));
        assert!(err.contains("id=999"));
    }

    #[test]
    fn validate_video_studio_manifest_rejects_negative_media_file_size() {
        let mut package = package();
        let mut video = video_with_file();
        video.file_path = None;
        video.thumbnail_path = None;
        video.file_size = Some(-1);
        package.generated_videos.push(video);

        let err = validate_package_manifest(&package, &empty_payload())
            .expect_err("negative media file size must fail");

        assert!(err.contains("file_size 不能为负数"));
        assert!(err.contains("owner_kind=generated_video"));
    }

    #[test]
    fn validate_media_entry_bytes_rejects_generated_video_file_size_mismatch() {
        let mut package = package();
        let video = video_with_file();
        package.generated_videos.push(video.clone());
        let entry = VideoStudioMediaFileEntry {
            owner_kind: MEDIA_OWNER_VIDEO.to_string(),
            owner_id: video.id,
            field: MEDIA_FIELD_FILE.to_string(),
            original_path: video.file_path.unwrap(),
            package_path: "media/generated_video/10/file.mp4".to_string(),
        };

        let err = validate_media_entry_bytes(&package, &entry, b"short")
            .expect_err("payload bytes must match generated video file_size");

        assert!(err.contains("媒体文件大小与清单不一致"));
        assert!(err.contains("owner_kind=generated_video"));
    }

    #[test]
    fn validate_media_entry_bytes_rejects_media_asset_checksum_mismatch() {
        let mut package = package();
        let asset = media_asset_with_file(b"expected bytes");
        package.media_assets.push(asset.clone());
        let entry = VideoStudioMediaFileEntry {
            owner_kind: MEDIA_OWNER_MEDIA_ASSET.to_string(),
            owner_id: asset.id,
            field: MEDIA_FIELD_FILE.to_string(),
            original_path: asset.file_path.unwrap(),
            package_path: "media/media_asset/60/file.mp4".to_string(),
        };

        let err = validate_media_entry_bytes(&package, &entry, b"different data")
            .expect_err("payload bytes must match media asset checksum");

        assert!(err.contains("checksum 与清单不一致"));
        assert!(err.contains("owner_kind=media_asset"));
    }

    #[test]
    fn validate_video_studio_manifest_rejects_missing_generation_spec_segment_target() {
        let mut package = package();
        package
            .generation_specs
            .push(generation_spec_with_target("video_segment", Some(999)));

        let err = validate_package_manifest(&package, &empty_payload())
            .expect_err("generation spec target must point to package-local segment");

        assert!(err.contains("生成意图引用了不存在的视频片段"));
        assert!(err.contains("target_entity_id=999"));
    }

    #[test]
    fn validate_video_studio_manifest_rejects_unknown_generation_spec_target_kind() {
        let mut package = package();
        package
            .generation_specs
            .push(generation_spec_with_target("legacy_chapter", Some(1)));

        let err = validate_package_manifest(&package, &empty_payload())
            .expect_err("unknown generation spec target kind with id must fail");

        assert!(err.contains("暂不支持生成意图目标类型"));
        assert!(err.contains("legacy_chapter"));
    }
}
