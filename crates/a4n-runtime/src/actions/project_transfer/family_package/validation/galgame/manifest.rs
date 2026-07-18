use super::super::*;
use super::choices::validate_galgame_choices;
use super::lines::validate_galgame_lines;
use super::scenes::validate_galgame_scenes;
use super::scripts::validate_galgame_scripts;

pub(in crate::actions::project_transfer::family_package::validation) fn validate_galgame_manifest(
    package: &ProjectFamilyPackage,
) -> Result<(), String> {
    let projects = package
        .projects
        .iter()
        .map(|project| (project.id, project))
        .collect::<HashMap<_, _>>();
    let snapshots = package
        .source_snapshots
        .iter()
        .map(|snapshot| (snapshot.id, snapshot))
        .collect::<HashMap<_, _>>();
    let snapshot_chapters = package
        .source_snapshot_chapters
        .iter()
        .map(|chapter| (chapter.id, chapter))
        .collect::<HashMap<_, _>>();
    let character_profile_ids = package
        .text_core
        .character_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<HashSet<_>>();
    let text_anchors = package
        .text_anchors
        .iter()
        .map(|anchor| (anchor.id, anchor))
        .collect::<HashMap<_, _>>();
    let scripts = package
        .galgame_scripts
        .iter()
        .map(|script| (script.id, script))
        .collect::<HashMap<_, _>>();
    let scenes = package
        .galgame_scenes
        .iter()
        .map(|scene| (scene.id, scene))
        .collect::<HashMap<_, _>>();
    let lines = package
        .galgame_lines
        .iter()
        .map(|line| (line.id, line))
        .collect::<HashMap<_, _>>();

    validate_galgame_scripts(package, &projects, &snapshots)?;
    validate_galgame_scenes(
        package,
        &scripts,
        &scenes,
        &snapshot_chapters,
        &character_profile_ids,
    )?;
    validate_galgame_lines(
        package,
        &scripts,
        &scenes,
        &text_anchors,
        &character_profile_ids,
    )?;
    validate_galgame_choices(package, &lines, &scenes)
}
