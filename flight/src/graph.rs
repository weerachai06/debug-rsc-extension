//! Locations resolved through references, and the one-level child listing built on that.
//!
//! A path passes through references like React's `getOutlinedModel`: `$L5`, `$@5` and `$5:a:b`
//! are followed wherever they are met. Cycles are safe because children are listed one level at a time,
//! and a hop limit stops reference loops.

use std::collections::HashMap;

use serde_json::Value;

use crate::decode::Row;
use crate::value::{self, Sigil};
use crate::{Loc, Node, NodeKind, Seg, StreamId};

const MAX_HOPS: usize = 64;

pub struct View<'a> {
    pub stream: &'a StreamId,
    pub rows: &'a [Row],
    /// Id -> index of the first row a reference to that id lands on.
    pub by_id: &'a HashMap<u32, usize>,
    /// Rows may still arrive, so a missing target is pending rather than dangling.
    pub open: bool,
}

pub enum Target<'a> {
    Found(Loc, &'a Value),
    Pending(u32),
    Dangling(String),
    Stale,
}

#[derive(Clone)]
pub struct Cursor<'a> {
    row: usize,
    path: Vec<Seg>,
    value: &'a Value,
    hops: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(rows: &'a [Row], row: usize) -> Option<Self> {
        rows.get(row).map(|r| Cursor { row, path: vec![], value: &r.value, hops: 0 })
    }

    pub fn path(&self) -> &[Seg] {
        &self.path
    }

    pub fn value(&self) -> &'a Value {
        self.value
    }

    /// One step down, without following references. Numeric keys index arrays; element field names index tuples.
    pub fn step(mut self, seg: &Seg) -> Option<Self> {
        let (value, seg) = match (self.value, seg) {
            (Value::Object(m), Seg::Key(k)) => (m.get(k)?, seg.clone()),
            (Value::Object(m), Seg::Index(i)) => (m.get(&i.to_string())?, Seg::Key(i.to_string())),
            (Value::Array(a), Seg::Index(i)) => (a.get(*i as usize)?, seg.clone()),
            (Value::Array(a), Seg::Key(k)) => match value::element_field(k).filter(|_| value::is_element(a)) {
                Some(i) => (a.get(i)?, seg.clone()),
                None => {
                    let i: usize = k.parse().ok()?;
                    (a.get(i)?, Seg::Index(i as u32))
                }
            },
            _ => return None,
        };
        self.path.push(seg);
        self.value = value;
        Some(self)
    }

    fn follow(mut self, view: &View<'a>) -> Result<Self, Target<'a>> {
        while let Value::String(s) = self.value {
            let Sigil::Ref { id, path, .. } = value::sigil(s) else { break };
            self.hops += 1;
            if self.hops > MAX_HOPS {
                return Err(Target::Dangling("reference loop".into()));
            }
            let Some(&row) = view.by_id.get(&id) else {
                return Err(if view.open { Target::Pending(id) } else { Target::Dangling(format!("no row {id:x}")) });
            };
            let mut next = Cursor { row, path: vec![], value: &view.rows[row].value, hops: self.hops };
            for name in path {
                next = next.follow(view)?;
                next = next
                    .step(&Seg::Key(name.into()))
                    .ok_or_else(|| Target::Dangling(format!("no {name} in row {id:x}")))?;
            }
            self = next;
        }
        Ok(self)
    }

    fn loc(&self, view: &View<'_>) -> Loc {
        Loc { stream: view.stream.clone(), row: self.row as u32, id: view.rows[self.row].id, path: self.path.clone() }
    }
}

/// The value at `at`, with references followed at every step and at the end.
pub fn resolve<'a>(view: &View<'a>, at: &Loc) -> Target<'a> {
    let Some(mut cur) = Cursor::new(view.rows, at.row as usize) else { return Target::Stale };
    for seg in &at.path {
        cur = match cur.follow(view) {
            Ok(c) => c,
            Err(t) => return t,
        };
        cur = match cur.step(seg) {
            Some(c) => c,
            None => return Target::Stale,
        };
    }
    match cur.follow(view) {
        Ok(c) => Target::Found(c.loc(view), c.value),
        Err(t) => t,
    }
}

pub fn children(view: &View<'_>, at: &Loc) -> Vec<Node> {
    match resolve(view, at) {
        Target::Stale => vec![],
        Target::Pending(id) => vec![placeholder(at, NodeKind::Pending, format!("waiting for row {id:x}"))],
        Target::Dangling(why) => vec![placeholder(at, NodeKind::Dangling, why)],
        Target::Found(loc, value) => value::entries(value).into_iter().map(|(seg, child)| node(view, &loc, seg, child)).collect(),
    }
}

fn placeholder(at: &Loc, kind: NodeKind, preview: String) -> Node {
    Node { at: at.clone(), key: None, kind, preview, target: None, expandable: false }
}

fn node(view: &View<'_>, parent: &Loc, seg: Seg, value: &Value) -> Node {
    let mut at = parent.clone();
    at.path.push(seg.clone());
    let is_ref = matches!(value, Value::String(s) if matches!(value::sigil(s), Sigil::Ref { .. }));
    let (target, expandable) = if !is_ref {
        (None, !value::entries(value).is_empty())
    } else {
        match resolve(view, &at) {
            Target::Found(loc, v) => (Some(loc), !value::entries(v).is_empty()),
            // Expanding shows why: still pending, or dangling.
            Target::Pending(_) | Target::Dangling(_) => (None, true),
            Target::Stale => (None, false),
        }
    };
    Node { at, key: Some(seg), kind: value::classify(value), preview: value::preview(value), target, expandable }
}
