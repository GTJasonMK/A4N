/// 从中文叙事文本中提取短地点短语，供 RAG 查询扩展复用。
pub fn extract_scene_location(text: &str) -> Option<String> {
    for cue in ["来到", "抵达", "前往", "回到"] {
        if let Some(location) = extract_location_after_cue(text, cue, false) {
            return Some(location);
        }
    }
    extract_location_after_cue(text, "在", true)
}

fn extract_location_after_cue(
    text: &str,
    cue: &str,
    require_location_suffix: bool,
) -> Option<String> {
    let characters = text.chars().collect::<Vec<_>>();
    let cue_characters = cue.chars().collect::<Vec<_>>();
    if cue_characters.is_empty() || characters.len() <= cue_characters.len() {
        return None;
    }
    for start in 0..=characters.len().saturating_sub(cue_characters.len()) {
        if characters[start..start + cue_characters.len()] != cue_characters {
            continue;
        }
        let mut location = String::new();
        let mut has_location_suffix = false;
        for character in characters.iter().skip(start + cue_characters.len()) {
            if location.chars().count() >= 10 || is_location_stop_char(*character) {
                break;
            }
            if location.chars().count() >= 2 && is_location_action_start(*character) {
                break;
            }
            if matches!(*character, '中' | '里' | '内' | '上' | '下' | '旁' | '边') {
                has_location_suffix = true;
                break;
            }
            location.push(*character);
        }
        let location = location.trim_matches(|character: char| {
            character.is_whitespace()
                || matches!(
                    character,
                    '《' | '》' | '"' | '\'' | '“' | '”' | '「' | '」'
                )
        });
        if (2..=10).contains(&location.chars().count())
            && (!require_location_suffix || has_location_suffix)
        {
            return Some(location.to_string());
        }
    }
    None
}

fn is_location_stop_char(character: char) -> bool {
    character.is_whitespace()
        || matches!(
            character,
            '，' | '。'
                | '、'
                | '；'
                | '：'
                | '！'
                | '？'
                | ','
                | '.'
                | ';'
                | ':'
                | '!'
                | '?'
                | '\n'
                | '\r'
        )
}

fn is_location_action_start(character: char) -> bool {
    matches!(
        character,
        '调' | '确' | '寻' | '会' | '发' | '阻' | '救' | '追' | '见' | '问' | '记' | '查'
    )
}

#[cfg(test)]
mod tests {
    use super::extract_scene_location;

    #[test]
    fn extracts_locations_after_directional_cues() {
        assert_eq!(
            extract_scene_location("陈恪前往玄武门确认伏兵").as_deref(),
            Some("玄武门")
        );
        assert_eq!(
            extract_scene_location("众人回到旧书房查看密信").as_deref(),
            Some("旧书房")
        );
    }

    #[test]
    fn requires_location_suffix_for_ambiguous_zai_cue() {
        assert_eq!(
            extract_scene_location("众人在显德殿中等待消息").as_deref(),
            Some("显德殿")
        );
        assert!(extract_scene_location("他在沉默之后做出选择").is_none());
    }

    #[test]
    fn rejects_short_locations_and_caps_long_candidates() {
        assert!(extract_scene_location("他前往门").is_none());
        assert_eq!(
            extract_scene_location("他前往一二三四五六七八九十一确认消息").as_deref(),
            Some("一二三四五六七八九十")
        );
    }
}
