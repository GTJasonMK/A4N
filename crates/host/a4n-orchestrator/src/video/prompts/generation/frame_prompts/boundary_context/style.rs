use super::super::*;

pub(in crate::video::prompts::generation::frame_prompts) fn frame_prompt_with_style_template(
    prompt: &str,
    style: &str,
    prompt_output_language: PromptOutputLanguage,
) -> String {
    let style = style.trim();
    if style.is_empty() || prompt.contains(style) {
        return prompt.trim().to_string();
    }
    let style_intro = match prompt_output_language {
        PromptOutputLanguage::ZhCn => "画面风格模板原文",
        PromptOutputLanguage::En => "Verbatim visual style template",
    };
    let style_line = format!("\n{style_intro}: {style}\n");
    let frame_sections = prompt_output_language.frame_sections();
    let style_section = frame_sections[3];
    let next_section = frame_sections[4];
    if let Some(section_start) = prompt.find(style_section) {
        let after_section_start = section_start + style_section.len();
        let after_section = &prompt[after_section_start..];
        if let Some(next_section_start) = after_section.find(next_section) {
            let insert_at = after_section_start + next_section_start;
            let mut value = String::with_capacity(prompt.len() + style_line.len());
            value.push_str(prompt[..insert_at].trim_end());
            value.push_str(&style_line);
            value.push_str(prompt[insert_at..].trim_start());
            return value.trim().to_string();
        }
    }
    format!("{}\n{}{}", prompt.trim(), style_section, style_line)
        .trim()
        .to_string()
}

pub(in crate::video::prompts::generation::frame_prompts) fn apply_boundary_frame_style_template(
    frame: &mut VideoFramePrompt,
    style: &str,
    prompt_output_language: PromptOutputLanguage,
) {
    frame.frame_prompt =
        frame_prompt_with_style_template(&frame.frame_prompt, style, prompt_output_language);
}
