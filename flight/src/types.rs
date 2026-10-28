//! The contract with the TypeScript side. Every type here is exported by ts-rs.
//!
//! Field rules: only String, bool, u32, Vec, Option and enums with struct or unit variants.
//! No 64-bit ints, usize, f64 or HashMap, because JSON transport cannot honour what ts-rs emits for them.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StreamId(pub String);

/// One piece of a stream, as captured. Order within a stream is Open, any number of data chunks, End.
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Chunk {
    pub stream: StreamId,
    pub body: Body,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Body {
    Open { origin: Origin },
    Text { data: String },
    Base64 { data: String },
    FormState { json: String },
    End,
    Other { json: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Origin {
    Document { url: String },
    Fetch { url: String, method: String, status: u32 },
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Overview {
    pub streams: Vec<StreamSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StreamSummary {
    pub id: StreamId,
    pub origin: Option<Origin>,
    pub state: StreamState,
    pub bytes: u32,
    pub rows: Vec<RowSummary>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum StreamState {
    Open,
    Ended,
    Truncated,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RowSummary {
    pub id: u32,
    pub tag: RowTag,
    pub bytes: u32,
    pub preview: String,
    pub at: Loc,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RowTag {
    Model,
    Module,
    Hint,
    Error,
    Text,
    Debug,
    Console,
    Timing,
    IoInfo,
    Stream,
    Close,
    Binary,
    Unknown { ch: String },
}

/// A canonical address: a row of a stream, then a path of keys and indices into its value.
/// `row` is the arrival index, because ids repeat (debug and stream rows share the id they annotate);
/// `id` is the Flight id, for display and for `$<hex>` references.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Loc {
    pub stream: StreamId,
    pub row: u32,
    pub id: u32,
    pub path: Vec<Seg>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum Seg {
    Key(String),
    Index(u32),
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Node {
    pub at: Loc,
    pub key: Option<Seg>,
    pub kind: NodeKind,
    pub preview: String,
    pub target: Option<Loc>,
    pub expandable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum NodeKind {
    Scalar,
    Special,
    Array,
    Object,
    Element,
    Module,
    Text,
    Lazy,
    Promise,
    ModelRef,
    Map,
    Set,
    FormData,
    Blob,
    ServerRef,
    Iterator,
    ErrorRef,
    Temporary,
    Pending,
    Dangling,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Query {
    pub text: String,
    pub field: QueryField,
    pub exact: bool,
    pub scope: Option<Loc>,
    pub limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum QueryField {
    Key,
    Value,
    Any,
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Hit {
    pub at: Loc,
    pub key: Option<Seg>,
    pub preview: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Diagnostic {
    pub row: Option<u32>,
    pub offset: u32,
    pub code: DiagCode,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum DiagCode {
    BadFraming,
    BadJson,
    UnknownTag,
    UnknownSigil,
    ChunkBeforeOpen,
    ChunkAfterEnd,
    InvalidBase64,
}
