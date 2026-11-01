//! The deny list names a tool, or an interface command by its words.
use crate::*;

/// A command is denied by its words alone, never by a tool's own name.
#[test]
fn deny_names_a_command_by_its_words() {
    assert_eq!(
        DeniableTool::from_name("purview configure"),
        Some(DeniableTool::Command("purview configure")),
    );
    assert_eq!(DeniableTool::from_name(" equip commit "), Some(DeniableTool::Command("equip commit")));
    assert_eq!(DeniableTool::from_name("interface"), Some(DeniableTool::Interface));
    assert_eq!(DeniableTool::from_name("purview_configure"), None);
    assert_eq!(DeniableTool::from_name("equip"), None);
}
