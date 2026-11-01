//! An instance's names, its id and its namespace, each held to one form.
use crate::*;

/// An instance's id or namespace, `[A-Za-z0-9][A-Za-z0-9._-]{0,63}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InstanceName(String);

impl InstanceName {
    /// `text` as the `field` it names, refused where it isn't a name.
    pub fn parse(
        field: &'static str,
        text: &str,
    ) -> GrammarSshResult<Self> {
        if valid_name(text) {
            Ok(Self(text.to_string()))
        } else {
            Err(GrammarSshError::NameInvalid {
                field,
                name: text.to_string(),
            })
        }
    }

    /// The name's text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for InstanceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
