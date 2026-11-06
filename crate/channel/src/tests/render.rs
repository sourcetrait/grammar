use crate::*;

#[test]
fn escape_line_folds_a_multi_line_render_back_onto_one_line() {
    assert_eq!(escape_line("a\nb"), "a\\nb");
    assert_eq!(escape_line("a\r\nb"), "a\\r\\nb");
    assert_eq!(escape_line("plain"), "plain");
}

/// Data of one text field holding two lines.
fn some_data() -> nu::Value {
    let span = nu::Span::unknown();
    let mut data = nu::Record::new();
    data.insert("msg", nu::Value::string("line one\nline two".to_string(), span));
    nu::Value::record(data, span)
}

fn an_id() -> MsgId {
    mint_msg_id(&datum::NonceGenerator::new(), "mcp", "mcp/channel/Open", "{}", None)
}

/// One attached file's entry.
fn a_file(
    file: &str,
    size: u64,
) -> AttachedFile {
    AttachedFile {
        file: file.to_string(),
        size,
    }
}

/// Data of one text field `n` characters long.
fn data_of(n: usize) -> nu::Value {
    let span = nu::Span::unknown();
    let mut data = nu::Record::new();
    data.insert("text", nu::Value::string("t".repeat(n), span));
    nu::Value::record(data, span)
}

/// A rendered line read back as its record.
fn record_of(line: &str) -> nu::Record {
    nuon::from_nuon(line, None).expect("NUON").as_record().expect("record").clone()
}

/// The bytes of a relay's table as `<id>.attached.nuon` holds it.
fn table_bytes(files: &[AttachedFile]) -> u64 {
    render_nuon(&attached_table(files)).expect("render").len() as u64
}

/// A field's filesize, none where it holds anything else.
fn size_of(
    record: &nu::Record,
    field: &str,
) -> Option<i64> {
    record.get(field).and_then(|v| v.as_filesize().ok()).map(|size| size.get())
}

/// A record's columns in order.
fn columns(record: &nu::Record) -> Vec<&str> {
    record.columns().map(String::as_str).collect()
}

#[test]
fn a_packet_with_nothing_attached_is_the_envelope_s_first_variant() {
    let rendered = render_packet(an_id(), "mcp", "mcp/channel/Open", &some_data(), None).expect("render");
    assert!(!rendered.line.contains('\n'), "a packet never spans lines: {:?}", rendered.line);
    assert_eq!(rendered.data, None, "nothing spilled, so no file");
    let record = record_of(&rendered.line);
    assert_eq!(columns(&record), vec!["id", "from", "model", "data"]);
    assert!(
        record.get("id").and_then(|v| v.as_str().ok()).is_some_and(|s| !s.is_empty()),
        "every packet carries an id",
    );
    assert_eq!(record.get("from").and_then(|v| v.as_str().ok()), Some("mcp"));
    assert_eq!(record.get("model").and_then(|v| v.as_str().ok()), Some("mcp/channel/Open"));
    let data = record.get("data").and_then(|v| v.as_record().ok()).expect("data");
    assert_eq!(
        data.get("msg").and_then(|v| v.as_str().ok()),
        Some("line one\nline two"),
        "the escape round-trips to the original string",
    );
}

#[test]
fn a_relay_s_attachment_is_the_table_of_its_files() {
    let files = [a_file("notes.nuon", 42)];
    let attached = Attached::Files {
        files: &files,
        bytes: table_bytes(&files),
    };
    let rendered = render_packet(an_id(), "remote/channel/peer", "foo/bar/Car", &some_data(), Some(attached))
        .expect("render");
    let record = record_of(&rendered.line);
    assert_eq!(columns(&record), vec!["id", "from", "model", "data", "attached"]);
    let rows = record.get("attached").and_then(|v| v.as_list().ok()).expect("a table");
    assert_eq!(rows.len(), 1);
    let row = rows[0].as_record().expect("a row");
    assert_eq!(row.get("file").and_then(|v| v.as_str().ok()), Some("notes.nuon"), "each file by its path in `<id>/`");
    assert_eq!(size_of(row, "size"), Some(42));
}

#[test]
fn a_local_attachment_is_its_file_s_size() {
    let attached = Attached::Value { bytes: 42 };
    let rendered = render_packet(an_id(), "thread/abc", "foo/Bar", &some_data(), Some(attached)).expect("render");
    let record = record_of(&rendered.line);
    assert_eq!(size_of(&record, "attached"), Some(42), "a local value rides as its file's size");
    assert!(record.get("data").and_then(|v| v.as_record().ok()).is_some(), "its data inline");
}

#[test]
fn a_line_past_the_cap_spills_its_attached_table_first() {
    let files = [a_file(&format!("{}.txt", "n".repeat(NOTIFICATION_CAP)), 6)];
    let bytes = table_bytes(&files);
    let attached = Attached::Files { files: &files, bytes };
    let rendered = render_packet(an_id(), "remote/channel/peer", "foo/Bar", &some_data(), Some(attached))
        .expect("render");
    assert!(rendered.line.chars().count() <= NOTIFICATION_CAP, "the line fits the cap");
    assert_eq!(rendered.data, None, "the table's spill was enough, so the data stays");
    let record = record_of(&rendered.line);
    assert_eq!(size_of(&record, "attached"), Some(bytes as i64), "the table as its file's size");
    assert!(record.get("data").and_then(|v| v.as_record().ok()).is_some(), "the data inline");
}

#[test]
fn a_line_still_past_the_cap_spills_its_data_too() {
    let files = [a_file("notes.nuon", 6)];
    let bytes = table_bytes(&files);
    let attached = Attached::Files { files: &files, bytes };
    let rendered = render_packet(an_id(), "remote/channel/peer", "foo/Bar", &data_of(NOTIFICATION_CAP), Some(attached))
        .expect("render");
    assert!(rendered.line.chars().count() <= NOTIFICATION_CAP, "the line fits the cap");
    let text = rendered.data.as_deref().expect("the data spilled");
    let spilled = nuon::from_nuon(text, None).expect("NUON");
    let field = spilled.as_record().expect("the data alone").get("text").and_then(|v| v.as_str().ok());
    assert_eq!(field.map(str::len), Some(NOTIFICATION_CAP), "the file holds the data whole");
    let record = record_of(&rendered.line);
    assert_eq!(size_of(&record, "data"), Some(text.len() as i64), "the data as its file's size");
    assert_eq!(size_of(&record, "attached"), Some(bytes as i64), "attached spilled before it");
}

#[test]
fn the_envelope_fits_spilled_at_the_longest_model_and_alias() {
    let from = format!("{}/thread/{}", "a".repeat(128), "n".repeat(11));
    let model = "m".repeat(sourcetrait_grammar_protocol::MODEL_MAX_CHARS);
    let files = [a_file(&"f".repeat(NOTIFICATION_CAP), 6)];
    let attached = Attached::Files {
        files: &files,
        bytes: table_bytes(&files),
    };
    let rendered = render_packet(an_id(), &from, &model, &data_of(NOTIFICATION_CAP), Some(attached)).expect("render");
    let count = rendered.line.chars().count();
    assert!(count <= NOTIFICATION_CAP, "the spilled envelope fits: {count}");
    let refused = render_packet(an_id(), "thread/abc", &"m".repeat(NOTIFICATION_CAP), &data_of(1), None);
    assert!(
        refused.is_err_and(|e| e.contains("cap")),
        "an envelope past the cap is refused, never truncated",
    );
}

#[test]
fn a_line_fits_by_its_fields_its_id_counted_at_its_longest() {
    let files = [a_file("message.md", 11)];
    let attached = Attached::Files {
        files: &files,
        bytes: table_bytes(&files),
    };
    let fits = |n: usize| {
        line_fits("remote/channel/peer", "channel/Chat", &data_of(n), Some(attached)).expect("render")
    };
    let longest = (0..NOTIFICATION_CAP).take_while(|n| fits(*n)).last().expect("small data fits");
    assert!(!fits(longest + 1), "one character more passes the cap");
    let worst = (0..64u64)
        .map(|n| {
            let id = mint_msg_id(&datum::NonceGenerator::new(), "x", "y", &n.to_string(), None);
            let rendered = render_packet(id, "remote/channel/peer", "channel/Chat", &data_of(longest), Some(attached))
                .expect("render");
            assert_eq!(rendered.data, None, "data the check passed stays on the line");
            rendered.line.chars().count()
        })
        .max()
        .expect("lines");
    assert!(worst <= NOTIFICATION_CAP, "a line the check passed fits whatever id is minted: {worst}");
}

#[test]
fn a_spill_is_a_bare_filesize() {
    let size = spill_size(4096);
    assert_eq!(size.as_filesize().ok().map(|size| size.get()), Some(4096));
    assert!(size.as_record().is_err(), "no record, so no real data reads as one");
}

#[test]
fn a_packet_s_files_are_named_beside_it() {
    assert_eq!(data_file("abc"), "abc.data.nuon");
    assert_eq!(attached_file("abc"), "abc.attached.nuon");
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
