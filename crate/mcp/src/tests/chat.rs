//! A chat message's pages: their bound, their cut at a line break, and their text.
use crate::server::tool::chat_read::page_bounds;

/// Each page's text, by its bounds.
fn pages(text: &str) -> Vec<&str> {
    page_bounds(text).into_iter().map(|(start, end)| &text[start..end]).collect()
}

#[test]
fn a_short_message_is_one_page_and_an_empty_one_one_empty_page() {
    assert_eq!(pages("a line\nanother"), ["a line\nanother"]);
    assert_eq!(pages(""), [""]);
}

#[test]
fn a_long_message_is_cut_after_the_last_line_break_within_each_page() {
    let line = format!("{}\n", "x".repeat(99));
    let text = line.repeat(100);
    let cut = pages(&text);
    assert_eq!(cut.concat(), text, "the pages rebuild the message");
    assert!(cut.iter().all(|page| page.len() <= 4096), "no page past 4 KiB");
    assert!(cut[..cut.len() - 1].iter().all(|page| page.ends_with('\n')), "each page but the last ends a line");
    assert_eq!(cut[0].len(), 4000, "forty whole lines fit a page");
}

#[test]
fn a_page_with_no_line_break_is_cut_at_a_character_boundary() {
    let text = "é".repeat(3000);
    let cut = pages(&text);
    assert_eq!(cut.concat(), text);
    assert_eq!(cut[0].len(), 4096, "two-byte characters fill the page exactly");
    let text = format!("a{}", "é".repeat(3000));
    let cut = pages(&text);
    assert_eq!(cut.concat(), text);
    assert_eq!(cut[0].len(), 4095, "a character spanning the bound goes to the next page");
}
