//! What a JSON value means on the Flight wire: `$` sigils, element tuples, short previews.

use serde_json::Value;

use crate::{NodeKind, Seg};

const PREVIEW_MAX: usize = 80;

/// Positions 1..=6 of an element tuple. Reference paths use these names too (`$1:props:children`).
pub const ELEMENT_FIELDS: [&str; 6] = ["type", "key", "props", "owner", "stack", "validated"];

pub enum Sigil<'a> {
    /// An ordinary string, or `$$…` (an escaped literal).
    Plain,
    /// Literals with no target: undefined, NaN, ±Infinity, -0, Date, BigInt, Symbol, source, dev markers.
    Special,
    Temporary,
    /// Production `$Z`: an error whose details were stripped.
    BareError,
    /// Starts with `$` but is nothing React emits.
    Unknown,
    /// Points at row `id`, then walks `path`.
    Ref { kind: NodeKind, id: u32, path: Vec<&'a str> },
}

pub fn sigil(s: &str) -> Sigil<'_> {
    let Some(rest) = s.strip_prefix('$') else { return Sigil::Plain };
    let kind = match rest.as_bytes().first() {
        None | Some(b'$') => return Sigil::Plain,
        Some(b'u' | b'N' | b'I' | b'-' | b'D' | b'n' | b'S' | b'E' | b'Y' | b'P') => return Sigil::Special,
        Some(b'T') => return Sigil::Temporary,
        Some(b'Z') if rest.len() == 1 => return Sigil::BareError,
        Some(b'Z') => NodeKind::ErrorRef,
        Some(b'L') => NodeKind::Lazy,
        Some(b'@' | b'w') => NodeKind::Promise,
        Some(b'Q') => NodeKind::Map,
        Some(b'W') => NodeKind::Set,
        Some(b'K') => NodeKind::FormData,
        Some(b'B') => NodeKind::Blob,
        Some(b'i') => NodeKind::Iterator,
        Some(b'h' | b'F' | b'H') => NodeKind::ServerRef,
        Some(b'0'..=b'9' | b'a'..=b'f') => return reference(NodeKind::ModelRef, rest),
        Some(_) => return Sigil::Unknown,
    };
    reference(kind, &rest[1..])
}

fn reference(kind: NodeKind, rest: &str) -> Sigil<'_> {
    let mut parts = rest.split(':');
    let id = parts
        .next()
        .filter(|hex| !hex.is_empty() && hex.len() <= 8 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
        .and_then(|hex| u32::from_str_radix(hex, 16).ok());
    match id {
        Some(id) => Sigil::Ref { kind, id, path: parts.collect() },
        None => Sigil::Unknown,
    }
}

pub fn is_element(a: &[Value]) -> bool {
    a.len() >= 4 && a[0].as_str() == Some("$")
}

/// Index into an element tuple for a field name, e.g. "props" -> 3.
pub fn element_field(name: &str) -> Option<usize> {
    ELEMENT_FIELDS.iter().position(|f| *f == name).map(|i| i + 1)
}

pub fn classify(v: &Value) -> NodeKind {
    match v {
        Value::String(s) => match sigil(s) {
            Sigil::Plain => NodeKind::Scalar,
            Sigil::Special => NodeKind::Special,
            Sigil::Temporary => NodeKind::Temporary,
            Sigil::BareError => NodeKind::ErrorRef,
            Sigil::Unknown => NodeKind::Unknown,
            Sigil::Ref { kind, .. } => kind,
        },
        Value::Array(a) if is_element(a) => NodeKind::Element,
        Value::Array(_) => NodeKind::Array,
        Value::Object(_) => NodeKind::Object,
        _ => NodeKind::Scalar,
    }
}

/// The children of a value, in wire order. Elements list their named fields, not the "$" marker.
pub fn entries(v: &Value) -> Vec<(Seg, &Value)> {
    match v {
        Value::Object(m) => m.iter().map(|(k, v)| (Seg::Key(k.clone()), v)).collect(),
        Value::Array(a) if is_element(a) => {
            ELEMENT_FIELDS.iter().zip(&a[1..]).map(|(name, v)| (Seg::Key((*name).into()), v)).collect()
        }
        Value::Array(a) => a.iter().enumerate().map(|(i, v)| (Seg::Index(i as u32), v)).collect(),
        _ => vec![],
    }
}

fn clip(s: String) -> String {
    if s.chars().count() <= PREVIEW_MAX {
        return s;
    }
    s.chars().take(PREVIEW_MAX).chain(['…']).collect()
}

pub fn preview(v: &Value) -> String {
    clip(match v {
        Value::String(s) => match sigil(s) {
            Sigil::Plain => Value::String(s.strip_prefix('$').filter(|r| r.starts_with('$')).unwrap_or(s).into()).to_string(),
            _ => s.clone(),
        },
        Value::Array(a) if is_element(a) => {
            let ty = a[1].as_str().unwrap_or("?");
            match a[2].as_str() {
                Some(key) => format!("<{ty} key=\"{key}\">"),
                None => format!("<{ty}>"),
            }
        }
        Value::Array(a) => format!("Array({})", a.len()),
        Value::Object(m) => {
            let mut out = String::from("{");
            for (i, key) in m.keys().enumerate() {
                if out.chars().count() + key.chars().count() > PREVIEW_MAX - 4 {
                    out.push('…');
                    break;
                }
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(key);
            }
            out + "}"
        }
        other => other.to_string(),
    })
}
