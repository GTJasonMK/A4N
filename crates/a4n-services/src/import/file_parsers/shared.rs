use regex::Regex;

pub(super) fn decode_xml_entities(text: &str) -> String {
    let entity_re = Regex::new(r"&(#x[0-9A-Fa-f]+|#\d+|amp|lt|gt|quot|apos|nbsp);").unwrap();

    entity_re
        .replace_all(text, |caps: &regex::Captures<'_>| {
            let entity = &caps[1];
            match entity {
                "amp" => "&".to_string(),
                "lt" => "<".to_string(),
                "gt" => ">".to_string(),
                "quot" => "\"".to_string(),
                "apos" => "'".to_string(),
                "nbsp" => " ".to_string(),
                _ if entity.starts_with("#x") => u32::from_str_radix(&entity[2..], 16)
                    .ok()
                    .and_then(char::from_u32)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| caps[0].to_string()),
                _ if entity.starts_with('#') => entity[1..]
                    .parse::<u32>()
                    .ok()
                    .and_then(char::from_u32)
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| caps[0].to_string()),
                _ => caps[0].to_string(),
            }
        })
        .into_owned()
}

pub(super) fn extract_xml_text(xml: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"<[^:]*:{tag}[^>]*>([^<]*)</[^:]*:{tag}>|<{tag}[^>]*>([^<]*)</{tag}>");
    let re = Regex::new(&pattern).ok()?;
    re.captures(xml).and_then(|cap| {
        cap.get(1)
            .or_else(|| cap.get(2))
            .map(|m| decode_xml_entities(m.as_str()))
    })
}
