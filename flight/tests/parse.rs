//! The parser is tested only through `Session`: feed chunks, read `overview`, `children`, `search`.

use base64::{Engine, engine::general_purpose::STANDARD};
use flight::{Body, Chunk, DiagCode, Loc, Node, NodeKind, Origin, Query, QueryField, RowTag, Seg, Session, StreamId, StreamState};

fn chunk(body: Body) -> Chunk {
    Chunk { stream: StreamId("s".into()), body }
}

fn open() -> Chunk {
    chunk(Body::Open { origin: Origin::Document { url: "https://example.com/".into() } })
}

fn parse(text: &str) -> Session {
    let mut session = Session::default();
    session.feed(open());
    session.feed(chunk(Body::Text { data: text.into() }));
    session.feed(chunk(Body::End));
    session
}

/// Same bytes, split in two at `at`, as Base64 so the split may fall inside a multi-byte character.
fn parse_split(bytes: &[u8], at: usize) -> Session {
    let mut session = Session::default();
    session.feed(open());
    session.feed(chunk(Body::Base64 { data: STANDARD.encode(&bytes[..at]) }));
    session.feed(chunk(Body::Base64 { data: STANDARD.encode(&bytes[at..]) }));
    session.feed(chunk(Body::End));
    session
}

fn rows(session: &Session) -> Vec<flight::RowSummary> {
    session.overview().streams[0].rows.clone()
}

/// The first row with this id that is not debug info (debug rows share the id they annotate).
fn root(session: &Session, id: u32) -> Loc {
    rows(session).into_iter().find(|r| r.id == id && r.tag != RowTag::Debug).expect("row").at
}

fn kids(session: &Session, at: &Loc) -> Vec<Node> {
    session.children(at)
}

fn key(node: &Node) -> String {
    match &node.key {
        Some(Seg::Key(k)) => k.clone(),
        Some(Seg::Index(i)) => i.to_string(),
        None => String::new(),
    }
}

fn child<'a>(nodes: &'a [Node], name: &str) -> &'a Node {
    nodes.iter().find(|n| key(n) == name).unwrap_or_else(|| panic!("no child {name} in {:?}", nodes.iter().map(key).collect::<Vec<_>>()))
}

fn diagnostics(session: &Session) -> Vec<DiagCode> {
    session.overview().streams[0].diagnostics.iter().map(|d| d.code.clone()).collect()
}

/// Everything observable about a session, for comparing two ways of feeding the same bytes.
fn dump(session: &Session) -> String {
    let overview = session.overview();
    let mut out = serde_json::to_string(&overview).unwrap();
    for row in &overview.streams[0].rows {
        out += &serde_json::to_string(&session.children(&row.at)).unwrap();
    }
    out
}

const SPLIT_VECTOR: &str = "1:T3,é\n0:{\"t\":\"$1\",\"p\":\"$2:a:b\",\"q\":\"$$lit\"}\n2:{\"a\":{\"b\":42}}\n";

#[test]
fn text_rows_are_framed_by_byte_length_not_by_newline() {
    let session = parse(SPLIT_VECTOR);

    let rows = rows(&session);
    assert_eq!(rows.iter().map(|r| (r.id, r.tag.clone(), r.bytes)).collect::<Vec<_>>(), [
        (1, RowTag::Text, 3),
        (0, RowTag::Model, 35),
        (2, RowTag::Model, 14),
    ]);
    assert_eq!(diagnostics(&session), []);
    assert_eq!(session.overview().streams[0].state, StreamState::Ended);
}

#[test]
fn feeding_at_any_split_gives_the_same_result() {
    let bytes = SPLIT_VECTOR.as_bytes();
    let whole = dump(&parse(SPLIT_VECTOR));

    for at in 0..=bytes.len() {
        assert_eq!(dump(&parse_split(bytes, at)), whole, "split at byte {at}");
    }
}

#[test]
fn next_marker_byte_before_the_first_row_is_not_a_row() {
    for marker in ['#', '~'] {
        let session = parse(&format!("{marker}0:\"x\"\n"));

        assert_eq!(rows(&session).len(), 1, "{marker}");
        assert_eq!(diagnostics(&session), [], "{marker}");
    }
}

#[test]
fn hints_have_an_empty_id_and_do_not_become_the_root() {
    let session = parse(":HL[\"/a.css\",\"style\"]\n0:null\n");

    let rows = rows(&session);
    assert_eq!((rows[0].id, rows[0].tag.clone()), (0, RowTag::Hint));
    assert!(rows[0].preview.starts_with('L'), "{}", rows[0].preview);
    assert_eq!(rows[1].tag, RowTag::Model);
}

#[test]
fn repeated_ids_are_separate_rows_and_references_skip_debug_rows() {
    let session = parse("9:D{\"time\":1}\n9:{\"k\":1}\n0:{\"r\":\"$9\"}\n");

    let rows = rows(&session);
    assert_eq!(rows.iter().map(|r| (r.id, r.at.row)).collect::<Vec<_>>(), [(9, 0), (9, 1), (0, 2)]);
    let r = child(&kids(&session, &root(&session, 0)), "r").at.clone();
    let target = kids(&session, &r);
    assert_eq!((key(&target[0]), target[0].at.row, target[0].at.id), ("k".into(), 1, 9));
}

#[test]
fn a_forward_reference_is_pending_until_its_row_arrives() {
    let mut session = Session::default();
    session.feed(open());
    session.feed(chunk(Body::Text { data: "0:{\"a\":\"$L1\"}\n".into() }));
    let a = child(&kids(&session, &root(&session, 0)), "a").at.clone();

    assert_eq!(kids(&session, &a)[0].kind, NodeKind::Pending);
    assert_eq!(child(&kids(&session, &root(&session, 0)), "a").target, None);

    session.feed(chunk(Body::Text { data: "1:[10,20]\n".into() }));
    let jump = child(&kids(&session, &root(&session, 0)), "a").target.clone();
    assert_eq!(jump.map(|t| (t.id, t.path)), Some((1, vec![])));
    let resolved = kids(&session, &a);
    assert_eq!(resolved.iter().map(|n| n.preview.as_str()).collect::<Vec<_>>(), ["10", "20"]);
}

#[test]
fn a_reference_that_never_arrives_is_dangling_once_the_stream_ended() {
    let session = parse("0:{\"a\":\"$L1\"}\n");
    let a = child(&kids(&session, &root(&session, 0)), "a").at.clone();

    assert_eq!(kids(&session, &a)[0].kind, NodeKind::Dangling);
}

#[test]
fn children_follow_references_and_report_the_canonical_location() {
    let session = parse("0:{\"p\":\"$2:a\"}\n2:{\"a\":{\"b\":42}}\n");
    let nodes = kids(&session, &root(&session, 0));
    let p = child(&nodes, "p");

    assert_eq!(p.kind, NodeKind::ModelRef);
    assert!(p.expandable);
    let target = p.target.as_ref().expect("a resolved reference has a target");
    assert_eq!((target.id, target.path.clone()), (2, vec![Seg::Key("a".into())]));
    let b = &kids(&session, &p.at)[0];
    assert_eq!((key(b), b.kind.clone(), b.preview.as_str()), ("b".into(), NodeKind::Scalar, "42"));
    assert_eq!((b.at.id, b.at.path.clone()), (2, vec![Seg::Key("a".into()), Seg::Key("b".into())]));
}

#[test]
fn elements_expose_type_key_props_and_paths_through_those_names() {
    let session = parse("1:[\"$\",\"b\",null,{\"children\":\"deep\"}]\n0:{\"r\":\"$1:props\"}\n");

    let element = kids(&session, &root(&session, 1));
    assert_eq!(element.iter().map(key).collect::<Vec<_>>(), ["type", "key", "props"]);
    let r = child(&kids(&session, &root(&session, 0)), "r").at.clone();
    assert_eq!(kids(&session, &r).iter().map(key).collect::<Vec<_>>(), ["children"]);
}

#[test]
fn reference_cycles_do_not_hang() {
    let session = parse("0:\"$0\"\n1:{\"self\":\"$1\"}\n");

    assert_eq!(kids(&session, &root(&session, 0))[0].kind, NodeKind::Dangling);
    let self_ref = child(&kids(&session, &root(&session, 1)), "self").at.clone();
    assert_eq!(kids(&session, &self_ref).iter().map(key).collect::<Vec<_>>(), ["self"]);
}

#[test]
fn every_sigil_gets_its_kind() {
    let session = parse(concat!(
        "0:{\"a\":\"$undefined\",\"b\":\"$D1970-01-01T00:00:00.000Z\",\"c\":\"$n10\",\"d\":\"$$x\",\"e\":\"$Sreact.fragment\",",
        "\"f\":\"$Q1\",\"g\":\"$W1\",\"h\":\"$@1\",\"i\":\"$L1\",\"j\":\"$h1\",\"k\":\"$5\",\"l\":\"$T\",\"m\":\"$NaN\",",
        "\"n\":\"$-0\",\"o\":\"$K1\",\"p\":\"$B1\",\"q\":\"$i1\",\"r\":\"$Z\",\"s\":\"$-Infinity\",\"t\":\"$Infinity\",\"u\":\"plain\"}\n",
        "1:[]\n"
    ));

    let nodes = kids(&session, &root(&session, 0));
    let kinds: Vec<_> = nodes.iter().map(|n| (key(n), n.kind.clone())).collect();
    use NodeKind::*;
    let want = [
        ("a", Special), ("b", Special), ("c", Special), ("d", Scalar), ("e", Special), ("f", Map), ("g", Set), ("h", Promise),
        ("i", Lazy), ("j", ServerRef), ("k", ModelRef), ("l", Temporary), ("m", Special), ("n", Special), ("o", FormData),
        ("p", Blob), ("q", Iterator), ("r", ErrorRef), ("s", Special), ("t", Special), ("u", Scalar),
    ];
    assert_eq!(kinds, want.map(|(k, kind)| (k.to_string(), kind)));
    assert_eq!(child(&nodes, "d").preview, "\"$x\"");
}

#[test]
fn streams_close_and_report_each_row_kind() {
    let session = parse("5:X\n5:300\n5:C\n1:R\n2:r\n3:x\n");

    assert_eq!(
        rows(&session).iter().map(|r| r.tag.clone()).collect::<Vec<_>>(),
        [RowTag::Stream, RowTag::Model, RowTag::Close, RowTag::Stream, RowTag::Stream, RowTag::Stream]
    );
}

#[test]
fn binary_rows_are_counted_in_bytes() {
    let mut session = Session::default();
    session.feed(open());
    session.feed(chunk(Body::Base64 { data: STANDARD.encode(b"5:o3,\x01\x02\x036:L4,\x01\x00\x00\x00") }));
    session.feed(chunk(Body::End));

    let rows = rows(&session);
    assert_eq!(rows.iter().map(|r| (r.id, r.tag.clone(), r.bytes)).collect::<Vec<_>>(), [(5, RowTag::Binary, 3), (6, RowTag::Binary, 4)]);
    assert!(rows[0].preview.contains("Uint8Array"), "{}", rows[0].preview);
    assert_eq!(diagnostics(&session), []);
}

#[test]
fn a_stream_that_ends_mid_row_is_truncated() {
    let session = parse("0:{\"x\":");

    assert_eq!(session.overview().streams[0].state, StreamState::Truncated);
    assert_eq!(diagnostics(&session), [DiagCode::BadFraming]);
}

#[test]
fn bad_json_and_unknown_tags_become_diagnostics_and_parsing_continues() {
    let session = parse("0:{oops\n1:Zabc\n2:7\n");

    assert_eq!(diagnostics(&session), [DiagCode::BadJson, DiagCode::UnknownTag]);
    assert_eq!(rows(&session).len(), 3);
    assert_eq!(rows(&session)[1].tag, RowTag::Unknown { ch: "Z".into() });
}

fn search(session: &Session, text: &str, field: QueryField, exact: bool, scope: Option<Loc>, limit: u32) -> Vec<(u32, String)> {
    session
        .search(&Query { text: text.into(), field, exact, scope, limit })
        .into_iter()
        .map(|h| (h.at.id, format!("{:?}", h.at.path)))
        .collect()
}

#[test]
fn search_matches_keys_and_values_like_find_key() {
    let session = parse("0:{\"filterData\":{\"brandName\":\"Nike\",\"brands\":[{\"name\":\"Nike\"}]}}\n1:{\"brands\":[]}\n");

    let keys = |text: &str, exact: bool| search(&session, text, QueryField::Key, exact, None, 100);
    assert_eq!(keys("brand", false).len(), 3);
    assert_eq!(keys("BRAND", false).len(), 3);
    assert_eq!(keys("brands", true), [(0, "[Key(\"filterData\"), Key(\"brands\")]".into()), (1, "[Key(\"brands\")]".into())]);
    assert_eq!(keys("brand", true), []);
    assert_eq!(search(&session, "nike", QueryField::Value, false, None, 100).len(), 2);
    assert_eq!(search(&session, "nike", QueryField::Any, false, None, 100).len(), 2);
    assert_eq!(search(&session, "brand", QueryField::Key, false, None, 1).len(), 1);
    assert_eq!(search(&session, "brand", QueryField::Key, false, Some(root(&session, 1)), 100), [(1, "[Key(\"brands\")]".into())]);
}

const PROD_FETCH: &str = include_str!("../../fixtures/next-16-prod-fetch.json");
const PROD_DOCUMENT: &str = include_str!("../../fixtures/next-16-prod-document.json");
const DEV_FETCH: &str = include_str!("../../fixtures/next-16-dev-fetch.json");
const DEV_DOCUMENT: &str = include_str!("../../fixtures/next-16-dev-document.json");

fn fixture(json: &str) -> Vec<Chunk> {
    serde_json::from_str(json).unwrap()
}

fn session_of(chunks: &[Chunk]) -> Session {
    let mut session = Session::default();
    chunks.iter().cloned().for_each(|c| session.feed(c));
    session
}

#[test]
fn real_next_16_payloads_parse_without_a_single_diagnostic() {
    for (name, json) in [("prod fetch", PROD_FETCH), ("prod document", PROD_DOCUMENT), ("dev fetch", DEV_FETCH), ("dev document", DEV_DOCUMENT)] {
        let stream = session_of(&fixture(json)).overview().streams.remove(0);

        assert_eq!(stream.state, StreamState::Ended, "{name}");
        assert_eq!(stream.diagnostics, [], "{name}");
        assert!(stream.rows.len() > 10, "{name}: {} rows", stream.rows.len());
    }
}

#[test]
fn real_payload_has_the_thai_text_row_and_the_server_values() {
    let session = session_of(&fixture(PROD_FETCH));

    let text: Vec<_> = rows(&session).into_iter().filter(|r| r.tag == RowTag::Text).collect();
    assert_eq!((text.len(), text[0].bytes), (1, 4800));

    let hits = session.search(&Query { text: "countDown".into(), field: QueryField::Key, exact: true, scope: None, limit: 10 });
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].preview, "1793465999");
    assert_eq!(hits[0].at.path.last(), Some(&Seg::Key("countDown".into())));
    let filter_data = &hits[0].at.path[..hits[0].at.path.len() - 1];
    assert_eq!(filter_data.last(), Some(&Seg::Key("filterData".into())));
}

#[test]
fn real_payload_survives_being_split_anywhere() {
    for json in [PROD_FETCH, DEV_FETCH, PROD_DOCUMENT] {
        let chunks = fixture(json);
        let bytes: Vec<u8> = chunks
            .iter()
            .filter_map(|c| match &c.body {
                Body::Text { data } => Some(data.as_bytes().to_vec()),
                _ => None,
            })
            .flatten()
            .collect();
        let whole = dump(&parse_split(&bytes, bytes.len()));

        for at in (0..bytes.len()).step_by(7) {
            assert_eq!(dump(&parse_split(&bytes, at)), whole, "split at byte {at}");
        }
    }
}
