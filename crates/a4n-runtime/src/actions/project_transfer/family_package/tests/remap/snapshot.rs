use super::*;

#[test]
fn remap_snapshot_hash_json_rewrites_known_ids() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(1, 101);
    maps.chapter_ids.insert(10, 110);
    maps.character_profile_ids.insert(20, 120);
    maps.object_profile_ids.insert(30, 130);
    maps.scene_profile_ids.insert(40, 140);
    let character_payload = json!({"id": 20, "project_id": 1, "name": "角色"});
    let object_payload = json!({"id": 30, "project_id": 1, "name": "物体"});
    let scene_payload = json!({"id": 40, "project_id": 1, "name": "场景", "parent_scene_id": null});
    let snapshot_profiles = [
        source_snapshot_profile::Model {
            id: 1,
            snapshot_id: 901,
            root_project_id: 1,
            profile_kind: "character".to_string(),
            source_profile_id: 20,
            payload_hash: hash_json(&character_payload),
            payload_json: character_payload,
        },
        source_snapshot_profile::Model {
            id: 2,
            snapshot_id: 901,
            root_project_id: 1,
            profile_kind: "object".to_string(),
            source_profile_id: 30,
            payload_hash: hash_json(&object_payload),
            payload_json: object_payload,
        },
        source_snapshot_profile::Model {
            id: 3,
            snapshot_id: 901,
            root_project_id: 1,
            profile_kind: "scene".to_string(),
            source_profile_id: 40,
            payload_hash: hash_json(&scene_payload),
            payload_json: scene_payload,
        },
    ];
    let snapshot_profile_index = snapshot_profiles
        .iter()
        .map(|profile| {
            (
                (
                    profile.snapshot_id,
                    snapshot_profile_key(&profile.profile_kind, profile.source_profile_id),
                ),
                profile,
            )
        })
        .collect::<HashMap<_, _>>();

    let chapter_hashes = remap_chapter_hashes_json(
        &json!([{"source_chapter_id": 10, "content_hash": "a"}]),
        &maps,
    )
    .unwrap();
    let profile_hashes = remap_profile_hashes_json(
        &json!([
            {"kind": "character", "id": 20, "hash": "b"},
            {"kind": "object", "id": 30, "hash": "c"},
            {"kind": "scene", "id": 40, "hash": "d"}
        ]),
        901,
        &snapshot_profile_index,
        &maps,
    )
    .unwrap();

    assert_eq!(chapter_hashes[0]["source_chapter_id"], json!(110));
    assert_eq!(profile_hashes[0]["id"], json!(120));
    assert_eq!(profile_hashes[1]["id"], json!(130));
    assert_eq!(profile_hashes[2]["id"], json!(140));
    assert_eq!(
        profile_hashes[0]["hash"],
        json!(hash_json(
            &json!({"id": 120, "project_id": 101, "name": "角色"})
        ))
    );
    assert_eq!(
        profile_hashes[1]["hash"],
        json!(hash_json(
            &json!({"id": 130, "project_id": 101, "name": "物体"})
        ))
    );
    assert_eq!(
        profile_hashes[2]["hash"],
        json!(hash_json(
            &json!({"id": 140, "project_id": 101, "name": "场景", "parent_scene_id": null})
        ))
    );
}

#[test]
fn remap_source_snapshot_outline_payload_rewrites_payload_identity_and_hash() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(1, 101);
    maps.part_outline_ids.insert(301, 1301);
    maps.chapter_outline_ids.insert(302, 1302);

    let mut outline = source_snapshot_outline_record("chapter");
    outline.source_outline_id = 302;
    outline.payload_json = json!({
        "id": 302,
        "project_id": 1,
        "part_id": 301,
        "title": "冻结章纲"
    });
    outline.payload_hash = hash_json(&outline.payload_json);

    let (payload, payload_hash) = remap_source_snapshot_outline_payload(&outline, &maps).unwrap();

    assert_eq!(payload["id"], json!(1302));
    assert_eq!(payload["project_id"], json!(101));
    assert_eq!(payload["part_id"], json!(1301));
    assert_eq!(payload_hash, hash_json(&payload));
    assert_ne!(payload_hash, outline.payload_hash);
}

#[test]
fn remap_source_snapshot_profile_payload_rewrites_scene_parent_and_hash() {
    let mut maps = ImportIdMaps::default();
    maps.project_ids.insert(1, 101);
    maps.scene_profile_ids.insert(401, 1401);
    maps.scene_profile_ids.insert(402, 1402);

    let mut profile = source_snapshot_profile_record("scene");
    profile.source_profile_id = 402;
    profile.payload_json = json!({
        "id": 402,
        "project_id": 1,
        "parent_scene_id": 401,
        "name": "子场景"
    });
    profile.payload_hash = hash_json(&profile.payload_json);

    let (payload, payload_hash) = remap_source_snapshot_profile_payload(&profile, &maps).unwrap();

    assert_eq!(payload["id"], json!(1402));
    assert_eq!(payload["project_id"], json!(101));
    assert_eq!(payload["parent_scene_id"], json!(1401));
    assert_eq!(payload_hash, hash_json(&payload));
    assert_ne!(payload_hash, profile.payload_hash);
}
