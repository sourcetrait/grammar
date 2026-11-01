use crate::*;

/// Renders a Liquid template string with a piped record of values.
pub(crate) struct SoakStrCommand;

/// The command's name.
const SOAK_STR: &str = "soak str";

impl nu::SimplePluginCommand for SoakStrCommand {
    type Plugin = SoakPlugin;

    fn name(&self) -> &str {
        SOAK_STR
    }

    fn description(&self) -> &str {
        "Render a Liquid template string using a piped record of values."
    }

    fn signature(&self) -> nu::Signature {
        nu::Signature::build(SOAK_STR)
            .required(
                "liquid",
                nu::SyntaxShape::String,
                "Liquid template",
            )
            .input_output_types(vec![(
                nu::Type::record(),
                nu::Type::String,
            )])
            .category(nu::Category::Formats)
    }

    fn examples(&self) -> Vec<nu::Example<'_>> {
        vec![nu::Example {
            example: r#"{ name: World } | soak str 'Hello, {{ name }}!'"#,
            description: "Render a Liquid template against a piped record.",
            result: Some(nu::Value::string("Hello, World!", consts::NOSPAN)),
        }]
    }

    fn run(
        &self,
        _plugin: &SoakPlugin,
        _engine: &nu::EngineInterface,
        call: &nu::EvaluatedCall,
        input: &nu::Value,
    ) -> Result<nu::Value, nu::LabeledError> {
        let template = call.req::<nu::Value>(0)?;
        render_str(template.as_str()?, input.as_record()?)
            .map_err(|e| e.nu(call.head))
    }
}
