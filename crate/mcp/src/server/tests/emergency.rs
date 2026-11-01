use crate::*;
use crate::server::emergency::{emergency_line, emergency_model};
use sourcetrait_grammar_protocol::{
    BackgroundJobsWarningEmergency, ChannelSpamErrorEmergency, ChannelSpamWarningEmergency,
    CpuWarningEmergency, CriticalEmergency, DiskWarningEmergency, HungEngineThreadEmergency,
    MCP_RESERVED_PREFIX, RamWarningEmergency, VramWarningEmergency,
};

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

#[test]
fn to_nuon_line_is_single_line_and_parses() {
    let line = emergency_line(&a_hang(), 9999, "nomAAA", None).unwrap();
    assert!(!line.contains('\n'), "an emergency line must not span lines: {line:?}");
    let v = nu::from_nuon(&line, None).expect("valid NUON record");
    let rec = v.as_record().expect("record");
    assert_eq!(rec.get("kind").and_then(|x| x.as_str().ok()), Some("hung_engine_thread"));
    assert_eq!(rec.get("mcp_nom").and_then(|x| x.as_str().ok()), Some("nomAAA"));
    assert_eq!(rec.get("nonce").and_then(|x| x.as_str().ok()), Some("abc123"));
    assert_eq!(rec.get("lane").and_then(|x| x.as_str().ok()), Some("stateless"));
    assert_eq!(rec.get("tool").and_then(|x| x.as_str().ok()), Some("execute"));
    assert!(rec.get("host").is_none(), "the MCP's own emergency names no host");
}

/// A remote engine's emergency names its host by alias, beside the MCP's nom.
#[test]
fn a_remote_engines_line_names_its_host() {
    let line = emergency_line(&a_hang(), 9999, "nomAAA", Some("lab")).unwrap();
    let v = nu::from_nuon(&line, None).expect("valid NUON record");
    let rec = v.as_record().expect("record");
    assert_eq!(rec.get("host").and_then(|x| x.as_str().ok()), Some("lab"));
    assert_eq!(rec.get("mcp_nom").and_then(|x| x.as_str().ok()), Some("nomAAA"));
    assert_eq!(rec.get("nonce").and_then(|x| x.as_str().ok()), Some("abc123"));
}

/// No emergency's event names a field the packet's envelope carries.
#[test]
fn no_event_field_collides_with_the_packet_envelope() {
    for em in all_kinds() {
        let line = emergency_line(&em, 1, "nom", None).unwrap();
        let v = nu::from_nuon(&line, None).expect("valid NUON record");
        let rec = v.as_record().expect("record");
        assert!(
            rec.get("from").is_none(),
            "{} carries a `from` event field, which collides with the envelope's own",
            em.kind().name(),
        );
        if em.kind().name().starts_with("channel_spam") {
            assert!(
                rec.get("origin").is_some(),
                "{} must name the offending origin",
                em.kind().name(),
            );
        }
    }
}

/// Every emergency's model sits under the `mcp/` reservation, each its own.
#[test]
fn every_model_lives_under_the_mcp_reservation_and_is_distinct() {
    let models: Vec<&str> = all_kinds().iter().map(emergency_model).collect();
    for model in &models {
        assert!(
            model.starts_with(MCP_RESERVED_PREFIX),
            "{model} must live under the {MCP_RESERVED_PREFIX} reservation",
        );
    }
    let mut unique = models.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), models.len(), "models must be distinct: {models:?}");
}

/// One of every variant, so neither test can silently miss a newly added Emergency.
fn all_kinds() -> Vec<Emergency> {
    vec![
        Emergency::HungEngineThread(HungEngineThreadEmergency {
            nonce: "n".into(),
            lane: "stateful".into(),
            tool: "execute".into(),
            started_at: 1,
            cancelled_at: 2,
            hung_ms: 3,
            pool_held: 1,
            pool_cap: 2,
        }),
        Emergency::CpuWarning(CpuWarningEmergency {
            cpu_pct: 175.5,
            sample_ms: 2000,
        }),
        Emergency::RamWarning(RamWarningEmergency { rss_kb: 123456 }),
        Emergency::VramWarning(VramWarningEmergency {
            used_mib: 20000,
            total_mib: 24000,
        }),
        Emergency::BackgroundJobsWarning(BackgroundJobsWarningEmergency { job_count: 40 }),
        Emergency::DiskWarning(DiskWarningEmergency {
            mount: "/var".into(),
            used_pct: 85,
            threshold_pct: 80,
        }),
        Emergency::ChannelSpamWarning(ChannelSpamWarningEmergency {
            origin: "thread/abc".into(),
            hits: 10,
            window_secs: 10,
            rate: 10,
        }),
        Emergency::ChannelSpamError(ChannelSpamErrorEmergency {
            origin: "thread/abc".into(),
            hits: 15,
            window_secs: 10,
            rate: 15,
            action: "signals triggered".into(),
        }),
        Emergency::Critical(CriticalEmergency {
            reason: "all stateless engine-thread permits held by hung threads".into(),
            hung: 8,
            cap: 8,
        }),
    ]
}

#[test]
fn every_kind_serializes_single_line() {
    for em in all_kinds() {
        let want = em.kind().name().to_string();
        let line = emergency_line(&em, 1, "nom", None).unwrap();
        assert!(!line.contains('\n'), "line spans multiple lines: {line:?}");
        let v = nu::from_nuon(&line, None).expect("valid NUON record");
        let rec = v.as_record().expect("record");
        assert_eq!(rec.get("kind").and_then(|x| x.as_str().ok()), Some(want.as_str()));
    }
}
