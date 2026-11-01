//! Values held to their types.
use crate::*;

fn ty(text: &str) -> NuType {
    NuType::from_text(text).expect("a type")
}

fn value(text: &str) -> NuVal {
    NuVal::from_nuon(text).expect("a value")
}

#[test]
fn a_value_of_its_type_passes() {
    let cases = [
        ("any", "{a: 1}"),
        ("nothing", "null"),
        ("int", "5"),
        ("number", "5.5"),
        ("path", "\"/tmp\""),
        ("directory", "\"dir\""),
        ("glob", "\"*.rs\""),
        ("datetime", "2024-01-01"),
        ("filesize", "1kb"),
        ("duration", "2sec"),
        ("list<int>", "[1 2 3]"),
        ("record", "{anything: 1}"),
        ("record<a: int, b: list<string>>", "{a: 1, b: [x y]}"),
        ("table<a: int>", "[[a]; [1] [2]]"),
        ("table", "[{x: 1}]"),
        ("oneof<int, nothing>", "null"),
        ("record<a: oneof<record<b: path>, nothing>>", "{a: {b: here}}"),
    ];
    for (t, v) in cases {
        assert_eq!(ty(t).check(&value(v)), Ok(()), "{t} holds {v}");
    }
}

#[test]
fn a_mismatch_names_its_path_and_its_kinds() {
    let cases = [
        ("int", "\"5\"", "expected `int`, found string"),
        ("record<a: int>", "{a: 1, b: 2}", "at b: an undeclared field"),
        ("record<a: int, b: int>", "{a: 1}", "at b: a declared field is missing"),
        ("record<a: list<int>>", "{a: [1 x]}", "at a.1: expected `int`, found string"),
        ("table<a: int>", "[[a]; [1] [x]]", "at 1.a: expected `int`, found string"),
        ("table<a: int>", "[1]", "at 0: expected a record, found int"),
        ("oneof<int, nothing>", "\"x\"", "expected one of `oneof<int, nothing>`, found string"),
        ("path", "1", "expected `path`, found int"),
    ];
    for (t, v, message) in cases {
        let err = ty(t).check(&value(v)).expect_err("a mismatch");
        assert_eq!(err.to_string(), message, "{t} against {v}");
    }
}
