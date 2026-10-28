//! Substring or exact search over every row's own value. It does not follow references, so it is
//! linear and cycle-free, and every hit carries its canonical location.

use serde_json::Value;

use crate::decode::Row;
use crate::graph::Cursor;
use crate::value;
use crate::{Hit, Loc, Query, QueryField, Seg, StreamId};

struct Run<'q> {
    query: &'q Query,
    needle: String,
    hits: Vec<Hit>,
}

impl Run<'_> {
    fn matches(&self, text: &str) -> bool {
        if self.query.exact { text == self.query.text } else { text.to_lowercase().contains(&self.needle) }
    }

    fn walk(&mut self, at: &mut Loc, key: Option<&Seg>, v: &Value) {
        if self.hits.len() >= self.query.limit as usize {
            return;
        }
        let by_key = !matches!(self.query.field, QueryField::Value) && matches!(key, Some(Seg::Key(k)) if self.matches(k));
        let by_value = !by_key && !matches!(self.query.field, QueryField::Key) && leaf_text(v).is_some_and(|t| self.matches(&t));
        if by_key || by_value {
            self.hits.push(Hit { at: at.clone(), key: key.cloned(), preview: value::preview(v) });
        }
        for (seg, child) in value::entries(v) {
            at.path.push(seg.clone());
            self.walk(at, Some(&seg), child);
            at.path.pop();
        }
    }
}

fn leaf_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

pub fn search<'a>(streams: impl Iterator<Item = (&'a StreamId, &'a [Row])>, query: &Query) -> Vec<Hit> {
    let mut run = Run { query, needle: query.text.to_lowercase(), hits: vec![] };
    if query.text.is_empty() {
        return vec![];
    }
    for (stream, rows) in streams {
        if query.scope.as_ref().is_some_and(|s| s.stream != *stream) {
            continue;
        }
        for (index, row) in rows.iter().enumerate().filter(|(_, r)| r.tag.is_searchable()) {
            let Some(mut cursor) = Cursor::new(rows, index) else { continue };
            if let Some(scope) = &query.scope {
                if scope.row as usize != index {
                    continue;
                }
                match scope.path.iter().try_fold(cursor, |c, seg| c.step(seg)) {
                    Some(c) => cursor = c,
                    None => continue,
                }
            }
            let mut at = Loc { stream: stream.clone(), row: index as u32, id: row.id, path: cursor.path().to_vec() };
            run.walk(&mut at, None, cursor.value());
        }
    }
    run.hits
}
