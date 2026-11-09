use serde::Deserializer;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};
use std::fmt;

/// JSON objects must have unambiguous keys before any business deserialization.
pub fn parse_strict_json_value(raw: &str) -> Result<Value, serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_str(raw);
    let value = JsonSeed {
        pointer: String::new(),
    }
    .deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(value)
}

struct JsonSeed {
    pointer: String,
}

impl<'de> DeserializeSeed<'de> for JsonSeed {
    type Value = Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        struct JsonVisitor {
            pointer: String,
        }
        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("JSON with unique object keys")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut fields = Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    let pointer = format!(
                        "{}/{}",
                        self.pointer,
                        key.replace('~', "~0").replace('/', "~1")
                    );
                    if fields.contains_key(&key) {
                        return Err(de::Error::custom(format!(
                            "重复 JSON 字段 `{key}`，JSON Pointer `{pointer}`"
                        )));
                    }
                    fields.insert(key, map.next_value_seed(JsonSeed { pointer })?);
                }
                Ok(Value::Object(fields))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element_seed(JsonSeed {
                    pointer: format!("{}/{}", self.pointer, values.len()),
                })? {
                    values.push(value);
                }
                Ok(Value::Array(values))
            }

            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(value.into())
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(value.into())
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(value.into())
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(Value::Number)
                    .ok_or_else(|| de::Error::custom("JSON 数字必须有限"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(value.into())
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(Value::Null)
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                self.visit_none()
            }
        }
        deserializer.deserialize_any(JsonVisitor {
            pointer: self.pointer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_keys_at_any_depth_and_trailing_content() {
        for raw in [
            r#"{"id":1,"id":2}"#,
            r#"{"items":[{"id":1,"\u0069d":2}]}"#,
            "{} {}",
            "{",
            "NaN",
        ] {
            assert!(parse_strict_json_value(raw).is_err(), "{raw}");
        }
        let value =
            parse_strict_json_value(r#"{"items":[{"id":1},{"id":2}],"empty":{},"null":null}"#)
                .unwrap();
        assert_eq!(value["items"][1]["id"], 2);
    }

    #[test]
    fn duplicate_errors_identify_the_object_without_rejecting_keys_in_other_objects() {
        for (raw, pointer) in [
            (
                r#"{"visual_anchors":{"scenes":[{"weather":null,"weather":null}]}}"#,
                "/visual_anchors/scenes/0/weather",
            ),
            (
                r#"{"visual_anchors":{"scenes":[{"weather":null,"\u0077eather":"rain"}]}}"#,
                "/visual_anchors/scenes/0/weather",
            ),
            (
                r#"{"a/b~c":{"weather":null,"weather":"rain"}}"#,
                "/a~1b~0c/weather",
            ),
        ] {
            let error = parse_strict_json_value(raw).unwrap_err();
            assert!(error.to_string().contains(pointer), "{error}");
            assert!(error.line() > 0 && error.column() > 0);
        }
        parse_strict_json_value(
            r#"{"scenes":[{"weather":null},{"weather":"rain"}],"description":"weather, weather"}"#,
        )
        .unwrap();
    }
}
