use crate::*;

/// Renders a directory of Liquid templates with a piped record of values.
pub(crate) struct SoakDirCommand;

/// The command's name.
const SOAK_DIR: &str = "soak dir";

impl nu::SimplePluginCommand for SoakDirCommand {
    type Plugin = SoakPlugin;

    fn name(&self) -> &str {
        SOAK_DIR
    }

    fn description(&self) -> &str {
        "Render a directory of Liquid templates using a piped record of values."
    }

    fn extra_description(&self) -> &str {
        "Each `<name>.liquid` renders to `<name>`, each other file is copied, and \
         directories are mirrored. `<src>/liquid/` holds the partials that \
         `include` and `render` name and is not emitted; dotted entries are \
         skipped and symlinks followed. An output already in `dst` is \
         overwritten and every other file there stays. Every template renders \
         before anything is written, so a failed soak leaves `dst` as it was."
    }

    fn signature(&self) -> nu::Signature {
        nu::Signature::build(SOAK_DIR)
            .required(
                "src",
                nu::SyntaxShape::Directory,
                "Source directory",
            )
            .required(
                "dst",
                nu::SyntaxShape::Directory,
                "Destination directory",
            )
            .input_output_types(vec![(
                nu::Type::record(),
                nu::Type::Nothing,
            )])
            .category(nu::Category::Formats)
    }

    fn examples(&self) -> Vec<nu::Example<'_>> {
        vec![nu::Example {
            example: r#"{ name: World } | soak dir /tmp/input-dir /tmp/output-dir"#,
            description: "Render a directory of Liquid templates against a piped record.",
            result: None,
        }]
    }

    fn run(
        &self,
        _plugin: &SoakPlugin,
        engine: &nu::EngineInterface,
        call: &nu::EvaluatedCall,
        input: &nu::Value,
    ) -> Result<nu::Value, nu::LabeledError> {
        let src = call.req::<nu::Value>(0)?;
        let dst = call.req::<nu::Value>(1)?;
        let src_path = resolve_arg(engine, src.as_str()?)?;
        let dst_path = resolve_arg(engine, dst.as_str()?)?;
        render_dir(
            PathSpan { path: &src_path, span: src.span() },
            PathSpan { path: &dst_path, span: dst.span() },
            input.as_record()?,
        ).map_err(|e| e.nu(call.head))
    }
}

/// `raw` as an absolute path, `~` expanded, relative to the caller's directory.
fn resolve_arg(engine: &nu::EngineInterface, raw: &str) -> Result<PathBuf, nu::LabeledError> {
    let expanded = nu::expand_path(raw, true);
    if expanded.is_absolute() {
        return Ok(expanded);
    }
    let cwd = engine.get_current_dir()?;
    Ok(nu::expand_path_with(raw, cwd, true))
}
