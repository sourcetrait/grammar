use crate::*;

#[test]
fn an_alias_past_its_cap_is_refused_and_one_at_it_taken() {
    let at_cap = format!("{}/{}", "a".repeat(64), "b".repeat(63));
    assert_eq!(at_cap.chars().count(), ALIAS_MAX_CHARS);
    assert_eq!(check_alias(&at_cap), Ok(()), "an alias at the cap is taken");
    let past = format!("{}/{}", "a".repeat(64), "b".repeat(64));
    let refused = check_alias(&past).expect_err("an alias past the cap, each name valid");
    assert!(
        refused.contains(&format!("passes {ALIAS_MAX_CHARS} characters")),
        "{refused}"
    );
}
