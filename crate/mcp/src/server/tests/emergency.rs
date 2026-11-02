use crate::*;
use crate::server::emergency::emergency_model;
use sourcetrait_grammar_protocol::{
    BackgroundJobsWarningEmergency, ChannelSpamErrorEmergency, ChannelSpamWarningEmergency,
    CpuWarningEmergency, CriticalEmergency, DiskWarningEmergency, HungEngineThreadEmergency,
    MCP_RESERVED_PREFIX, RamWarningEmergency, VramWarningEmergency,
};

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

/// Every kind's line reads back in nushell, no field taking the envelope's `from`.
#[test]
fn no_event_field_collides_with_the_packet_envelope() {
    for em in all_kinds() {
        let line = emergency_line(&em, 1, "nom", Some("lab"));
        assert!(!line.contains('\n'), "line spans multiple lines: {line:?}");
        let v = nu::from_nuon(&line, None).expect("valid NUON record");
        let rec = v.as_record().expect("record");
        assert_eq!(rec.get("kind").and_then(|x| x.as_str().ok()), Some(em.kind().name()));
        assert_eq!(rec.get("host").and_then(|x| x.as_str().ok()), Some("lab"));
        assert!(
            rec.get("from").is_none(),
            "{} carries a `from` event field, which collides with the envelope's own",
            em.kind().name(),
        );
        if em.kind().name().starts_with("channel_spam") {
            assert!(rec.get("origin").is_some(), "{} must name the offending origin", em.kind().name());
        }
    }
}

/// One of every variant, so the test can't silently miss a newly added Emergency.
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
            action: "signals triggered\nand more".into(),
        }),
        Emergency::Critical(CriticalEmergency {
            reason: "all stateless engine-thread permits held by hung threads".into(),
            hung: 8,
            cap: 8,
        }),
    ]
}
