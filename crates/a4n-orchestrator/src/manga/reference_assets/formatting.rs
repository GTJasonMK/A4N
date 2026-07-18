use super::contracts::MangaReferenceContext;

pub(in crate::manga) fn format_character_portrait_context(
    reference_context: &MangaReferenceContext,
) -> String {
    if reference_context.character_portraits.is_empty() {
        return "无可用角色立绘。".to_string();
    }

    reference_context
        .character_portraits
        .iter()
        .map(|portrait| {
            let mut fields = vec![
                format!("portrait_id={}", portrait.portrait_id),
                format!("type={}", portrait.portrait_type),
            ];
            if let Some(expression) = portrait
                .expression
                .as_deref()
                .filter(|v| !v.trim().is_empty())
            {
                fields.push(format!("expression={}", expression.trim()));
            }
            if let Some(pose) = portrait.pose.as_deref().filter(|v| !v.trim().is_empty()) {
                fields.push(format!("pose={}", pose.trim()));
            }
            fields.push(format!("visual_prompt={}", portrait.prompt));
            format!("- {}: {}", portrait.character_name, fields.join(", "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}
