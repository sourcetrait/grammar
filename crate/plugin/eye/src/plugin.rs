use crate::*;

/// File inspection: a directory's compact sized tree, and a repository's.
pub struct EyePlugin;

impl nu::Plugin for EyePlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn nu::PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(EyeDirCommand),
            Box::new(EyeGitCommand),
        ]
    }
}
