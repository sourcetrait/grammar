use crate::*;

/// Renders a piped Liquid template string with a record of values.
pub(crate) struct StrSoakCommand;

/// The command's name.
const STR_SOAK: &str = "str soak";

impl nu::SimplePluginCommand for StrSoakCommand {
    type Plugin = SoakPlugin;

    fn name(&self) -> &str {
        STR_SOAK
    }

    fn description(&self) -> &str {
        "Render a piped Liquid template string using a record of values."
    }

    fn signature(&self) -> nu::Signature {
        nu::Signature::build(STR_SOAK)
            .required(
                "fill",
                nu::SyntaxShape::Record(vec![].into()),
                "Values applied to the template",
            )
            .input_output_types(vec![(nu::Type::String, nu::Type::String)])
            .category(nu::Category::Formats)
    }

    fn examples(&self) -> Vec<nu::Example<'_>> {
        vec![nu::Example {
            example: r#"'Hello, {{ name }}!' | str soak { name: World }"#,
            description: "Render a Liquid string against a fill record.",
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
        let fill = call.req::<nu::Value>(0)?;
        render_str(input.as_str()?, fill.as_record()?)
            .map_err(|e| e.nu(call.head))
    }
}
