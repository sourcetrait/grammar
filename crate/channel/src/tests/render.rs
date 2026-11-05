use crate::*;

#[test]
fn escape_line_folds_a_multi_line_render_back_onto_one_line() {
    assert_eq!(escape_line("a\nb"), "a\\nb");
    assert_eq!(escape_line("a\r\nb"), "a\\r\\nb");
    assert_eq!(escape_line("plain"), "plain");
}

fn an_event() -> nu::Value {
    let span = nu::Span::unknown();
    let mut data = nu::Record::new();
    data.insert("msg", nu::Value::string("line one\nline two".to_string(), span));
    nu::Value::record(data, span)
}

fn an_id() -> MsgId {
    mint_msg_id(&datum::NonceGenerator::new(), "mcp", "mcp/channel/Open", "{}", None)
}

#[test]
fn a_rendered_packet_is_one_line_and_parses_back() {
    let event = an_event();
    let line = render_packet(an_id(), "mcp", "mcp/channel/Open", &event, None).expect("render");
    assert!(!line.contains('\n'), "a packet must never span lines: {line:?}");

    let value = nuon::from_nuon(&line, None).expect("valid NUON record");
    let record = value.as_record().expect("record");
    assert!(
        record.get("id").and_then(|v| v.as_str().ok()).is_some_and(|s| !s.is_empty()),
        "every message carries an id",
    );
    assert_eq!(record.get("from").and_then(|v| v.as_str().ok()), Some("mcp"));
    assert_eq!(
        record.get("model").and_then(|v| v.as_str().ok()),
        Some("mcp/channel/Open"),
    );
    assert!(
        record.get("attached").is_none(),
        "attached is omitted when there is nothing attached, never sent empty",
    );
    let event = record.get("event").expect("event").as_record().expect("record");
    assert_eq!(
        event.get("msg").and_then(|v| v.as_str().ok()),
        Some("line one\nline two"),
        "the escape must round-trip to the original string, not merely survive framing",
    );
}

#[test]
fn an_attachment_is_carried_by_its_names() {
    let names = vec![ATTACHED_FILE.to_string()];
    let line = render_packet(an_id(), "thread/abc", "foo/bar/Car", &an_event(), Some(&names))
        .expect("render");
    let value = nuon::from_nuon(&line, None).expect("valid NUON record");
    let record = value.as_record().expect("record");
    let attached: Vec<&str> = record
        .get("attached")
        .and_then(|v| v.as_list().ok())
        .expect("a list")
        .iter()
        .filter_map(|v| v.as_str().ok())
        .collect();
    assert_eq!(
        attached,
        vec![ATTACHED_FILE],
        "the wire names the files in the packet's own directory, no path",
    );
    assert_eq!(
        record.get("from").and_then(|v| v.as_str().ok()),
        Some("thread/abc"),
    );
}

#[test]
fn ids_are_distinct_for_identical_packets() {
    let nonce_gen = datum::NonceGenerator::new();
    let a = mint_msg_id(&nonce_gen, "mcp", "mcp/channel/Open", "{}", None);
    let b = mint_msg_id(&nonce_gen, "mcp", "mcp/channel/Open", "{}", None);
    assert_ne!(
        a.to_string(),
        b.to_string(),
        "the generator mixes a counter and a timestamp, so two byte-identical packets \
         still get distinct ids",
    );
    assert!(cereal::is_base62(&a.to_string()), "ids render base62");
}

#[test]
fn an_event_overflows_past_the_cap_less_the_wrapper() {
    let fits = "x".repeat(NOTIFICATION_CAP - 120);
    assert!(!event_overflows(&fits), "an event that leaves the wrapper its room fits");
    assert!(
        event_overflows(&format!("{fits}x")),
        "one character more would push the wrapped line past the client's cap",
    );
}

#[test]
fn a_spilled_event_carries_its_file_size_alone() {
    let pointer = event_spill_pointer(4096);
    let record = pointer.as_record().expect("record");
    assert_eq!(record.columns().map(String::as_str).collect::<Vec<_>>(), vec![EVENT_SIZE_KEY], "no path rides the wire");
    assert_eq!(record.get(EVENT_SIZE_KEY).and_then(|v| v.as_filesize().ok()).map(|size| size.get()), Some(4096));
}
