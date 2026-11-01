//! nutype's types to nushell's shapes and types and back.
use crate::*;

/// Types of every shape, alone and nested.
fn types() -> Vec<NuType> {
    let scalars = vec![
        NuType::Any, NuType::Nothing, NuType::Bool, NuType::Int, NuType::Float, NuType::Number,
        NuType::String, NuType::Path, NuType::Directory, NuType::Glob, NuType::Binary, NuType::Date,
        NuType::Duration, NuType::Filesize, NuType::Range, NuType::CellPath,
    ];
    let mut types = scalars.clone();
    for scalar in &scalars {
        types.push(NuType::List(Box::new(scalar.clone())));
        types.push(NuType::Record(vec![NuField::new("a", scalar.clone()), NuField::new("b c", NuType::Int)]));
        types.push(NuType::Table(vec![NuField::new("col", scalar.clone())]));
    }
    types.push(NuType::Record(Vec::new()));
    types.push(NuType::Table(Vec::new()));
    types.push(NuType::OneOf(vec![NuType::Int, NuType::Nothing]));
    types.push(NuType::Record(vec![NuField::new("x", NuType::List(Box::new(NuType::Record(vec![NuField::new("y", NuType::Path)]))))]));
    types
}

#[test]
fn every_type_crosses_nushells_shape_unchanged() {
    for ty in types() {
        let shape = nu::SyntaxShape::from(&ty);
        assert_eq!(NuType::try_from(&shape).expect("back"), ty, "{shape}");
    }
}

#[test]
fn every_type_crosses_nushells_type_resolved() {
    for ty in types().into_iter().filter(|ty| !matches!(ty, NuType::OneOf(_))) {
        let nushell = nu::Type::from(&ty);
        assert_eq!(NuType::try_from(&nushell).expect("back"), ty.resolved(), "{nushell}");
    }
}

#[test]
fn a_shape_no_value_takes_is_refused() {
    for shape in [nu::SyntaxShape::Closure(None), nu::SyntaxShape::Block, nu::SyntaxShape::Error] {
        assert!(matches!(NuType::try_from(&shape), Err(NuTypeError::Unsupported { .. })), "{shape}");
    }
    assert!(matches!(NuType::try_from(&nu::Type::Closure), Err(NuTypeError::Unsupported { .. })));
}
