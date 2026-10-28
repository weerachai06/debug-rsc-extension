//! Turns a framed row into a typed value plus the short preview shown in the row list.

use serde_json::Value;

use crate::frame::RawRow;
use crate::value;
use crate::{DiagCode, RowTag};

pub struct Row {
    pub id: u32,
    pub tag: RowTag,
    /// JSON for most rows, the string for text rows, null for stream and binary rows.
    pub value: Value,
    /// Payload size, without the header.
    pub bytes: u32,
    pub preview: String,
}

pub struct Issue {
    pub code: DiagCode,
    pub message: String,
}

const BINARY: &[(u8, &str)] = &[
    (b'A', "ArrayBuffer"),
    (b'O', "Int8Array"),
    (b'o', "Uint8Array"),
    (b'b', "byte chunk"),
    (b'U', "Uint8ClampedArray"),
    (b'S', "Int16Array"),
    (b's', "Uint16Array"),
    (b'L', "Int32Array"),
    (b'l', "Uint32Array"),
    (b'G', "Float32Array"),
    (b'g', "Float64Array"),
    (b'M', "BigInt64Array"),
    (b'm', "BigUint64Array"),
    (b'V', "DataView"),
];

fn json(text: &str) -> (Value, Option<Issue>) {
    if text.is_empty() {
        return (Value::Null, None);
    }
    match serde_json::from_str(text) {
        Ok(v) => (v, None),
        Err(e) => (Value::String(text.into()), Some(Issue { code: DiagCode::BadJson, message: e.to_string() })),
    }
}

/// `:HL["/a.css","style"]` -> ["L", ["/a.css","style"]]
fn hint(text: &str) -> (Value, Option<Issue>) {
    let mut chars = text.chars();
    let code = chars.next().map(String::from).unwrap_or_default();
    let (payload, issue) = json(chars.as_str());
    (Value::Array(vec![Value::String(code), payload]), issue)
}

pub fn decode(raw: RawRow) -> (Row, Option<Issue>) {
    let bytes = raw.payload.len() as u32;
    let text = || String::from_utf8_lossy(&raw.payload).into_owned();
    let (tag, (value, issue)) = match raw.tag {
        0 => (RowTag::Model, json(&text())),
        b'I' => (RowTag::Module, json(&text())),
        b'E' => (RowTag::Error, json(&text())),
        b'D' => (RowTag::Debug, json(&text())),
        b'W' => (RowTag::Console, json(&text())),
        b'N' => (RowTag::Timing, json(&text())),
        b'J' => (RowTag::IoInfo, json(&text())),
        b'C' => (RowTag::Close, json(&text())),
        b'H' => (RowTag::Hint, hint(&text())),
        b'T' => (RowTag::Text, (Value::String(text()), None)),
        b'R' | b'r' | b'X' | b'x' => (RowTag::Stream, (Value::Null, None)),
        t if BINARY.iter().any(|(b, _)| *b == t) => (RowTag::Binary, (Value::Null, None)),
        t => {
            let ch = (t as char).to_string();
            let issue = Issue { code: DiagCode::UnknownTag, message: format!("unknown row tag {ch}") };
            (RowTag::Unknown { ch }, (json(&text()).0, Some(issue)))
        }
    };
    let preview = preview(raw.tag, &value, bytes);
    (Row { id: raw.id, tag, value, bytes, preview }, issue)
}

fn preview(tag: u8, value: &Value, bytes: u32) -> String {
    match tag {
        0 if bytes == 0 => "(halted)".into(),
        b'I' => module(value).unwrap_or_else(|| value::preview(value)),
        b'H' => match value {
            Value::Array(a) if a.len() == 2 => format!("{} {}", a[0].as_str().unwrap_or(""), value::preview(&a[1])),
            _ => value::preview(value),
        },
        b'E' => value
            .get("message")
            .or_else(|| value.get("digest"))
            .and_then(Value::as_str)
            .map_or_else(|| value::preview(value), str::to_string),
        b'R' => "ReadableStream".into(),
        b'r' => "ReadableStream (bytes)".into(),
        b'X' => "AsyncIterable".into(),
        b'x' => "AsyncIterator".into(),
        b'C' if value.is_null() => "close".into(),
        t => match BINARY.iter().find(|(b, _)| *b == t) {
            Some((_, name)) => format!("{name} {bytes} B"),
            None => value::preview(value),
        },
    }
}

/// `I[id, chunks, name, async?]` -> "id name"
fn module(value: &Value) -> Option<String> {
    let a = value.as_array()?;
    let id = match a.first()? {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    Some(format!("{id} {}", a.get(2)?.as_str()?))
}

impl RowTag {
    /// Whether a `$<id>` reference can land on a row of this kind. Debug, hint and control rows annotate ids.
    pub fn holds_chunk(&self) -> bool {
        matches!(self, RowTag::Model | RowTag::Module | RowTag::Text | RowTag::Error | RowTag::Stream | RowTag::Binary | RowTag::Unknown { .. })
    }

    pub fn is_searchable(&self) -> bool {
        matches!(self, RowTag::Model | RowTag::Module | RowTag::Text | RowTag::Error | RowTag::Unknown { .. })
    }
}
