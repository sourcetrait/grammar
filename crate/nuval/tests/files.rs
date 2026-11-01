//! The repository's `.nutype` files, read and written back as they stand, and
//! its NUON files, each held to its model.
use std::path::{Path, PathBuf};

use nuval::{NuTypeFile, NuVal};

/// The repository's `.nutype` files, from the workspace's root.
const FILES: [&str; 7] = [
    "include/config/Config.nutype",
    "include/scratch/Lock.nutype",
    "include/scratch/Owner.nutype",
    "crate/protocol/schema/Schema.nutype",
    "crate/protocol/tests/vectors/Budget.nutype",
    "crate/protocol/tests/vectors/Manifest.nutype",
    "crate/protocol/tests/vectors/Vectors.nutype",
];

/// The repository's NUON files, each with its model, from the workspace's root.
const MODELED: [(&str, &str); 8] = [
    ("crate/protocol/schema/v1.nuon", "crate/protocol/schema/Schema.nutype"),
    ("crate/protocol/tests/vectors/budget.nuon", "crate/protocol/tests/vectors/Budget.nutype"),
    ("crate/protocol/tests/vectors/engine.nuon", "crate/protocol/tests/vectors/Vectors.nutype"),
    ("crate/protocol/tests/vectors/hello.nuon", "crate/protocol/tests/vectors/Vectors.nutype"),
    ("crate/protocol/tests/vectors/manifest.nuon", "crate/protocol/tests/vectors/Manifest.nutype"),
    ("crate/protocol/tests/vectors/peer.nuon", "crate/protocol/tests/vectors/Vectors.nutype"),
    ("crate/protocol/tests/vectors/session.nuon", "crate/protocol/tests/vectors/Vectors.nutype"),
    ("crate/protocol/tests/vectors/value.nuon", "crate/protocol/tests/vectors/Vectors.nutype"),
];

/// The workspace's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A file's text, from the workspace's root.
fn read(file: &str) -> String {
    std::fs::read_to_string(root().join(file)).unwrap_or_else(|e| panic!("read {file}: {e}"))
}

/// A `.nutype` file's model.
fn model(file: &str) -> NuTypeFile {
    NuTypeFile::from_text(&read(file)).unwrap_or_else(|e| panic!("{file}: {e}"))
}

#[test]
fn each_repository_nutype_reads_and_writes_back_unchanged() {
    for file in FILES {
        let text = read(file);
        let model = model(file);
        assert!(!model.doc.is_empty(), "{file} keeps its documentation");
        assert_eq!(model.to_text(), text, "{file} writes back as it stands");
    }
}

#[test]
fn each_repository_nuon_holds_to_its_model() {
    for (file, model_file) in MODELED {
        let value = NuVal::from_nuon(&read(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
        model(model_file).ty.check(&value).unwrap_or_else(|e| panic!("{file} against {model_file}: {e}"));
    }
}

/// Each file's type annotates a definition's parameter, and nushell reads it
/// as nuval does.
#[cfg(feature = "nushell")]
#[test]
fn each_repository_nutype_annotates_a_parameter_as_nushell_reads_it() {
    for file in FILES {
        let model = model(file);
        let engine = nu_cmd_lang::create_default_context();
        let mut working_set = nu_protocol::engine::StateWorkingSet::new(&engine);
        let source = format!("def t [x: {}] {{}}", model.ty.to_block());
        nu_parser::parse(&mut working_set, None, source.as_bytes(), false);
        assert!(working_set.parse_errors.is_empty(), "{file}: {:?}", working_set.parse_errors);
        let decl = working_set.find_decl(b"t").expect("the definition");
        let shape = &working_set.get_decl(decl).signature().required_positional[0].shape;
        assert_eq!(nuval::NuType::try_from(shape).expect("a shape nuval holds"), model.ty, "{file}");
    }
}
