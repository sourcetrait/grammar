//! The v1 gate: every golden vector decoded by the fixture driver to its own
//! bytes, its truncations and a trailing byte refused, random and mutated bytes
//! decoded within the budget without a panic and canonical once decoded, every
//! field of a fixed width, and the manifest pinning every vector file, the
//! schema, and the codec.

use std::path::Path;

use nuval::{NuVal, NuonStyle};
use sourcetrait_grammar_protocol::fixture::{self, FixtureVector};
use sourcetrait_grammar_protocol::*;

#[path = "support/golden.rs"]
mod golden;

/// The manifest pinning the v1 vectors.
const MANIFEST: &str = "tests/vectors/manifest.nuon";

/// The manifest's model.
const MANIFEST_MODEL: &str = "tests/vectors/Manifest.nutype";

/// The vector files, the budget's payloads apart: they are refused by design.
const FILES: [&str; 5] = ["engine.nuon", "hello.nuon", "peer.nuon", "session.nuon", "value.nuon"];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn text(file: &str) -> String {
    std::fs::read_to_string(root().join("tests/vectors").join(file)).unwrap_or_else(|e| panic!("{file}: {e}"))
}

fn vectors() -> Vec<(&'static str, FixtureVector)> {
    FILES
        .iter()
        .flat_map(|file| fixture::read_vectors(&text(file)).unwrap_or_else(|e| panic!("{file}: {e}")).into_iter().map(move |v| (*file, v)))
        .collect()
}

#[test]
fn every_vector_decodes_by_its_type_to_its_own_bytes_and_refuses_malformed_forms() {
    let all = vectors();
    for file in FILES {
        assert!(all.iter().any(|(held, _)| *held == file), "{file} holds no vector");
    }
    for (file, vector) in &all {
        fixture::check_vector(vector, DECODE_BUDGET_DEFAULT).unwrap_or_else(|e| panic!("{file}: {e}"));
    }
    let named: std::collections::BTreeSet<_> = all.iter().map(|(_, v)| v.ty.as_str()).collect();
    let known: std::collections::BTreeSet<_> = fixture::FIXTURE_TYPES.iter().copied().collect();
    assert_eq!(named, known, "every driver type names some vector, and every vector a driver type");
}

/// A deterministic generator, so a failure names its own seed.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
fn random_and_mutated_bytes_decode_within_the_budget_and_canonicalize() {
    let mut rng = XorShift(0x9e37_79b9_7f4a_7c15);
    let all = vectors();
    for ty in fixture::FIXTURE_TYPES {
        let seeds: Vec<&FixtureVector> = all.iter().map(|(_, v)| v).filter(|v| v.ty == *ty).collect();
        let mut inputs: Vec<Vec<u8>> = (0..300)
            .map(|_| {
                let len = (rng.next() % 129) as usize;
                (0..len).map(|_| rng.next() as u8).collect()
            })
            .collect();
        for seed in &seeds {
            for _ in 0..40 {
                let mut bytes = seed.bytes.clone();
                if !bytes.is_empty() {
                    let at = (rng.next() as usize) % bytes.len();
                    bytes[at] ^= 1 << (rng.next() % 8);
                }
                inputs.push(bytes);
            }
        }
        for input in inputs {
            if let Some(Ok(canonical)) = fixture::canonical(ty, &input, DECODE_BUDGET_DEFAULT) {
                let again = fixture::canonical(ty, &canonical, DECODE_BUDGET_DEFAULT);
                assert!(matches!(&again, Some(Ok(bytes)) if *bytes == canonical), "{ty}: {input:02x?} isn't canonical once decoded");
            }
        }
    }
}

#[test]
fn every_field_type_has_a_fixed_width() {
    let text = std::fs::read_to_string(root().join("schema/v1.nuon")).expect("the schema");
    let NuVal::Record(tables) = NuVal::from_nuon(&text).expect("the schema's NUON") else { panic!("a record") };
    let (_, NuVal::List(fields)) = tables.iter().find(|(name, _)| name == "fields").expect("its fields") else {
        panic!("a fields table")
    };
    for field in fields {
        let NuVal::Record(columns) = field else { panic!("a field row") };
        let Some((_, NuVal::String(ty))) = columns.iter().find(|(name, _)| name == "type") else { panic!("a type") };
        let words: Vec<&str> = ty.split(|c: char| !c.is_alphanumeric() && c != '_').collect();
        assert!(!words.iter().any(|w| *w == "usize" || *w == "isize"), "a field of {ty}, its width the target's");
    }
}

/// The manifest as the vector files and the schema stand.
fn manifest() -> NuVal {
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let files = FILES
        .iter()
        .map(|file| {
            let digest = fixture::digest(&text(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
            let count = fixture::read_vectors(&text(file)).expect("read").len();
            golden::row([
                ("file", NuVal::String(file.to_string())),
                ("vectors", NuVal::Int(count as i64)),
                ("sha256", NuVal::String(hex(&digest))),
            ])
        })
        .collect();
    let version = format!("{}.{}", PROTOCOL_VERSION.major, PROTOCOL_VERSION.minor);
    golden::row([
        ("version", NuVal::String(version)),
        ("schema_id", NuVal::String(hex(&SCHEMA_ID))),
        ("codec", NuVal::String(CODEC_ID.to_string())),
        ("files", NuVal::List(files)),
    ])
}

#[test]
fn the_manifest_pins_every_vector_file_the_schema_and_the_codec() {
    let actual = manifest();
    let golden = golden::read(MANIFEST, MANIFEST_MODEL, &actual);
    assert_eq!(golden, actual, "a pinned file moved; a deliberate change re-blesses {MANIFEST}");
    // A checkout's line endings and the files' layout move no digest.
    for file in FILES {
        let text = text(file);
        let crlf = text.replace('\n', "\r\n");
        assert_eq!(fixture::digest(&crlf), fixture::digest(&text), "{file}");
        let raw = NuVal::from_nuon(&text).expect("NUON").to_nuon(NuonStyle::Raw);
        assert_eq!(fixture::digest(&raw), fixture::digest(&text), "{file}");
    }
}
