//! Values and errors on the wire: a golden bitcode vector for every variant,
//! the round trips, malformed bytes refused, and the wire's bounds.

use std::collections::BTreeMap;

use sourcetrait_grammar_protocol::*;

#[path = "support/golden.rs"]
mod golden;

/// A payload as `T` within the default decode budget, the path every frame takes.
fn decode<T: bitcode::DecodeOwned>(payload: &[u8]) -> Result<T, FrameError> {
    decode_payload(payload, DECODE_BUDGET_DEFAULT)
}

/// The golden vectors' file, every vector's bitcode bytes in hex.
const GOLDEN: &str = "tests/vectors/value.nuon";

fn int_range(start: i64, step: i64, end: Bounded<i64>) -> NuVal {
    NuVal::Range(RangeData::Int(TypedRangeData { start, step, end }))
}

fn float_range(start: f64, step: f64, end: Bounded<f64>) -> NuVal {
    NuVal::Range(RangeData::Float(TypedRangeData { start, step, end }))
}

fn record(fields: &[(&str, NuVal)]) -> NuVal {
    NuVal::Record(fields.iter().map(|(k, v)| (k.to_string(), v.clone())).collect())
}

/// Every value vector: each variant, its boundaries, and nested values.
fn values() -> Vec<(&'static str, NuVal)> {
    vec![
        ("bool_false", NuVal::Bool(false)),
        ("bool_true", NuVal::Bool(true)),
        ("int_zero", NuVal::Int(0)),
        ("int_minus_one", NuVal::Int(-1)),
        ("int_min", NuVal::Int(i64::MIN)),
        ("int_max", NuVal::Int(i64::MAX)),
        ("float_zero", NuVal::Float(0.0)),
        ("float_minus_zero", NuVal::Float(-0.0)),
        ("float_min", NuVal::Float(f64::MIN)),
        ("float_max", NuVal::Float(f64::MAX)),
        ("float_min_positive", NuVal::Float(f64::MIN_POSITIVE)),
        ("float_subnormal", NuVal::Float(f64::from_bits(1))),
        ("float_infinity", NuVal::Float(f64::INFINITY)),
        ("float_minus_infinity", NuVal::Float(f64::NEG_INFINITY)),
        ("string_empty", NuVal::String(String::new())),
        ("string_ascii", NuVal::String("grammar".into())),
        ("string_unicode", NuVal::String("héllo wörld ✓ 𝄞 日本".into())),
        ("glob_expand", NuVal::Glob(GlobData { glob: "**/*.rs".into(), no_expand: false })),
        ("glob_literal", NuVal::Glob(GlobData { glob: "a[b]*".into(), no_expand: true })),
        ("filesize_min", NuVal::Filesize(i64::MIN)),
        ("filesize_zero", NuVal::Filesize(0)),
        ("filesize_max", NuVal::Filesize(i64::MAX)),
        ("duration_min", NuVal::Duration(i64::MIN)),
        ("duration_max", NuVal::Duration(i64::MAX)),
        (
            "date_epoch",
            NuVal::Date(DateData { seconds: 0, seconds_fraction: 0, utc_offset_minus: 0 }),
        ),
        (
            "date_offset",
            NuVal::Date(DateData {
                seconds: 1_793_232_000,
                seconds_fraction: 999_999_999,
                utc_offset_minus: -25_200,
            }),
        ),
        (
            "date_boundaries",
            NuVal::Date(DateData {
                seconds: i64::MIN,
                seconds_fraction: u32::MAX,
                utc_offset_minus: i32::MAX,
            }),
        ),
        ("range_int_included", int_range(1, 2, Bounded::Included(9))),
        ("range_int_excluded", int_range(i64::MIN, 1, Bounded::Excluded(i64::MAX))),
        ("range_int_unbounded", int_range(0, -1, Bounded::Unbounded)),
        ("range_float_included", float_range(0.5, 0.25, Bounded::Included(2.0))),
        ("range_float_unbounded", float_range(-1.0, 0.1, Bounded::Unbounded)),
        ("record_empty", NuVal::Record(Vec::new())),
        (
            "record_unicode",
            record(&[("ключ", NuVal::Int(1)), ("", NuVal::Nothing), ("✓", NuVal::Bool(true))]),
        ),
        ("list_empty", NuVal::List(Vec::new())),
        (
            "list_nested",
            NuVal::List(vec![
                NuVal::List(vec![NuVal::List(Vec::new()), record(&[("a", NuVal::Int(1))])]),
                NuVal::Nothing,
            ]),
        ),
        ("binary_empty", NuVal::Binary(Vec::new())),
        ("binary_every_byte", NuVal::Binary((0..=255).collect())),
        ("cell_path_empty", NuVal::CellPath(Vec::new())),
        (
            "cell_path_members",
            NuVal::CellPath(vec![
                CellPathMemberData::String {
                    value: "name".into(),
                    optional: false,
                    case_sensitive: true,
                },
                CellPathMemberData::String {
                    value: "ä".into(),
                    optional: true,
                    case_sensitive: false,
                },
                CellPathMemberData::Int { value: 0, optional: false },
                CellPathMemberData::Int { value: u64::MAX, optional: true },
            ]),
        ),
        ("nothing", NuVal::Nothing),
    ]
}

fn label(text: Option<&str>, start: u64, end: u64, primary: bool) -> NuValErrorLabel {
    NuValErrorLabel { text: text.map(String::from), span: SpanData { start, end }, primary }
}

fn error(kind: NuValErrorKind, severity: NuValSeverity) -> NuValError {
    NuValError {
        kind,
        severity,
        msg: kind.message().to_string(),
        code: kind.code().map(String::from),
        labels: Vec::new(),
        help: None,
        url: None,
        cause: None,
        source: None,
        inner: Vec::new(),
    }
}

/// Every error vector: each kind family and severity, and a full error.
fn errors() -> Vec<(&'static str, NuValError)> {
    use NuValErrorKind as K;
    use NuValSeverity as S;
    let full = NuValError {
        labels: vec![label(Some("here"), 2, 8, true), label(None, u64::MAX, u64::MAX, false)],
        help: Some("the help".into()),
        url: Some("https://example.invalid/help".into()),
        cause: Some("the cause".into()),
        source: Some(NuValErrorSource { name: Some("source.nu".into()), text: "1 / 0".into() }),
        inner: vec![NuValError {
            inner: vec![error(K::Other, S::Advice)],
            ..error(K::Job(JobErrorKind::NotFound), S::Error)
        }],
        ..error(K::Shell(ShellErrorKind::DivisionByZero), S::Error)
    };
    vec![
        ("shell", error(K::Shell(ShellErrorKind::DivisionByZero), S::Error)),
        ("parse", error(K::Parse(ParseErrorKind::ExtraPositional), S::Error)),
        ("compile", error(K::Compile(CompileErrorKind::Garbage), S::Error)),
        ("config", error(K::Config(ConfigErrorKind::TypeMismatch), S::Error)),
        ("io_std", error(K::Io(IoErrorKind::Std(StdIoErrorKind::NotFound)), S::Error)),
        ("io_nushell", error(K::Io(IoErrorKind::FileNotFound), S::Error)),
        ("network", error(K::Network(NetworkErrorKind::DnsAgain), S::Error)),
        ("job", error(K::Job(JobErrorKind::RecvTimeout), S::Error)),
        ("shell_warning", error(K::ShellWarning(ShellWarningKind::Deprecated), S::Warning)),
        ("parse_warning", error(K::ParseWarning(ParseWarningKind::Deprecated), S::Warning)),
        (
            "config_warning",
            error(K::ConfigWarning(ConfigWarningKind::IncompatibleOptions), S::Warning),
        ),
        ("generic", error(K::Generic, S::Error)),
        ("labeled", error(K::Labeled, S::Error)),
        ("chained", error(K::Chained, S::Error)),
        ("other_advice", error(K::Other, S::Advice)),
        ("full", full),
    ]
}

/// Every vector's name and its encoding.
fn encoded() -> BTreeMap<String, golden::Vector> {
    let values = values().into_iter().map(|(n, v)| (format!("value/{n}"), golden::vector(&v)));
    let errors = errors().into_iter().map(|(n, e)| (format!("error/{n}"), golden::vector(&e)));
    values.chain(errors).collect()
}

#[test]
fn every_vector_encodes_to_its_golden_bytes() {
    let actual = golden::hex_table(encoded());
    let golden = golden::read(GOLDEN, golden::VECTORS_MODEL, &actual);
    assert_eq!(golden, actual, "an encoding moved; a deliberate change re-blesses {GOLDEN}");
}

#[test]
fn every_value_and_error_round_trips() {
    for (name, value) in values() {
        let back: NuVal = decode(&bitcode::encode(&value)).expect(name);
        match (&value, &back) {
            (NuVal::Float(a), NuVal::Float(b)) => assert_eq!(a.to_bits(), b.to_bits(), "{name}"),
            _ => assert_eq!(back, value, "{name}"),
        }
    }
    for (name, error) in errors() {
        let back: NuValError = decode(&bitcode::encode(&error)).expect(name);
        assert_eq!(back, error, "{name}");
    }
}

#[test]
fn a_nan_keeps_its_bits() {
    let nan = f64::from_bits(0x7ff8_0000_0000_0001);
    let back: NuVal = decode(&bitcode::encode(&NuVal::Float(nan))).expect("decoded");
    let NuVal::Float(back) = back else { panic!("a float") };
    assert_eq!(back.to_bits(), nan.to_bits());
}

#[test]
fn truncated_or_trailing_bytes_are_refused() {
    for (name, vector) in encoded() {
        let bytes = vector.bytes;
        let decode = |b: &[u8]| -> bool {
            if name.starts_with("value/") {
                decode::<NuVal>(b).is_ok()
            } else {
                decode::<NuValError>(b).is_ok()
            }
        };
        for cut in 0..bytes.len() {
            assert!(!decode(&bytes[..cut]), "{name} decoded from its first {cut} bytes");
        }
        let mut longer = bytes.clone();
        longer.push(0);
        assert!(!decode(&longer), "{name} decoded with a trailing byte");
    }
}

#[test]
fn a_one_byte_value_is_nothing_or_refused() {
    let canonical = bitcode::encode(&NuVal::Nothing);
    assert_eq!(canonical.len(), 1);
    let mut accepted = Vec::new();
    for tag in 0..=u8::MAX {
        if let Ok(value) = decode::<NuVal>(&[tag]) {
            assert_eq!(value, NuVal::Nothing, "byte {tag} decoded to a payload variant");
            accepted.push(tag);
        }
    }
    assert!(accepted.contains(&canonical[0]));
    assert!(accepted.len() < 256, "some variant index is refused");
    eprintln!("one-byte inputs decoding to Nothing: {accepted:?}");
}

/// A list nested to a depth, its innermost list empty.
fn nested(depth: u32) -> NuVal {
    (1..depth).fold(NuVal::List(Vec::new()), |inner, _| NuVal::List(vec![inner]))
}

#[test]
fn a_value_is_held_to_each_bound_and_no_further() {
    let v1 = ValueLimits::V1;
    assert_eq!(check_value(&nested(64), v1), Ok(()));
    assert_eq!(check_value(&nested(65), v1), Err(ValueLimitError::Depth { limit: 64 }));

    let at = NuVal::List(vec![NuVal::Nothing; 100_000]);
    assert_eq!(check_value(&at, v1), Ok(()));
    let past = NuVal::List(vec![NuVal::Nothing; 100_001]);
    assert_eq!(check_value(&past, v1), Err(ValueLimitError::Elements { limit: 100_000 }));

    let bound = 32 * 1024 * 1024;
    assert_eq!(check_value(&NuVal::Binary(vec![0; bound]), v1), Ok(()));
    let over = NuVal::List(vec![NuVal::Binary(vec![0; bound]), NuVal::String("x".into())]);
    assert_eq!(check_value(&over, v1), Err(ValueLimitError::Bytes { limit: 32 << 20 }));
}

#[test]
fn every_vector_fits_version_one() {
    for (name, value) in values() {
        assert_eq!(check_value(&value, ValueLimits::V1), Ok(()), "{name}");
    }
    for (name, error) in errors() {
        assert_eq!(check_error(&error, ValueLimits::V1), Ok(()), "{name}");
    }
}

#[test]
fn an_error_is_held_to_its_depth_and_its_bytes() {
    let base = error(NuValErrorKind::Other, NuValSeverity::Error);
    let chain = |depth: u32| {
        (1..depth).fold(base.clone(), |inner, _| NuValError { inner: vec![inner], ..base.clone() })
    };
    assert_eq!(check_error(&chain(64), ValueLimits::V1), Ok(()));
    assert_eq!(
        check_error(&chain(65), ValueLimits::V1),
        Err(ValueLimitError::Depth { limit: 64 })
    );
    let tight = ValueLimits { depth: 64, elements: 100_000, bytes: 4 };
    let wordy = NuValError { help: Some("five!".into()), msg: String::new(), ..base };
    assert_eq!(check_error(&wordy, tight), Err(ValueLimitError::Bytes { limit: 4 }));
}
