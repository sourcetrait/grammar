//! Type text held to nushell's own reading of it.
use crate::*;

/// nushell's reading of type text as nuval holds it, or none where nushell
/// refuses the text or nuval the shape: the text annotates a definition's
/// parameter, read back from its signature.
fn nushell_read(text: &str) -> Option<NuType> {
    let engine = nu_cmd_lang::create_default_context();
    let mut working_set = nu_protocol::engine::StateWorkingSet::new(&engine);
    let source = format!("def t [x: {}] {{}}", text.trim());
    nu_parser::parse(&mut working_set, None, source.as_bytes(), false);
    if !working_set.parse_errors.is_empty() {
        return None;
    }
    let signature = working_set.get_decl(working_set.find_decl(b"t")?).signature();
    NuType::try_from(&signature.required_positional.first()?.shape).ok()
}

/// Type text at the edges of nushell's reading, each read or refused as
/// nushell does.
const WRITTEN: &[&str] = &[
    "any", "binary", "bool", "cell-path", "datetime", "directory", "duration", "float", "filesize",
    "glob", "int", "nothing", "number", "path", "range", "string", "date", "boolean", "closure",
    "block", "error", "list", "list<>", "list<int>", "list<int, string>", "list<list<string>>",
    "record", "record<>", "record<a: int>", "record<a:int,b:string>", "record<a: int b: string>",
    "record<a: int,>", "record<name>", "record<a, b: int>", "record<\"with space\": int>",
    "record<'q': int>", "record<a: record<b: list<int>>>", "record<a: int, a: string>",
    "record<true: int>", "record<\"true\": int>", "table", "table<>", "table<a: int, b: string>",
    "oneof", "oneof<>", "oneof<int, nothing>", "oneof<int string>", "oneof<record<a: int>, nothing>",
    "record<\n    a: int,\n    b: list<string>\n>", "list <int>", "int<string>", "record<a: >",
    "record<a: int", "foo", "list<foo>", "record<a-b: int, c_d: int>",
];

#[test]
fn type_text_reads_as_nushell_reads_it() {
    let mut differ = Vec::new();
    for text in WRITTEN {
        let ours = NuType::from_text(text).ok();
        let theirs = nushell_read(text);
        if ours != theirs {
            differ.push(format!("{text:?}: nuval {ours:?}, nushell {theirs:?}"));
        }
    }
    assert!(differ.is_empty(), "{} differ:\n{}", differ.len(), differ.join("\n"));
}

/// A type of every shape, nested.
fn every_shape() -> NuType {
    NuType::Record(vec![
        NuField::new("scalars", NuType::OneOf(vec![
            NuType::Any, NuType::Nothing, NuType::Bool, NuType::Int, NuType::Float, NuType::Number,
            NuType::String, NuType::Path, NuType::Directory, NuType::Glob, NuType::Binary,
            NuType::Date, NuType::Duration, NuType::Filesize, NuType::Range, NuType::CellPath,
        ])),
        NuField::new("listed", NuType::List(Box::new(NuType::Record(vec![NuField::new("x", NuType::Int)])))),
        NuField::new("open", NuType::Record(Vec::new())),
        NuField::new("rows", NuType::Table(vec![
            NuField::new("name", NuType::String),
            NuField::new("with space", NuType::OneOf(vec![NuType::Path, NuType::Nothing])),
        ])),
        NuField::new("true", NuType::Table(Vec::new())),
    ])
}

#[test]
fn a_type_reads_back_from_either_spelling_and_nushell_reads_both() {
    let ty = every_shape();
    for text in [ty.to_inline(), ty.to_block()] {
        assert_eq!(NuType::from_text(&text).expect("read back"), ty, "{text}");
        assert_eq!(nushell_read(&text).as_ref(), Some(&ty), "{text}");
    }
}

#[test]
fn a_type_lays_out_as_the_repositorys_files_do() {
    let ty = NuType::from_text("record<a: oneof<int, nothing>, b: list<string>, c: oneof<record<d: path>, nothing>, e: table<f: int>>")
        .expect("a type");
    let expected = "record<
    a: oneof<int, nothing>,
    b: list<string>,
    c: oneof<
        record<
            d: path
        >,
        nothing
    >,
    e: table<
        f: int
    >
>";
    assert_eq!(ty.to_block(), expected);
}

#[test]
fn a_type_serializes_as_its_text_and_crosses_the_wire_in_binary() {
    let ty = every_shape();
    let nuon = to_nuon(&ty, NuonStyle::Raw).expect("written");
    assert_eq!(nuon, to_nuon(&ty.to_inline(), NuonStyle::Raw).expect("its text"));
    assert_eq!(from_nuon::<NuType>(&nuon).expect("read back"), ty);
    let bytes = bitcode::encode(&ty);
    assert_eq!(bitcode::decode::<NuType>(&bytes).expect("bitcode"), ty);
    let archived = rkyv::to_bytes::<rkyv::rancor::Error>(&ty).expect("rkyv");
    assert_eq!(rkyv::from_bytes::<NuType, rkyv::rancor::Error>(&archived).expect("rkyv back"), ty);
}
