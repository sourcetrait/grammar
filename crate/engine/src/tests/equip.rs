use crate::*;
use crate::equip::store::{
    IndexFunction, IndexModule, RigIndex, index_from_nuon, index_source_from_nuon, index_to_nuon,
};

fn columns(fields: Vec<(&str, nu::SyntaxShape)>) -> nu::CollectionColumns<nu::SyntaxShape> {
    fields
        .into_iter()
        .map(|(name, shape)| (name.to_string(), shape))
        .collect()
}

/// A data typedef with a path, an open record, and a oneof holding `nothing`.
fn wide_data() -> NuType {
    data_typedef(&nu::SyntaxShape::Record(columns(vec![
        ("n", nu::SyntaxShape::Int),
        ("p", nu::SyntaxShape::Filepath),
        ("extra", nu::SyntaxShape::record()),
        (
            "u",
            nu::SyntaxShape::OneOf(vec![nu::SyntaxShape::Int, nu::SyntaxShape::Nothing]),
        ),
    ])))
    .expect("a data type")
}

/// A return typedef with a table, a list column, and a nested record.
fn wide_result() -> NuType {
    let row = vec![
        ("name".to_string(), nu::Type::String),
        ("tags".to_string(), nu::Type::List(Box::new(nu::Type::String))),
    ];
    let nested = vec![("a".to_string(), nu::Type::Bool)];
    result_typedef(&nu::Type::Record(
        vec![
            ("rows".to_string(), nu::Type::Table(row.into())),
            ("nested".to_string(), nu::Type::Record(nested.into())),
        ]
        .into(),
    ))
    .expect("a return type")
}

/// The index carries each call's contract, so its round-trip has to survive the
/// whole type grammar rather than the easy scalar case: `nothing` as data and as a
/// return, an OPEN record, a oneof with `nothing` among its members, a table, a
/// list, and a nested record. Each is a place where a NUON render could quietly
/// change the value's type on the way back.
#[test]
fn the_index_round_trips_through_nuon_across_the_type_grammar() {
    let index = RigIndex {
        source_path: PathBuf::from("/tmp/proj/thing"),
        functions: Vec::new(),
        modules: vec![IndexModule {
            name: "m".to_string(),
            functions: vec![
                IndexFunction {
                    name: "quiet".to_string(),
                    data_type: NuType::Nothing,
                    result_type: NuType::Nothing,
                    takes_timeout: false,
                },
                IndexFunction {
                    name: "wide".to_string(),
                    data_type: wide_data(),
                    result_type: wide_result(),
                    takes_timeout: true,
                },
            ],
            modules: vec![IndexModule {
                name: "deep".to_string(),
                functions: Vec::new(),
                modules: Vec::new(),
            }],
        }],
    };

    let nuon = index_to_nuon(&index).expect("render");
    assert!(
        !nuon.contains('\n'),
        "the render is compact, so an index is one line; got {nuon}",
    );

    let back = index_from_nuon(&nuon).expect("parse");
    assert_eq!(back.source_path, index.source_path, "the source path is the namespace's link to the authored tree");
    assert!(back.functions.is_empty(), "no root functions, and an empty list must stay empty");

    let m = &back.modules[0];
    assert_eq!(m.name, "m");
    assert_eq!(m.modules[0].name, "deep", "module nesting survives");
    assert_eq!(m.functions[0].data_type, NuType::Nothing, "`nothing` survives as data");
    assert_eq!(m.functions[0].result_type, NuType::Nothing, "and as a return");
    assert!(!m.functions[0].takes_timeout);
    assert_eq!(
        m.functions[1].data_type,
        wide_data(),
        "the path, the open record, and the oneof's `nothing` must survive",
    );
    assert_eq!(
        m.functions[1].result_type,
        wide_result(),
        "the table, its list column and the nested record must survive",
    );
    assert!(m.functions[1].takes_timeout);
}

/// A malformed index is a decode ERROR, never a silently empty rig - detection
/// keys on this file, so "parsed to nothing" and "not a rig" must not look alike.
#[test]
fn a_malformed_index_fails_to_parse() {
    assert!(index_from_nuon("{not: valid").is_err(), "truncated NUON");
    assert!(index_from_nuon("[1 2 3]").is_err(), "a list is not an index record");
}

/// An index the previous build wrote, its types in the structure it
/// serialized, loads with them read as nutype's, so a rig keeps answering
/// until it is committed again.
#[test]
fn an_earlier_typed_index_still_loads() {
    let earlier = "{source_path: \"/tmp/proj/thing/rig\", functions: \
                   [[name, data_type, result_type, takes_timeout]; \
                   [log, {Record: {fields: [[name, typedef]; [refs, {List: {element: {Scalar: String}}}]]}}, \
                   Nothing, false]], modules: []}";
    let index = index_from_nuon(earlier).expect("an earlier index loads");
    assert_eq!(index.functions[0].data_type.to_inline(), "record<refs: list<string>>");
    assert_eq!(index.functions[0].result_type, NuType::Nothing);
    let again = index_from_nuon(&index_to_nuon(&index).expect("written")).expect("read back");
    assert_eq!(again.functions[0].data_type, index.functions[0].data_type, "rewritten as type text");
}

/// An index an older build wrote, its entries carrying JSON schemas, as a
/// namespace holds one until its rig is committed again. The commit reads the
/// source from it, so the source must decode though the entries do not.
#[test]
fn an_older_index_still_names_its_source() {
    let older = "{source_path: \"/tmp/proj/thing/rig\", functions: [], modules: \
                 [[name, functions, modules]; [get, [[name, args_schema, result_schema]; \
                 [log, {refs: [string]}, {logs: [[rev, summary]; [int, string]]}]], []]]}";
    assert!(
        index_from_nuon(older).is_err(),
        "an older build's entries are not this build's",
    );
    assert_eq!(
        index_source_from_nuon(older).expect("the source decodes"),
        PathBuf::from("/tmp/proj/thing/rig"),
        "the source is the one field a commit reads",
    );
}
