use flight::{Body, Chunk, DiagCode, Origin, Session, StreamId, StreamState};

fn chunk(stream: &str, body: Body) -> Chunk {
    Chunk { stream: StreamId(stream.into()), body }
}

#[test]
fn fixture_stream_is_opened_counted_and_ended() {
    let chunks: Vec<Chunk> = serde_json::from_str(include_str!("../../fixtures/minimal.json")).unwrap();
    let mut session = Session::default();
    chunks.into_iter().for_each(|c| session.feed(c));

    let streams = session.overview().streams;
    assert_eq!(streams.len(), 1);
    assert_eq!(streams[0].id, StreamId("doc:1".into()));
    assert!(matches!(streams[0].origin, Some(Origin::Document { .. })));
    assert_eq!(streams[0].state, StreamState::Ended);
    assert_eq!(streams[0].bytes, 22);
    assert!(streams[0].diagnostics.is_empty());
}

#[test]
fn base64_bodies_count_decoded_bytes_and_bad_base64_is_a_diagnostic() {
    let mut session = Session::default();
    session.feed(chunk("s", Body::Base64 { data: "MDpudWxsCg==".into() }));
    session.feed(chunk("s", Body::Base64 { data: "!!".into() }));

    let s = &session.overview().streams[0];
    assert_eq!(s.bytes, 7);
    let codes: Vec<_> = s.diagnostics.iter().map(|d| &d.code).collect();
    assert_eq!(codes, [&DiagCode::ChunkBeforeOpen, &DiagCode::InvalidBase64]);
}

#[test]
fn chunks_after_end_are_dropped_with_a_diagnostic() {
    let mut session = Session::default();
    session.feed(chunk("s", Body::End));
    session.feed(chunk("s", Body::Text { data: "late".into() }));

    let s = &session.overview().streams[0];
    assert_eq!(s.bytes, 0);
    assert_eq!(s.diagnostics.last().map(|d| &d.code), Some(&DiagCode::ChunkAfterEnd));
}
