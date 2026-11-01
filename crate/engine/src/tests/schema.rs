use crate::*;

fn columns(fields: Vec<(&str, nu::SyntaxShape)>) -> nu::CollectionColumns<nu::SyntaxShape> {
    fields
        .into_iter()
        .map(|(name, shape)| (name.to_string(), shape))
        .collect()
}

fn record(fields: Vec<(&str, nu::SyntaxShape)>) -> nu::SyntaxShape {
    nu::SyntaxShape::Record(columns(fields))
}

fn table(fields: Vec<(&str, nu::SyntaxShape)>) -> nu::SyntaxShape {
    nu::SyntaxShape::Table(columns(fields))
}

fn data(shape: nu::SyntaxShape) -> NuType {
    data_typedef(&shape).expect("a data type")
}

fn checked(shape: nu::SyntaxShape, nuon: &str) -> Result<(), String> {
    let value = nuon::from_nuon(nuon, None).expect("NUON");
    crate::schema::check_value(&value, &data(shape), "data")
}

/// A `main` taking `data`, its return `output`.
fn signature(
    data: nu::SyntaxShape,
    output: nu::Type,
) -> nu::Signature {
    nu::Signature::build("main")
        .required("data", data, "the data")
        .input_output_types(vec![(nu::Type::Nothing, output)])
}

fn out_record() -> nu::Type {
    nu::Type::Record(vec![("out".to_string(), nu::Type::Int)].into())
}

#[test]
fn check_holds_a_record_to_its_fields() {
    let shape = || record(vec![("a", nu::SyntaxShape::Int)]);
    assert_eq!(checked(shape(), "{a: 1}"), Ok(()));
    assert_eq!(
        checked(shape(), "{a: 1, b: 2}"),
        Err("data.b: an undeclared field".to_string()),
    );
    assert_eq!(
        checked(shape(), "{}"),
        Err("data.a: a declared field is missing".to_string()),
    );
    assert_eq!(
        checked(shape(), "{a: x}"),
        Err("data.a: expected int, got string".to_string()),
    );
}

#[test]
fn check_takes_anything_inside_an_open_record() {
    let shape = record(vec![("fill", nu::SyntaxShape::record())]);
    assert_eq!(checked(shape, "{fill: {x: 1, y: [2, 3], z: {w: null}}}"), Ok(()));
}

#[test]
fn check_holds_each_table_row_to_its_columns() {
    let shape = || table(vec![("n", nu::SyntaxShape::Int)]);
    assert_eq!(checked(shape(), "[[n]; [1] [2]]"), Ok(()));
    assert_eq!(checked(shape(), "[]"), Ok(()));
    assert_eq!(
        checked(shape(), "[[n, m]; [1, 2]]"),
        Err("data[0].m: an undeclared field".to_string()),
    );
    assert_eq!(
        checked(shape(), "[{n: 1}, {n: 2.5}]"),
        Err("data[1].n: expected int, got float".to_string()),
    );
}

#[test]
fn check_walks_list_items_and_oneof_members() {
    let shape = || {
        record(vec![
            ("xs", nu::SyntaxShape::List(Box::new(nu::SyntaxShape::Int))),
            (
                "u",
                nu::SyntaxShape::OneOf(vec![nu::SyntaxShape::Int, nu::SyntaxShape::Nothing]),
            ),
        ])
    };
    assert_eq!(checked(shape(), "{xs: [1, 2], u: null}"), Ok(()));
    assert_eq!(
        checked(shape(), "{xs: [1, a], u: 1}"),
        Err("data.xs[1]: expected int, got string".to_string()),
    );
    let refused = checked(shape(), "{xs: [], u: s}").expect_err("a string is neither member");
    assert!(refused.starts_with("data.u: a value typed string matches none of"), "{refused}");
}

#[test]
fn check_reads_scalars_by_their_variant() {
    let shape = || {
        record(vec![
            ("p", nu::SyntaxShape::Filepath),
            ("n", nu::SyntaxShape::Number),
            ("d", nu::SyntaxShape::Duration),
        ])
    };
    assert_eq!(checked(shape(), "{p: here, n: 1, d: 2sec}"), Ok(()));
    assert_eq!(checked(shape(), "{p: here, n: 1.5, d: 2sec}"), Ok(()));
    assert_eq!(
        checked(shape(), "{p: here, n: 1, d: 2}"),
        Err("data.d: expected duration, got int".to_string()),
    );
}

#[test]
fn check_holds_nothing_to_null() {
    assert_eq!(checked(nu::SyntaxShape::Nothing, "null"), Ok(()));
    assert_eq!(
        checked(nu::SyntaxShape::Nothing, "{}"),
        Err("data: expected nothing, got record".to_string()),
    );
}

#[test]
fn a_signature_group_spells_its_typedef_compactly() {
    let refs = data(record(vec![(
        "refs",
        nu::SyntaxShape::List(Box::new(nu::SyntaxShape::String)),
    )]));
    assert_eq!(signature_group(&refs), "<refs:list<string>>");
    let rows = data(table(vec![("n", nu::SyntaxShape::Int), ("p", nu::SyntaxShape::Filepath)]));
    assert_eq!(signature_group(&rows), "<table<n:int,p:path>>");
    assert_eq!(signature_group(&NuType::Nothing), "<nothing>");
    let nested = data(record(vec![
        (
            "u",
            nu::SyntaxShape::OneOf(vec![nu::SyntaxShape::Int, nu::SyntaxShape::Nothing]),
        ),
        ("fill", nu::SyntaxShape::record()),
    ]));
    assert_eq!(signature_group(&nested), "<u:oneof<int,nothing>,fill:record<>>");
}

#[test]
fn the_contract_refuses_an_open_top_any_and_a_list_of_tables() {
    assert!(data_typedef(&nu::SyntaxShape::record()).is_err(), "an open record at the top");
    assert!(data_typedef(&nu::SyntaxShape::Int).is_err(), "a scalar at the top");
    assert!(
        data_typedef(&record(vec![("x", nu::SyntaxShape::Any)])).is_err(),
        "`any` anywhere",
    );
    let tables = nu::SyntaxShape::List(Box::new(table(vec![("a", nu::SyntaxShape::Int)])));
    assert!(data_typedef(&record(vec![("t", tables)])).is_err(), "a list of tables");
    assert!(result_typedef(&nu::Type::record()).is_err(), "an open record returned");
    assert!(result_typedef(&nu::Type::String).is_err(), "a scalar returned");
}

/// `execute.md` types a return as `nothing`, a record, or a table, as it types
/// data: `nothing` stands as the return itself and nested in one.
#[test]
fn a_return_is_nothing_a_record_or_a_table() {
    assert_eq!(result_typedef(&nu::Type::Nothing), Ok(NuType::Nothing));
    let nullable = nu::Type::Record(
        vec![("u".to_string(), nu::Type::one_of([nu::Type::Int, nu::Type::Nothing]))].into(),
    );
    assert_eq!(
        typedef_text(&result_typedef(&nullable).expect("a nested nothing stands")),
        "record<u: oneof<int, nothing>>",
    );
    let quiet = signature(nu::SyntaxShape::Nothing, nu::Type::Nothing);
    let held = contract_signature(&quiet, "main").expect("a nothing return holds");
    assert_eq!(held.result_type, NuType::Nothing);
    assert_eq!(
        result_typedef(&nu::Type::List(Box::new(nu::Type::Int))),
        Err("typed `nothing`, `record<...>`, or `table<...>`, not `list<int>`".to_string()),
    );
}

#[test]
fn a_main_signature_holds_to_the_contract() {
    let held = contract_signature(
        &signature(record(vec![("x", nu::SyntaxShape::Int)]), out_record()),
        "main",
    )
    .expect("data and a record return hold");
    assert_eq!(typedef_text(&held.data_type), "record<x: int>");
    assert_eq!(typedef_text(&held.result_type), "record<out: int>");
    assert!(!held.takes_timeout);

    let timed = signature(nu::SyntaxShape::Nothing, out_record()).optional(
        "timeout",
        nu::SyntaxShape::Duration,
        "the timeout",
    );
    assert!(contract_signature(&timed, "main").expect("holds").takes_timeout);

    let args = nu::Signature::build("main")
        .required("args", record(vec![("x", nu::SyntaxShape::Int)]), "the args")
        .input_output_types(vec![(nu::Type::Nothing, out_record())]);
    assert_eq!(
        contract_signature(&args, "main").expect_err("`args` is no contract name"),
        vec!["`main` takes one required parameter, `data`".to_string()],
    );

    let piped = nu::Signature::build("main")
        .required("data", nu::SyntaxShape::Nothing, "the data")
        .input_output_types(vec![(nu::Type::String, out_record())]);
    assert_eq!(
        contract_signature(&piped, "main").expect_err("input is nothing"),
        vec![
            "`main` declares one infix signature, its input `nothing`: `: nothing -> record<...>`"
                .to_string()
        ],
    );
}
