mod decode;
mod frame;
mod graph;
mod search;
mod types;
mod value;
#[cfg(target_arch = "wasm32")]
mod wasm;

use std::collections::HashMap;

use base64::{Engine, engine::general_purpose::STANDARD};

use decode::Row;
use frame::FrameState;
pub use types::*;

/// Incremental view of captured Flight streams.
#[derive(Default)]
pub struct Session {
    streams: Vec<Stream>,
}

struct Stream {
    summary: StreamSummary,
    frame: FrameState,
    rows: Vec<Row>,
    by_id: HashMap<u32, usize>,
}

impl Session {
    /// Never fails. Anything unexpected becomes a Diagnostic on that stream.
    /// Expected, not required: per stream, Open first, End last, chunks in order.
    pub fn feed(&mut self, Chunk { stream, body }: Chunk) {
        let i = match self.streams.iter().position(|s| s.summary.id == stream) {
            Some(i) => i,
            None => {
                let mut s = Stream {
                    summary: StreamSummary {
                        id: stream,
                        origin: None,
                        state: StreamState::Open,
                        bytes: 0,
                        rows: vec![],
                        diagnostics: vec![],
                    },
                    frame: FrameState::default(),
                    rows: vec![],
                    by_id: HashMap::new(),
                };
                if !matches!(body, Body::Open { .. }) {
                    diagnose(&mut s.summary, DiagCode::ChunkBeforeOpen, "chunk arrived before Open");
                }
                self.streams.push(s);
                self.streams.len() - 1
            }
        };
        let s = &mut self.streams[i];
        if s.summary.state != StreamState::Open {
            diagnose(&mut s.summary, DiagCode::ChunkAfterEnd, "chunk arrived after End");
            return;
        }
        match body {
            Body::Open { origin } => s.summary.origin = Some(origin),
            Body::Text { data } => ingest(s, data.as_bytes()),
            Body::Base64 { data } => match STANDARD.decode(&data) {
                Ok(bytes) => ingest(s, &bytes),
                Err(e) => diagnose(&mut s.summary, DiagCode::InvalidBase64, &e.to_string()),
            },
            Body::End if s.frame.is_idle() => s.summary.state = StreamState::Ended,
            Body::End => {
                s.summary.state = StreamState::Truncated;
                let offset = s.frame.row_offset();
                s.summary.diagnostics.push(Diagnostic {
                    row: None,
                    offset,
                    code: DiagCode::BadFraming,
                    message: "stream ended in the middle of a row".into(),
                });
            }
            Body::FormState { .. } | Body::Other { .. } => {}
        }
    }

    /// Streams in first-seen order; rows in arrival order. An id can repeat: debug and stream rows share one.
    pub fn overview(&self) -> Overview {
        Overview { streams: self.streams.iter().map(|s| s.summary.clone()).collect() }
    }

    /// One level of the value at `at`, with references followed. Unknown or stale `at` gives nothing;
    /// a target that has not arrived (or never will) gives a single Pending (or Dangling) node.
    pub fn children(&self, at: &Loc) -> Vec<Node> {
        let Some(s) = self.streams.iter().find(|s| s.summary.id == at.stream) else { return vec![] };
        let view = graph::View {
            stream: &s.summary.id,
            rows: &s.rows,
            by_id: &s.by_id,
            open: s.summary.state == StreamState::Open,
        };
        graph::children(&view, at)
    }

    /// Visits each row's own value once and does not follow references.
    pub fn search(&self, query: &Query) -> Vec<Hit> {
        search::search(self.streams.iter().map(|s| (&s.summary.id, s.rows.as_slice())), query)
    }
}

fn ingest(s: &mut Stream, bytes: &[u8]) {
    let (frame, raw_rows, faults) = frame::frame(std::mem::take(&mut s.frame), bytes);
    s.frame = frame;
    s.summary.bytes += bytes.len() as u32;
    for fault in faults {
        s.summary.diagnostics.push(Diagnostic {
            row: None,
            offset: fault.offset,
            code: DiagCode::BadFraming,
            message: fault.message.into(),
        });
    }
    for raw in raw_rows {
        let offset = raw.offset;
        let (row, issue) = decode::decode(raw);
        let index = s.rows.len();
        if let Some(issue) = issue {
            s.summary.diagnostics.push(Diagnostic { row: Some(index as u32), offset, code: issue.code, message: issue.message });
        }
        s.summary.rows.push(RowSummary {
            id: row.id,
            tag: row.tag.clone(),
            bytes: row.bytes,
            preview: row.preview.clone(),
            at: Loc { stream: s.summary.id.clone(), row: index as u32, id: row.id, path: vec![] },
        });
        if row.tag.holds_chunk() {
            s.by_id.entry(row.id).or_insert(index);
        }
        s.rows.push(row);
    }
}

fn diagnose(stream: &mut StreamSummary, code: DiagCode, message: &str) {
    stream.diagnostics.push(Diagnostic { row: None, offset: stream.bytes, code, message: message.into() });
}
