use super::super::super::contracts::PlannedContextPack;

pub(super) fn render_pack_section(
    title: &str,
    packs: Vec<(&PlannedContextPack, &str)>,
) -> (String, Vec<usize>) {
    let mut context = format!("[{title}]\n");
    let mut boundaries = Vec::new();
    for (pack, content) in packs {
        context.push_str("\n## ");
        context.push_str(pack.pack_ref.pack_kind.as_str());
        context.push_str(" / ");
        context.push_str(pack.pack_ref.compression_level.as_str());
        context.push('\n');
        context.push_str(content);
        context.push('\n');
        boundaries.push(context.len());
    }
    (context, boundaries)
}
