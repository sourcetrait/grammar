use crate::*;

#[test]
fn size_label_floors_to_the_largest_unit() {
    assert_eq!(size_label(0), "0");
    assert_eq!(size_label(1), "1b");
    assert_eq!(size_label(1023), "1023b");
    assert_eq!(size_label(1024), "1kb");
    assert_eq!(size_label(1536), "1kb");
    assert_eq!(size_label(1024 * 1024 - 1), "1023kb");
    assert_eq!(size_label(1024 * 1024), "1mb");
    assert_eq!(size_label(32 * 1024 * 1024), "32mb");
    assert_eq!(size_label(1024 * 1024 * 1024), "1gb");
    assert_eq!(size_label(1024_u64.pow(4)), "1tb");
    assert_eq!(size_label(5 * 1024_u64.pow(4)), "5tb");
}

#[test]
fn wrap_backticks_only_names_with_spaces() {
    assert_eq!(wrap("file.txt"), "file.txt");
    assert_eq!(wrap("my file"), "`my file`");
    assert_eq!(wrap("a b c"), "`a b c`");
}
