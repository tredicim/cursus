//! The published JSON Schema.
//!
//! The schema is generated from the Rust types with `schemars`, so it cannot
//! drift from the parser. It covers both entry and target documents.

use schemars::schema_for;
use serde_json::Value;

use crate::model::Document;

/// Return the JSON Schema as pretty-printable JSON.
pub fn json_schema() -> Value {
    let mut schema =
        serde_json::to_value(schema_for!(Document)).expect("the generated schema is JSON");

    if let Some(object) = schema.as_object_mut() {
        object.insert(
            "$id".to_owned(),
            Value::String("https://github.com/tredicim/cursus/blob/main/schema.json".to_owned()),
        );
        object.insert(
            "title".to_owned(),
            Value::String("Cursus document".to_owned()),
        );
    }

    schema
}
