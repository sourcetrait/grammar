//! A NuVal read by its parts.
use crate::*;

/// A value of every part the accessors read.
fn value() -> NuVal {
    NuVal::from_nuon("{name: a, n: 3, neg: -1, f: 1.5, on: true, none: null, items: [x, 2]}").expect("NUON")
}

#[test]
fn an_index_reads_a_field_or_an_item_and_finds_nothing_past_them() {
    let v = value();
    assert_eq!(v["name"], "a");
    assert_eq!(v["items"][0], "x");
    assert_eq!(v["items"][1], 2);
    assert!(v["missing"].is_nothing());
    assert!(v["items"][2].is_nothing());
    assert!(v["name"]["deeper"].is_nothing(), "a field of a string is nothing");
    assert!(v[0].is_nothing(), "an item of a record is nothing");
    assert_eq!(v.get("n"), Some(&NuVal::Int(3)));
    assert_eq!(v.get("missing"), None);
}

#[test]
fn each_accessor_reads_its_own_kind_alone() {
    let v = value();
    assert_eq!(v["name"].as_str(), Some("a"));
    assert_eq!(v["n"].as_str(), None);
    assert_eq!(v["n"].as_i64(), Some(3));
    assert_eq!(v["n"].as_u64(), Some(3));
    assert_eq!(v["neg"].as_u64(), None);
    assert_eq!(v["f"].as_f64(), Some(1.5));
    assert_eq!(v["n"].as_f64(), Some(3.0));
    assert_eq!(v["f"].as_i64(), None);
    assert_eq!(v["on"].as_bool(), Some(true));
    assert_eq!(v["items"].as_list().map(<[NuVal]>::len), Some(2));
    let fields: Vec<&str> = v.as_record().expect("a record").iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(fields, ["name", "n", "neg", "f", "on", "none", "items"]);
    assert!(v["none"].is_nothing());
}

#[test]
fn a_value_equals_a_primitive_of_its_own_kind_alone() {
    let v = value();
    assert_eq!(v["name"], "a".to_string());
    assert_ne!(v["name"], "b");
    assert_ne!(v["n"], "3", "an int is never its text");
    assert_ne!(NuVal::Float(3.0), 3, "a float is never an int");
    assert_eq!(v["on"], true);
    assert_eq!(NuVal::from("t"), NuVal::String("t".to_string()));
    assert_eq!(NuVal::from(7), NuVal::Int(7));
    assert_eq!(NuVal::from(false), NuVal::Bool(false));
}

#[test]
fn a_value_displays_as_its_raw_nuon() {
    assert_eq!(value()["items"].to_string(), "[x,2]");
    assert_eq!(
        NuVal::from_nuon("{a: 1, b: [x]}").expect("NUON").to_string(),
        NuVal::from_nuon("{a: 1, b: [x]}").expect("NUON").to_nuon(NuonStyle::Raw),
    );
}
