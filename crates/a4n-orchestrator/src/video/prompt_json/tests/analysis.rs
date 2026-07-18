#[test]
fn parses_video_shot_grouping_with_directing_beats() {
    let value = valid_shooting_script_projection_value();

    let analysis = parse_valid_shooting_script_projection(&value).expect("shooting script projection should parse");

    assert_eq!(analysis.cinematic_shots.len(), 1);
    let shot = &analysis.cinematic_shots[0];
    assert_eq!(shot.shot_id, 1);
    assert_eq!(shot.segment_duration_secs, 6);
    assert_eq!(shot.required_segment_count, 1);
    assert_eq!(shot.estimated_natural_seconds, 6.0);
    assert_eq!(shot.action_beats[0].beat_id, 1);
    assert_eq!(shot.action_beats[0].source_span_ids, vec!["ch1:s1:f1"]);
    assert_eq!(analysis.planning_quality.total_required_segments, 1);
}

#[test]
fn rejects_video_shooting_script_projection_with_model_owned_shot_id() {
    let mut value = valid_shooting_script_projection_value();
    value["cinematic_shots"][0]["shot_id"] = json!(1);

    let err =
        parse_valid_shooting_script_projection(&value).expect_err("shot_id must be derived by the backend");

    assert!(err.to_string().contains("unknown field `shot_id`"));
}

#[test]
fn parses_stored_video_shooting_script_projection_canonical_contract() {
    let analysis =
        parse_valid_shooting_script_projection(&valid_shooting_script_projection_value()).expect("raw analysis parses");
    let stored = serde_json::to_value(&analysis).expect("stored analysis serializes");

    let parsed = parse_stored_video_shooting_script_projection(&stored.to_string(), &valid_source_spans(), 6)
        .expect("stored canonical analysis should parse");

    let shot = &parsed.cinematic_shots[0];
    assert_eq!(shot.shot_id, 1);
    assert_eq!(shot.action_beats[0].beat_id, 1);
    assert_eq!(shot.estimated_natural_seconds, 6.0);
    assert_eq!(parsed.planning_quality.total_estimated_seconds, 6.0);
}

#[test]
fn reparses_stored_video_shooting_script_projection_with_current_duration_context() {
    let analysis =
        parse_valid_shooting_script_projection(&valid_shooting_script_projection_value()).expect("raw analysis parses");
    let mut stored = serde_json::to_value(&analysis).expect("stored analysis serializes");
    stored["cinematic_shots"][0]["segment_duration_secs"] = json!(6);
    stored["cinematic_shots"][0]["required_segment_count"] = json!(1);
    stored["planning_quality"]["total_estimated_seconds"] = json!(999.0);
    stored["planning_quality"]["total_required_segments"] = json!(99);

    let parsed = parse_stored_video_shooting_script_projection(&stored.to_string(), &valid_source_spans(), 5)
        .expect("stored duration-derived fields should be rederived");

    let shot = &parsed.cinematic_shots[0];
    assert_eq!(shot.segment_duration_secs, 5);
    assert_eq!(shot.required_segment_count, 2);
    assert_eq!(parsed.planning_quality.total_estimated_seconds, 6.0);
    assert_eq!(parsed.planning_quality.total_required_segments, 2);
}

#[test]
fn rejects_stored_video_shooting_script_projection_without_planning_quality() {
    let analysis =
        parse_valid_shooting_script_projection(&valid_shooting_script_projection_value()).expect("raw analysis parses");
    let mut stored = serde_json::to_value(&analysis).expect("stored analysis serializes");
    stored
        .as_object_mut()
        .expect("stored analysis should be object")
        .remove("planning_quality");

    let err = parse_stored_video_shooting_script_projection(&stored.to_string(), &valid_source_spans(), 6)
        .expect_err("stored planning_quality must be present");

    assert!(err.to_string().contains("missing field `planning_quality`"));
}

#[test]
fn rejects_legacy_stored_video_shooting_script_projection_planning_quality_shape() {
    let analysis =
        parse_valid_shooting_script_projection(&valid_shooting_script_projection_value()).expect("raw analysis parses");
    let mut stored = serde_json::to_value(&analysis).expect("stored analysis serializes");
    stored["planning_quality"] = json!({
        "legacy": true,
        "model_owned_summary": "旧版本字段不作为恢复来源"
    });

    let err = parse_stored_video_shooting_script_projection(&stored.to_string(), &valid_source_spans(), 6)
        .expect_err("legacy planning_quality shape must not be accepted silently");

    assert!(err.to_string().contains("unknown field `legacy`"));
}

#[test]
fn reparses_stored_video_shooting_script_projection_with_stale_derived_fields() {
    let analysis =
        parse_valid_shooting_script_projection(&valid_shooting_script_projection_value()).expect("raw analysis parses");
    let mut stored = serde_json::to_value(&analysis).expect("stored analysis serializes");
    stored["cinematic_shots"][0]["estimated_natural_seconds"] = json!(999.0);

    let parsed = parse_stored_video_shooting_script_projection(&stored.to_string(), &valid_source_spans(), 6)
        .expect("stored shooting script projection duration-derived fields should be rederived");

    assert_eq!(parsed.cinematic_shots[0].estimated_natural_seconds, 6.0);
}

#[test]
fn rejects_stored_video_shooting_script_projection_with_unknown_field() {
    let analysis =
        parse_valid_shooting_script_projection(&valid_shooting_script_projection_value()).expect("raw analysis parses");
    let mut stored = serde_json::to_value(&analysis).expect("stored analysis serializes");
    stored["cinematic_shots"][0]["legacy_notes"] = json!("旧字段");

    let err = parse_stored_video_shooting_script_projection(&stored.to_string(), &valid_source_spans(), 6)
        .expect_err("stored shooting script projection must reject unknown fields");

    assert!(err.to_string().contains("unknown field `legacy_notes`"));
}

#[test]
fn rejects_video_shooting_script_projection_with_model_owned_source_span_range() {
    let mut value = valid_shooting_script_projection_value();
    value["cinematic_shots"][0]["source_span_range"] =
        json!({"start": "ch1:s1:f1", "end": "ch1:s1:f1"});

    let err = parse_valid_shooting_script_projection(&value)
        .expect_err("source_span_range must be derived by the backend");

    assert!(
        err.to_string()
            .contains("unknown field `source_span_range`")
    );
}

#[test]
fn validates_segment_plan_against_shooting_script_projection() {
    let shooting_script_projection =
        parse_valid_shooting_script_projection(&valid_shooting_script_projection_value()).expect("valid analysis");
    let segment_plan = parse_valid_segment_plan(&valid_segment_plan_value()).expect("valid plan");

    validate_segment_plan_against_shooting_script_projection(&segment_plan, &shooting_script_projection)
        .expect("segment plan should match shooting script projection");
}

#[test]
fn rejects_segment_plan_with_model_owned_segment_index() {
    let mut plan_value = valid_segment_plan_value();
    plan_value["segments"][0]["segment_index_in_shot"] = json!(2);

    let err = parse_valid_segment_plan(&plan_value)
        .expect_err("segment_index_in_shot must be backend-derived");

    assert!(
        err.to_string()
            .contains("unknown field `segment_index_in_shot`")
    );
}

#[test]
fn rejects_segment_plan_with_model_owned_duration_secs() {
    let mut plan_value = valid_segment_plan_value();
    plan_value["segments"][0]["duration_secs"] = json!(8);

    let err =
        parse_valid_segment_plan(&plan_value).expect_err("duration_secs must be backend-derived");

    assert!(err.to_string().contains("unknown field `duration_secs`"));
}

#[test]
fn accepts_segment_plan_when_variable_duration_reduces_default_segment_count() {
    let mut beats = valid_directing_beats();
    beats[1].natural_seconds = 4.0;
    let shooting_script_projection = parse_video_shot_grouping_response(
        &valid_shooting_script_projection_value().to_string(),
        &valid_source_spans(),
        6,
        &beats,
    )
    .expect("valid analysis");
    let segment_plan =
        parse_valid_segment_plan(&valid_segment_plan_value()).expect("plan shape is valid");

    assert_eq!(shooting_script_projection.cinematic_shots[0].required_segment_count, 2);
    validate_segment_plan_against_shooting_script_projection(&segment_plan, &shooting_script_projection)
        .expect("one longer variable-duration segment can cover the required beats");
}

#[test]
fn rejects_segment_plan_missing_required_beat_coverage() {
    let shooting_script_projection =
        parse_valid_shooting_script_projection(&valid_shooting_script_projection_value()).expect("valid analysis");
    let mut plan_value = valid_segment_plan_value();
    plan_value["segments"][0]["covered_beat_ids"] = json!([1]);
    let segment_plan = parse_valid_segment_plan(&plan_value).expect("plan shape is valid");

    let err = validate_segment_plan_against_shooting_script_projection(&segment_plan, &shooting_script_projection)
        .expect_err("required beat coverage is required");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("必需动作节拍")
                && message.contains("beat_id 2")
    ));
}
