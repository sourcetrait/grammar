use crate::*;

/// Liquid templating: a template string, or a directory of templates.
pub struct SoakPlugin;

impl nu::Plugin for SoakPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn nu::PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(SoakDirCommand),
            Box::new(SoakStrCommand),
            Box::new(StrSoakCommand),
        ]
    }
}
