use regex::Regex;

pub fn decode_xml_entities(text: &str) -> String {
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
