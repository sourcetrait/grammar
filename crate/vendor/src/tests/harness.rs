//! A client's harness by its `clientInfo` name.
use crate::*;

#[test]
fn a_client_name_names_its_harness() {
    assert_eq!(Harness::of(CLAUDE_CLIENT), Harness::Claude);
    assert_eq!(Harness::of(CODEX_CLIENT), Harness::Codex);
    assert_eq!(Harness::of("other-client"), Harness::Other("other-client".to_string()));
}
