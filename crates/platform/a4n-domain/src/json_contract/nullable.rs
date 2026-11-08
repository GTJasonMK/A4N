use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// A required wire field whose value may be null. Ordinary Option fields may be absent.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(transparent)]
pub struct RequiredNullable<T>(pub Option<T>);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for RequiredNullable<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Required<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Required<T> {
            type Value = RequiredNullable<T>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a present field, possibly null")
            }
            fn visit_newtype_struct<D: serde::Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                Option::<T>::deserialize(deserializer).map(RequiredNullable)
            }
        }
        // MissingFieldDeserializer accepts deserialize_option as None. The
        // newtype boundary requires the field before decoding its nullable value.
        deserializer
            .deserialize_newtype_struct("RequiredNullable", Required(std::marker::PhantomData))
    }
}

impl<T: JsonSchema> JsonSchema for RequiredNullable<T> {
    fn schema_name() -> Cow<'static, str> {
        format!("RequiredNullable_{}", T::schema_name()).into()
    }
    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        <Option<T>>::json_schema(generator)
    }
    fn inline_schema() -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Deserialize, JsonSchema)]
    struct Example {
        #[allow(dead_code)]
        required: RequiredNullable<String>,
        #[allow(dead_code)]
        optional: Option<String>,
    }

    #[test]
    fn schema_and_serde_distinguish_missing_from_null() {
        let schema = super::super::typed_output_schema::<Example>();
        assert_eq!(schema["required"], json!(["required"]));
        let validator = super::super::OutputSchemaValidator::new(&schema).unwrap();
        for (value, expected) in [
            (json!({"required":null}), true),
            (json!({"required":"value"}), true),
            (json!({}), false),
            (json!({"required":3}), false),
        ] {
            assert_eq!(validator.validate(&value).is_ok(), expected);
            assert_eq!(serde_json::from_value::<Example>(value).is_ok(), expected);
        }
    }
}
