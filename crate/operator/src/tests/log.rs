use crate::*;

fn a_hang() -> Emergency {
    Emergency::HungEngineThread(HungEngineThreadEmergency {
        nonce: "abc123".to_string(),
        lane: "stateless".to_string(),
        tool: "execute".to_string(),
        started_at: 1000,
        cancelled_at: 2000,
        hung_ms: 6000,
        pool_held: 4,
        pool_cap: 8,
    })
}

/// A line's record, read back through nuval.
fn record(line: &str) -> Vec<(String, NuVal)> {
    match NuVal::from_nuon(line).expect("valid NUON") {
        NuVal::Record(fields) => fields,
        other => panic!("a record, not {other:?}"),
    }
}

/// A record's string field.
fn string<'a>(
    rec: &'a [(String, NuVal)],
    key: &str,
) -> Option<&'a str> {
    rec.iter().find(|(name, _)| name == key).and_then(|(_, value)| match value {
        NuVal::String(s) => Some(s.as_str()),
        _ => None,
    })
}

#[test]
fn a_line_is_one_record_on_one_line() {
    let line = emergency_line(&a_hang(), 9999, "nomAAA", None);
    assert!(!line.contains('\n'), "an emergency line must not span lines: {line:?}");
    let rec = record(&line);
    assert_eq!(string(&rec, "kind"), Some("hung_engine_thread"));
    assert_eq!(string(&rec, "mcp_nom"), Some("nomAAA"));
    assert_eq!(string(&rec, "nonce"), Some("abc123"));
    assert_eq!(string(&rec, "lane"), Some("stateless"));
    assert_eq!(string(&rec, "tool"), Some("execute"));
    assert!(rec.iter().all(|(name, _)| name != "host"), "a host's own emergency names no host");
    let names: Vec<&str> = rec.iter().take(3).map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["ts", "mcp_nom", "kind"], "the envelope leads");
}

/// A remote engine's emergency names its host by alias, beside the nom.
#[test]
fn a_remote_engines_line_names_its_host() {
    let line = emergency_line(&a_hang(), 9999, "nomAAA", Some("lab"));
    assert!(line.starts_with("{ts: 9999, mcp_nom: nomAAA, kind: hung_engine_thread, host: lab"), "{line}");
    let rec = record(&line);
    assert_eq!(string(&rec, "host"), Some("lab"));
    assert_eq!(string(&rec, "mcp_nom"), Some("nomAAA"));
    assert_eq!(string(&rec, "nonce"), Some("abc123"));
}

/// A newline in a field stays escaped on the line and reads back whole.
#[test]
fn a_fields_newline_stays_on_one_line() {
    let spam = Emergency::ChannelSpamError(ChannelSpamErrorEmergency {
        origin: "thread/abc".into(),
        hits: 15,
        window_secs: 10,
        rate: 15,
        action: "stopped\r\nagain".into(),
    });
    let line = emergency_line(&spam, 1, "nom", None);
    assert!(!line.contains('\n') && !line.contains('\r'), "{line:?}");
    assert_eq!(string(&record(&line), "action"), Some("stopped\r\nagain"));
}

/// The announced record leads with the host and carries no envelope.
#[test]
fn the_announced_record_is_the_fields_alone() {
    let NuVal::Record(fields) = announced_record(&a_hang(), Some("lab")) else { panic!("a record") };
    assert_eq!(fields.first().map(|(name, _)| name.as_str()), Some("host"));
    for envelope in ["ts", "mcp_nom", "kind", "from"] {
        assert!(fields.iter().all(|(name, _)| name != envelope), "{envelope} in the announcement");
    }
}

#[test]
fn the_log_sits_under_the_cache_home_by_nom() {
    let path = emergency_log_path(Path::new("cache"), "nomAAA");
    assert_eq!(path, Path::new("cache").join("log").join("nomAAA").join("emergency.nuonl"));
}
