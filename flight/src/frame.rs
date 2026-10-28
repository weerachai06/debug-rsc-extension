//! Byte-exact row framing, mirroring React's `processBinaryChunk`.
//!
//! A row is `<hex id>:<tag?><payload>`. The payload runs to the next `\n`, except for the length-prefixed
//! tags, whose payload is `<hex byte length>,<that many bytes>` with no trailing newline.
//! State carries across calls, so a row may be split anywhere, even inside a multi-byte character.

const LENGTH_PREFIXED: &[u8] = b"TAOobUSsLlGgMmV";

#[derive(Clone, Copy, Default, PartialEq)]
enum Phase {
    #[default]
    Id,
    Tag,
    Length,
    ByNewline,
    ByLength,
}

#[derive(Default)]
pub struct FrameState {
    phase: Phase,
    id: u32,
    tag: u8,
    length: u32,
    buf: Vec<u8>,
    started: bool,
    row_offset: u32,
    offset: u32,
}

impl FrameState {
    /// True between rows, so the stream may end here without losing anything.
    pub fn is_idle(&self) -> bool {
        self.phase == Phase::Id && !self.started
    }

    /// Stream offset of the row being read, for reporting a truncated one.
    pub fn row_offset(&self) -> u32 {
        self.row_offset
    }
}

pub struct RawRow {
    pub id: u32,
    /// 0 for a JSON model row, else the tag byte.
    pub tag: u8,
    pub payload: Vec<u8>,
    pub offset: u32,
}

pub struct Fault {
    pub offset: u32,
    pub message: &'static str,
}

fn finish(st: &mut FrameState, rows: &mut Vec<RawRow>) {
    rows.push(RawRow { id: st.id, tag: st.tag, payload: std::mem::take(&mut st.buf), offset: st.row_offset });
    (st.phase, st.id, st.tag, st.length, st.started) = (Phase::Id, 0, 0, 0, false);
}

fn digit(b: u8, offset: u32, faults: &mut Vec<Fault>) -> u32 {
    match b {
        b'0'..=b'9' => u32::from(b - b'0'),
        b'a'..=b'f' => u32::from(b - b'a' + 10),
        _ => {
            faults.push(Fault { offset, message: "invalid hex digit in row header" });
            0
        }
    }
}

pub fn frame(mut st: FrameState, bytes: &[u8]) -> (FrameState, Vec<RawRow>, Vec<Fault>) {
    let (mut rows, mut faults) = (vec![], vec![]);
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let at = st.offset + i as u32;
        match st.phase {
            Phase::Id => {
                i += 1;
                // Next prepends '#' (complete) or '~' (partial) to prerendered responses. It is not Flight.
                if at == 0 && (b == b'#' || b == b'~') {
                    continue;
                }
                if !st.started {
                    (st.started, st.row_offset) = (true, at);
                }
                if b == b':' {
                    st.phase = Phase::Tag;
                } else {
                    st.id = (st.id << 4) | digit(b, at, &mut faults);
                }
            }
            Phase::Tag => {
                if LENGTH_PREFIXED.contains(&b) {
                    (st.tag, st.phase) = (b, Phase::Length);
                    i += 1;
                } else if b.is_ascii_uppercase() || b == b'r' || b == b'x' || b == b'#' {
                    (st.tag, st.phase) = (b, Phase::ByNewline);
                    i += 1;
                } else {
                    // Not a tag: the byte is the first of a JSON model row.
                    (st.tag, st.phase) = (0, Phase::ByNewline);
                }
            }
            Phase::Length => {
                i += 1;
                if b == b',' {
                    st.phase = Phase::ByLength;
                    if st.length == 0 {
                        finish(&mut st, &mut rows);
                    }
                } else {
                    st.length = (st.length << 4) | digit(b, at, &mut faults);
                }
            }
            Phase::ByNewline => match bytes[i..].iter().position(|&c| c == b'\n') {
                Some(n) => {
                    st.buf.extend_from_slice(&bytes[i..i + n]);
                    i += n + 1;
                    finish(&mut st, &mut rows);
                }
                None => {
                    st.buf.extend_from_slice(&bytes[i..]);
                    i = bytes.len();
                }
            },
            Phase::ByLength => {
                let take = (st.length as usize - st.buf.len()).min(bytes.len() - i);
                st.buf.extend_from_slice(&bytes[i..i + take]);
                i += take;
                if st.buf.len() == st.length as usize {
                    finish(&mut st, &mut rows);
                }
            }
        }
    }
    st.offset += bytes.len() as u32;
    (st, rows, faults)
}
