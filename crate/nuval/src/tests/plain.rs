//! An `Option<NuVal>` field through serde as the value it holds.
use crate::*;

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct Row {
    kind: String,
    #[serde(default, with = "crate::plain")]
    detail: Option<NuVal>,
}

fn record(fields: &[(&str, NuVal)]) -> NuVal {
    NuVal::Record(fields.iter().map(|(k, v)| (k.to_string(), v.clone())).collect())
}

#[test]
fn a_detail_is_written_as_the_value_it_holds_and_read_back() {
    let items = NuVal::List(vec![NuVal::Bool(true), NuVal::Float(1.5), NuVal::Nothing]);
    let detail = record(&[("nom", "fixturenom1".into()), ("n", NuVal::Int(-3)), ("items", items)]);
    let row = Row { kind: "engine::restart".into(), detail: Some(detail.clone()) };
    let written = to_nuval(&row).expect("written");
    assert_eq!(written, record(&[("kind", "engine::restart".into()), ("detail", detail)]));
    assert_eq!(
        to_nuon(&row, NuonStyle::Raw).expect("its text"),
        "{kind:\"engine::restart\",detail:{nom:\"fixturenom1\",n:-3,items:[true,1.5,null]}}",
    );
    assert_eq!(from_nuval::<Row>(written).expect("read back"), row);
}

#[test]
fn no_detail_is_written_as_nothing_and_a_missing_one_reads_as_none() {
    let row = Row { kind: "lint::summary_length".into(), detail: None };
    let written = to_nuval(&row).expect("written");
    assert_eq!(written.get("detail"), Some(&NuVal::Nothing));
    assert_eq!(from_nuval::<Row>(written).expect("read back"), row);
    assert_eq!(from_nuon::<Row>("{kind: \"lint::summary_length\"}").expect("read"), row);
}

#[test]
fn a_kind_serde_has_no_form_for_is_written_as_from_nuval_hands_it_over() {
    let date = NuVal::Date(DateData { seconds: 0, seconds_fraction: 0, utc_offset_minus: 0 });
    let glob = NuVal::Glob(GlobData { glob: "*.nu".into(), no_expand: false });
    let detail = record(&[
        ("size", NuVal::Filesize(1024)),
        ("took", NuVal::Duration(5)),
        ("glob", glob),
        ("at", date.clone()),
        ("bytes", NuVal::Binary(vec![1, 2])),
    ]);
    let written = to_nuval(&Row { kind: "x".into(), detail: Some(detail) }).expect("written");
    let expected = record(&[
        ("size", NuVal::Int(1024)),
        ("took", NuVal::Int(5)),
        ("glob", "*.nu".into()),
        ("at", NuVal::String(date.to_nuon(NuonStyle::Raw))),
        ("bytes", NuVal::Binary(vec![1, 2])),
    ]);
    assert_eq!(written.get("detail"), Some(&expected));
}
