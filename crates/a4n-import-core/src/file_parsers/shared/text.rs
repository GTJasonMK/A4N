use super::entities::decode_xml_entities;
use regex::Regex;

#[cfg(test)]
mod tests;

pub fn extract_xml_text(xml: &str, tag: &str) -> Option<String> {
    let pattern = format!(r"<[^:]*:{tag}[^>]*>([^<]*)</[^:]*:{tag}>|<{tag}[^>]*>([^<]*)</{tag}>");
    let re = Regex::new(&pattern).ok()?;
    re.captures(xml).and_then(|cap| {
        cap.get(1)
            .or_else(|| cap.get(2))
            .map(|m| decode_xml_entities(m.as_str()))
    })
}
