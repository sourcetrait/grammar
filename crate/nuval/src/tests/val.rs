use crate::*;

const SPAN: nu::Span = nu::Span::unknown();

fn int_range(start: i64, step: i64, end: Bounded<i64>) -> NuVal {
    NuVal::Range(RangeData::Int(TypedRangeData { start, step, end }))
}

fn float_range(start: f64, step: f64, end: Bounded<f64>) -> NuVal {
    NuVal::Range(RangeData::Float(TypedRangeData { start, step, end }))
}

/// Every kind nuval carries, nested, as nushell builds it.
fn every_kind() -> nu::Value {
    let date = chrono::DateTime::parse_from_rfc3339("2026-10-27T14:30:05.123456789+02:00")
        .expect("a date");
    let mut inner = nu::Record::new();
    inner.push("deep", nu::Value::list(vec![nu::Value::int(-7, SPAN)], SPAN));
    let mut record = nu::Record::new();
    record.push("bool", nu::Value::bool(true, SPAN));
    record.push("int", nu::Value::int(i64::MIN, SPAN));
    record.push("float", nu::Value::float(-0.25, SPAN));
    record.push("string", nu::Value::string("héllo", SPAN));
    record.push("glob", nu::Value::glob("*.rs", true, SPAN));
    record.push("filesize", nu::Value::filesize(nu::Filesize::new(4096), SPAN));
    record.push("duration", nu::Value::duration(-1_500_000_000, SPAN));
    record.push("date", nu::Value::date(date, SPAN));
    record.push(
        "int_range",
        nu::Value::range(nu::Range::new_int(1, 3, Bound::Excluded(10)), SPAN),
    );
    record.push(
        "float_range",
        nu::Value::range(nu::Range::new_float(0.5, None, Bound::Unbounded), SPAN),
    );
    record.push("record", nu::Value::record(inner, SPAN));
    record.push("binary", nu::Value::binary(vec![0u8, 255, 7], SPAN));
    record.push(
        "cell_path",
        nu::Value::cell_path(
            nu::CellPath {
                members: vec![
                    nu::PathMember::String {
                        val: "name".into(),
                        span: SPAN,
                        optional: true,
                        casing: nu::Casing::Insensitive,
                    },
                    nu::PathMember::Int {
                        val: 3,
                        span: SPAN,
                        optional: false,
                    },
                ],
            },
            SPAN,
        ),
    );
    record.push("nothing", nu::Value::nothing(SPAN));
    nu::Value::list(vec![nu::Value::record(record, SPAN)], SPAN)
}

fn every_kind_nuval() -> NuVal {
    NuVal::try_from_nu(every_kind()).expect("every kind converts")
}

#[test]
fn every_kind_round_trips_through_nushell() {
    let val = every_kind_nuval();
    let back = val.clone().try_into_nu(SPAN).expect("back to nushell");
    assert_eq!(NuVal::try_from_nu(back).expect("again"), val);
}

#[test]
fn every_kind_round_trips_through_serde() {
    let val = every_kind_nuval();
    let text = to_nuon(&val, NuonStyle::Raw).expect("serialize");
    assert_eq!(from_nuon::<NuVal>(&text).expect("deserialize"), val);
}

#[test]
fn every_kind_round_trips_through_bitcode() {
    let val = every_kind_nuval();
    let bytes = bitcode::encode(&val);
    assert_eq!(bitcode::decode::<NuVal>(&bytes).expect("decode"), val);
}

#[test]
fn every_kind_round_trips_through_rkyv() {
    let val = every_kind_nuval();
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&val).expect("archive");
    assert_eq!(
        rkyv::from_bytes::<NuVal, rkyv::rancor::Error>(&bytes).expect("unarchive"),
        val
    );
}

#[test]
fn a_float_range_comes_back_with_its_exact_step() {
    let val = float_range(0.1, 0.2, Bounded::Included(0.9));
    let back = val.clone().try_into_nu(SPAN).expect("back to nushell");
    assert_eq!(NuVal::try_from_nu(back).expect("again"), val);
}

#[test]
fn a_date_keeps_its_offset_and_a_leap_second() {
    let val = NuVal::Date(DateData {
        seconds: 1_483_228_799,
        seconds_fraction: 1_500_000_000,
        utc_offset_minus: -5 * 3600,
    });
    let back = val.clone().try_into_nu(SPAN).expect("back to nushell");
    assert_eq!(NuVal::try_from_nu(back).expect("again"), val);
}

#[test]
fn an_offset_beyond_a_day_is_refused() {
    let val = NuVal::Date(DateData {
        seconds: 0,
        seconds_fraction: 0,
        utc_offset_minus: 86_400,
    });
    assert!(matches!(val.try_into_nu(SPAN), Err(ConvertError::Date { .. })));
}

#[test]
fn ranges_nushell_refuses_are_refused() {
    let refused = [
        int_range(1, 0, Bounded::Included(5)),
        int_range(1, 1, Bounded::Included(-5)),
        int_range(i64::MAX, 1, Bounded::Unbounded),
        float_range(1.0, 0.0, Bounded::Unbounded),
        float_range(f64::INFINITY, 1.0, Bounded::Unbounded),
        float_range(1.0, 1.0, Bounded::Excluded(f64::NAN)),
        float_range(1.0, -1.0, Bounded::Included(4.0)),
    ];
    for val in refused {
        assert!(
            matches!(val.clone().try_into_nu(SPAN), Err(ConvertError::Range { .. })),
            "{val:?} converted"
        );
    }
}

#[test]
fn an_infinite_float_end_comes_back_unbounded() {
    let val = float_range(0.0, 0.5, Bounded::Included(f64::INFINITY));
    let back = val.try_into_nu(SPAN).expect("back to nushell");
    assert_eq!(
        NuVal::try_from_nu(back).expect("again"),
        float_range(0.0, 0.5, Bounded::Unbounded)
    );
}

#[test]
fn a_duplicate_column_is_refused() {
    let val = NuVal::Record(vec![
        ("a".into(), NuVal::Int(1)),
        ("a".into(), NuVal::Int(2)),
    ]);
    assert!(matches!(
        val.try_into_nu(SPAN),
        Err(ConvertError::DuplicateColumn { column }) if column == "a"
    ));
}

#[test]
fn a_closure_is_refused() {
    let closure = nu::Value::closure(
        nu_protocol::engine::Closure {
            block_id: nu_protocol::BlockId::new(0),
            captures: vec![],
        },
        SPAN,
    );
    assert!(matches!(
        NuVal::try_from_nu(closure),
        Err(ConvertError::Unsupported { value_type }) if value_type == "closure"
    ));
}

#[test]
fn an_error_value_is_the_results_error_however_deep() {
    let error = nu::Value::error(nu::ShellError::DivisionByZero { span: SPAN }, SPAN);
    assert!(matches!(NuValResult::try_from_nu(error.clone()), Ok(Err(_))));
    let nested = nu::Value::list(vec![nu::Value::int(1, SPAN), error], SPAN);
    assert!(matches!(NuValResult::try_from_nu(nested), Ok(Err(_))));
    assert!(matches!(
        NuValResult::try_from_nu(nu::Value::int(1, SPAN)),
        Ok(Ok(NuVal::Int(1)))
    ));
}

#[test]
fn a_row_index_past_this_platform_is_refused_and_one_inside_it_round_trips() {
    let val = NuVal::CellPath(vec![CellPathMemberData::Int {
        value: u64::MAX,
        optional: false,
    }]);
    let back = val.clone().try_into_nu(SPAN);
    if usize::BITS < u64::BITS {
        assert!(matches!(back, Err(ConvertError::Index { .. })), "{back:?}");
    } else {
        let again = NuVal::try_from_nu(back.expect("a 64-bit usize holds it")).expect("back");
        assert_eq!(again, val);
    }
}
