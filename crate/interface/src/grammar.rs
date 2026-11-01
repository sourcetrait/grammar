//! The interface commands in the parse: their decls and their isolated engine.
use crate::*;

/// The parse's shape for an argument's.
pub(crate) fn syntax(shape: ArgShape) -> nu::SyntaxShape {
    match shape {
        ArgShape::String => nu::SyntaxShape::String,
        ArgShape::Strings => nu::SyntaxShape::List(Box::new(nu::SyntaxShape::String)),
        ArgShape::Int => nu::SyntaxShape::Int,
    }
}

/// The signature the parse checks a command's arguments against.
pub(crate) fn signature(spec: &CommandSpec) -> nu::Signature {
    let mut signature = nu::Signature::build(spec.name)
        .input_output_type(nu::Type::Nothing, nu::Type::Any)
        .category(nu::Category::Custom(INTERFACE.to_string()));
    for arg in spec.positionals {
        signature = if arg.optional {
            signature.optional(arg.name, syntax(arg.shape), arg.summary)
        } else {
            signature.required(arg.name, syntax(arg.shape), arg.summary)
        };
    }
    for flag in spec.flags {
        signature = signature.named(flag.name, syntax(flag.shape), flag.summary, None);
    }
    signature
}

/// A signature-only decl the parse resolves a command to; it never runs.
#[derive(Clone)]
pub(crate) struct InterfaceDecl {
    spec: &'static CommandSpec,
}

impl nu::Command for InterfaceDecl {
    fn name(&self) -> &str {
        self.spec.name
    }

    fn signature(&self) -> nu::Signature {
        signature(self.spec)
    }

    fn description(&self) -> &str {
        self.spec.summary
    }

    fn run(
        &self,
        _engine_state: &nu::EngineState,
        _stack: &mut nu::Stack,
        call: &nu::Call<'_>,
        _input: nu::PipelineData,
    ) -> Result<nu::PipelineData, nu::ShellError> {
        Err(nu::GenericError::new(
            format!("`{}` is never evaluated", self.spec.name),
            "an interface command is parsed and dispatched by the host",
            call.head,
        )
        .into())
    }
}

/// An engine holding the commands `denied` leaves by their words, nothing else.
pub fn interface_engine(denied: impl Fn(&str) -> bool) -> nu::EngineState {
    let mut engine_state = nu::EngineState::new();
    let mut working_set = nu::StateWorkingSet::new(&engine_state);
    for spec in COMMANDS.iter().filter(|spec| !denied(spec.words())) {
        working_set.add_decl(Box::new(InterfaceDecl { spec }));
    }
    let delta = working_set.render();
    let _ = engine_state.merge_delta(delta);
    engine_state
}
