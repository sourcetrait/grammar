//! NUON text held to nushell's own: nuval writes what `to nuon` writes and
//! reads what `from nuon` reads, and serde types travel through it.
use crate::*;

/// A small deterministic generator, so a case reruns from its seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }
}

/// The seeds the generated cases run from.
const SEEDS: [u64; 6] = [1, 7, 0x9e37_79b9_7f4a_7c15, 0xd1b5_4a32_d192_ed03, 0x2545_f491_4f6c_dd1d, 42];

/// Strings at the edges of nushell's quoting rule.
const STRINGS: [&str; 34] = [
    "", "a", "abc", "a-b", "a_b", "Self", "true", "false", "null", "&&", "inf", "-Infinity", "NaN",
    "+nan", "x y", "tab\there", "line\nbreak", "quote\"in", "back\\slash", "it's", "a,b", "a.b",
    "a:b", "#hash", "$var", "{x}", "[u8; 16]", "Vec<u8>", "u64", "\u{661}", "unicodé", "é\u{301}",
    "(x)", "-",
];

/// Record keys at the same edges.
const KEYS: [&str; 10] = ["a", "b", "name", "0", "with space", "type", "true", "kind-of", "é", "x.y"];

fn string(rng: &mut Rng) -> String {
    rng.pick(&STRINGS).to_string()
}

fn int(rng: &mut Rng) -> i64 {
    match rng.below(6) {
        0 => 0,
        1 => i64::MAX,
        2 => i64::MIN,
        3 => rng.next() as i64,
        _ => rng.below(2000) as i64 - 1000,
    }
}

fn float(rng: &mut Rng) -> f64 {
    match rng.below(8) {
        0 => 0.0,
        1 => -0.0,
        2 => 1e20,
        3 => 1.5e-7,
        4 => f64::INFINITY,
        5 => (rng.below(2000) as f64 - 1000.0) / 8.0,
        6 => (rng.below(100) as f64) * 3.0,
        _ => f64::from_bits(rng.next() >> 2),
    }
}

fn date(rng: &mut Rng) -> DateData {
    let fraction = match rng.below(4) {
        0 => 0,
        1 => rng.below(1000) as u32 * 1_000_000,
        2 => rng.below(1_000_000) as u32 * 1_000,
        _ => rng.below(1_000_000_000) as u32,
    };
    DateData {
        seconds: rng.below(8_000_000_000) as i64 - 2_000_000_000,
        seconds_fraction: fraction,
        utc_offset_minus: (rng.below(105) as i32 - 48) * 900,
    }
}

fn range(rng: &mut Rng) -> RangeData {
    let end = |rng: &mut Rng, end: i64| match rng.below(3) {
        0 => Bounded::Included(end),
        1 => Bounded::Excluded(end),
        _ => Bounded::Unbounded,
    };
    if rng.below(2) == 0 {
        let start = rng.below(200) as i64 - 100;
        let step: i64 = *rng.pick(&[1, -1, 2, -3, 5]);
        let span = rng.below(50) as i64;
        let end = end(rng, start + step.signum() * span);
        RangeData::Int(TypedRangeData { start, step, end })
    } else {
        let start = (rng.below(200) as f64 - 100.0) / 4.0;
        let step = *rng.pick(&[1.0, -1.0, 0.5, 2.25]);
        let end = match end(rng, 0) {
            Bounded::Included(_) => Bounded::Included(start + step * 8.0),
            Bounded::Excluded(_) => Bounded::Excluded(start + step * 8.0),
            Bounded::Unbounded => Bounded::Unbounded,
        };
        RangeData::Float(TypedRangeData { start, step, end })
    }
}

fn cell_path(rng: &mut Rng) -> Vec<CellPathMemberData> {
    (0..rng.below(4))
        .map(|_| match rng.below(2) {
            0 => CellPathMemberData::Int { value: rng.below(20), optional: rng.below(2) == 0 },
            _ => CellPathMemberData::String {
                value: rng.pick(&KEYS).to_string(),
                optional: rng.below(2) == 0,
                case_sensitive: rng.below(3) != 0,
            },
        })
        .collect()
}

fn record(rng: &mut Rng, depth: usize, keys: &[&str]) -> NuVal {
    NuVal::Record(keys.iter().map(|k| (k.to_string(), value(rng, depth + 1))).collect())
}

fn keys(rng: &mut Rng) -> Vec<&'static str> {
    let mut keys: Vec<&str> = Vec::new();
    for _ in 0..rng.below(4) {
        let key = *rng.pick(&KEYS);
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys
}

fn value(rng: &mut Rng, depth: usize) -> NuVal {
    let kinds = if depth > 2 { 11 } else { 14 };
    match rng.below(kinds) {
        0 => NuVal::Bool(rng.below(2) == 0),
        1 => NuVal::Int(int(rng)),
        2 => NuVal::Float(float(rng)),
        3 => NuVal::String(string(rng)),
        4 => NuVal::Filesize(int(rng)),
        5 => NuVal::Duration(int(rng)),
        6 => NuVal::Date(date(rng)),
        7 => NuVal::Range(range(rng)),
        8 => NuVal::CellPath(cell_path(rng)),
        9 => NuVal::Binary((0..rng.below(6)).map(|_| rng.next() as u8).collect()),
        10 => NuVal::Nothing,
        11 => {
            let keys = keys(rng);
            record(rng, depth, &keys)
        }
        12 => NuVal::List((0..rng.below(4)).map(|_| value(rng, depth + 1)).collect()),
        _ => {
            let keys = keys(rng);
            NuVal::List((0..1 + rng.below(3)).map(|_| record(rng, depth, &keys)).collect())
        }
    }
}

/// The styles nushell writes, nuval's and nuon's side by side.
fn styles() -> [(NuonStyle, ::nuon::ToStyle); 4] {
    [
        (NuonStyle::Default, ::nuon::ToStyle::Default),
        (NuonStyle::Raw, ::nuon::ToStyle::Raw),
        (NuonStyle::Spaces(2), ::nuon::ToStyle::Spaces(2)),
        (NuonStyle::Tabs(1), ::nuon::ToStyle::Tabs(1)),
    ]
}

/// nushell's own text for a value, or none where nushell can't hold it.
fn nushell_text(value: &NuVal, style: ::nuon::ToStyle) -> Option<String> {
    let value = value.clone().try_into_nu(nu::Span::unknown()).ok()?;
    let engine = nu_protocol::engine::EngineState::new();
    ::nuon::to_nuon(&engine, &value, ::nuon::ToNuonConfig::default().style(style)).ok()
}

/// nushell's own reading of text, or none where it refuses.
fn nushell_read(text: &str) -> Option<NuVal> {
    NuVal::try_from_nu(::nuon::from_nuon(text, None).ok()?).ok()
}

#[test]
fn the_writer_writes_what_nushell_writes_in_every_style() {
    let mut cases = 0;
    for seed in SEEDS {
        let mut rng = Rng(seed);
        for _ in 0..300 {
            let value = value(&mut rng, 0);
            for (ours, theirs) in styles() {
                let Some(expected) = nushell_text(&value, theirs.clone()) else {
                    continue;
                };
                assert_eq!(value.to_nuon(ours), expected, "seed {seed:#x}, {ours:?}: {value:?}");
                cases += 1;
            }
        }
    }
    assert!(cases > 4000, "only {cases} cases reached nushell");
}

#[test]
fn the_reader_reads_nushells_text_as_nushell_does() {
    let mut cases = 0;
    for seed in SEEDS {
        let mut rng = Rng(seed);
        for _ in 0..300 {
            let value = value(&mut rng, 0);
            for (_, theirs) in styles() {
                let Some(text) = nushell_text(&value, theirs) else {
                    continue;
                };
                let expected = nushell_read(&text).expect("nushell reads its own text");
                let read = NuVal::from_nuon(&text).unwrap_or_else(|e| panic!("seed {seed:#x}: {e}\n{text}"));
                assert_eq!(read, expected, "seed {seed:#x}: {text}");
                cases += 1;
            }
        }
    }
    assert!(cases > 4000, "only {cases} cases reached nushell");
}

/// Hand-written NUON at the edges of nushell's reading, each read or refused
/// as nushell does.
const WRITTEN: &[&str] = &[
    "", "  # only a comment\n", "1", "-1", "+5", "1_000", "0x1f", "0o17", "0b101", "0b", "0x", "0o",
    "0xZZ", "1.5", ".5", "5.", "1e3", "1E-3", "inf", "-inf", "infinity", "1_0.5", "10b", "0b10b",
    "1kb", "1KB", "1.5kb", "1.5b", "1.5MiB", "2gib", "1x2kb", "3rdb", "10ns", "1.5sec", "2min",
    "1hr", "1day", "1wk", "1\u{b5}s", "5ms", "1SEC", "1.2.3", "2024-03-05",
    "2024-03-05T06:07:08", "2024-03-05T06:07:08.5", "2024-03-05T06:07:08Z",
    "2024-03-05t06:07:08z", "2024-03-05 06:07:08+02:30", "2024-03-05T06:07:08.123456789-07:00",
    "2024-02-30", "2024-03-05T24:00:00Z", "2016-12-31T23:59:60Z", "1..5", "1..<5", "1..=5",
    "5..1", "1..3..9", "..5", "1..", "1.5..4.5", "0.1..0.3", "1..1..5", "a..b", "1...5", "true",
    "false", "null", "foo", "`foo`", "\"foo\"", "'foo'", "-foo", "not", "\"a\\nb\\t\\u{1F600}\"",
    "\"\\xc3\\xa9\"", "\"\\q\"", "\"a\"b", "r#'a\"b'#", "r##'x'#y'##", "[]", "[1 2 3]",
    "[1,2,3]", "[a, \"b c\", 'd', `e f`]", "[1\n2\n# c\n3]", "[x (y)]", "[a b", "[1; 2]",
    "[[a b]; [1 2] [3 4]]", "[[a,b];[1,2],[3,4]]", "[[a b]; [1]]", "[[a b];]",
    "[[\"0\" b]; [1 2]]", "[[0 b]; [1 2]]", "[[a a]; [1 2]]", "{}", "{a: 1}", "{a:1,b:2}",
    "{ a: 1\n  b: [x y] }", "{\"with space\": 1, 'q': 2}", "{a: 1, a: 2}", "{true: 1}",
    "{a 1}", "{a: }", "{a: datum::Nonce}", "{a: \"b:c\"}", "{a: r#'b:c'#}", "{a: 2024-01-01T06:07:08}",
    "[a:b]", "{k: 1kb, d: 2sec, t: 2024-01-01}", "$.a.b",
    "$.a?.0.\"b c\"!", "$.", "$foo", "0x[0a ff]", "0x[0aff, 01]", "0b[101]", "0o[17]",
    "0x[zz]", "1 2", "1\n2", "1; 2", "(1)", "{a: 1} # trailing", "[...[1]]", "\"unclosed",
    "[\n  1,\n  2\n]", "{\n\ta: 1\n}", "[\u{661}]",
];

#[test]
fn the_reader_reads_written_nuon_as_nushell_does() {
    let mut differ = Vec::new();
    for text in WRITTEN {
        let ours = NuVal::from_nuon(text);
        let theirs = nushell_read(text);
        match (&ours, &theirs) {
            (Ok(ours), Some(theirs)) if ours == theirs => {}
            (Err(_), None) => {}
            _ => differ.push(format!("{text:?}: nuval {ours:?}, nushell {theirs:?}")),
        }
    }
    assert!(differ.is_empty(), "{} differ:\n{}", differ.len(), differ.join("\n"));
}

/// A serde type of every shape serde has.
#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct Shapes {
    flag: bool,
    small: u8,
    big: i64,
    ratio: f64,
    name: String,
    letter: char,
    missing: Option<u32>,
    present: Option<String>,
    items: Vec<u16>,
    pair: (i32, String),
    map: std::collections::BTreeMap<String, i64>,
    numbered: std::collections::BTreeMap<u32, bool>,
    unit: (),
    unit_variant: Kind,
    newtype_variant: Kind,
    tuple_variant: Kind,
    struct_variant: Kind,
    wrapped: Wrapper,
    nested: Vec<Inner>,
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
enum Kind {
    Plain,
    Holds(String),
    Pair(u8, u8),
    Fields { a: i32, b: Vec<String> },
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct Wrapper(u64);

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct Inner {
    key: String,
    value: Option<Box<Inner>>,
}

fn shapes() -> Shapes {
    Shapes {
        flag: true,
        small: 200,
        big: -42,
        ratio: 2.5,
        name: "two words".into(),
        letter: 'q',
        missing: None,
        present: Some("here".into()),
        items: vec![1, 2, 3],
        pair: (-1, "one".into()),
        map: [("a".to_string(), 1), ("b c".to_string(), 2)].into(),
        numbered: [(1, true), (20, false)].into(),
        unit: (),
        unit_variant: Kind::Plain,
        newtype_variant: Kind::Holds("held".into()),
        tuple_variant: Kind::Pair(1, 2),
        struct_variant: Kind::Fields { a: 7, b: vec!["x".into()] },
        wrapped: Wrapper(9),
        nested: vec![Inner { key: "k".into(), value: Some(Box::new(Inner { key: "j".into(), value: None })) }],
    }
}

#[test]
fn a_serde_type_travels_through_nuon_in_every_style() {
    let value = shapes();
    for (style, _) in styles() {
        let text = to_nuon(&value, style).expect("written");
        assert_eq!(from_nuon::<Shapes>(&text).expect("read back"), value, "{style:?}\n{text}");
        // nushell reads the same text to the same value.
        assert_eq!(nushell_read(&text).as_ref(), NuVal::from_nuon(&text).ok().as_ref(), "{text}");
    }
}

#[test]
fn serde_shapes_take_nuons_natural_forms() {
    let value = to_nuval(&shapes()).expect("a value");
    let NuVal::Record(entries) = value else {
        panic!("a struct is a record");
    };
    let field = |name: &str| entries.iter().find(|(k, _)| k == name).map(|(_, v)| v.to_nuon(NuonStyle::Raw));
    assert_eq!(field("unit_variant").as_deref(), Some("\"Plain\""));
    assert_eq!(field("newtype_variant").as_deref(), Some("{Holds:held}"));
    assert_eq!(field("tuple_variant").as_deref(), Some("{Pair:[1,2]}"));
    assert_eq!(field("struct_variant").as_deref(), Some("{Fields:{a:7,b:[x]}}"));
    assert_eq!(field("missing").as_deref(), Some("null"));
    assert_eq!(field("wrapped").as_deref(), Some("9"));
    assert_eq!(field("numbered").as_deref(), Some("{\"1\":true,\"20\":false}"));
}

#[test]
fn a_refused_conversion_names_its_path() {
    let err = from_nuon::<Shapes>("{flag: true}").expect_err("fields missing");
    assert!(err.to_string().contains("small"), "{err}");
    let mut text = to_nuon(&shapes(), NuonStyle::Raw).expect("written");
    text = text.replace("{key:j,value:null}", "{key:j,value:{key:5,value:null}}");
    let err = from_nuon::<Shapes>(&text).expect_err("a number where a string goes");
    assert!(err.to_string().starts_with("at nested.0.value.value.key:"), "{err}");
    let err = to_nuval(&u64::MAX).expect_err("past an int");
    assert!(err.to_string().contains("past a 64-bit int"), "{err}");
}
