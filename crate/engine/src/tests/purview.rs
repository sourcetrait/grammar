//! The purview table's text and a rig pattern's spellings.
use crate::*;
use crate::equip::purview::{is_rig_pattern, purviews_from_nuon, purviews_to_nuon};

/// The table renders as nushell renders it and reads back whole.
#[test]
fn the_purview_table_writes_as_nushell_writes_it_and_reads_back() {
    let row = |id: &str, patterns: &[&str]| PurviewRow {
        id: id.to_string(),
        namepath_patterns: patterns.iter().map(|p| p.to_string()).collect(),
    };
    let rows = vec![
        row("default", &["@core", "acme/geo:shape:area"]),
        row("core", &["@grammar", "@gram"]),
        row("grammar", &["grammar:"]),
    ];
    let text = purviews_to_nuon(&rows).expect("the table renders");
    let value = nuval::to_nuval(&rows)
        .expect("a value")
        .try_into_nu(nu::Span::unknown())
        .expect("nushell's value");
    let nushell = nu::to_nuon(&nu::EngineState::new(), &value, nu::ToNuonConfig::default())
        .expect("nushell renders it");
    assert_eq!(text, nushell);
    assert_eq!(
        text,
        "[[id, namepath_patterns]; [default, [@core, \"acme/geo:shape:area\"]], \
         [core, [@grammar, @gram]], [grammar, [\"grammar:\"]]]"
    );
    assert_eq!(purviews_from_nuon(&text).expect("it reads back"), rows);
}

/// A builtin's whole-rig pattern is found under either of its spellings.
#[test]
fn a_rig_pattern_is_found_in_either_spelling() {
    assert!(is_rig_pattern("gram:", "sourcetrait/gram"));
    assert!(is_rig_pattern("sourcetrait/gram:", "sourcetrait/gram"));
    assert!(!is_rig_pattern("gram:log:", "sourcetrait/gram"));
    assert!(!is_rig_pattern("acme/gram:", "sourcetrait/gram"));
    assert!(!is_rig_pattern("@gram", "sourcetrait/gram"));
}
