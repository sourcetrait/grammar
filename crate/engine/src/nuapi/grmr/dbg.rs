use crate::*;

/// `grmr dbg <data>` - append a record or table to this eval's `debug.nuonl`.
#[derive(Clone)]
pub(crate) struct GrmrDbg;

impl nu::Command for GrmrDbg {
    fn name(&self) -> &str {
        "grmr dbg"
    }

    fn signature(&self) -> nu::Signature {
        nu::Signature::build("grmr dbg")
            .required("data", data_shape(), "the record or table to append")
            .input_output_types(vec![(nu::Type::Nothing, nu::Type::Nothing)])
            .category(nu::Category::Custom("grmr".to_string()))
    }

    fn description(&self) -> &str {
        "Append a record or table to this eval's debug.nuonl in its nonce log dir."
    }

    fn run(
        &self,
        engine_state: &nu::EngineState,
        stack: &mut nu::Stack,
        call: &nu::Call<'_>,
        _input: nu::PipelineData,
    ) -> Result<nu::PipelineData, nu::ShellError> {
        let data: nu::Value = call.req(engine_state, stack, 0)?;
        require_record_or_table(&data, call.head)?;
        NuapiCall::current(engine_state, stack, call.head)?.append_debug(&data, call.head)?;
        Ok(nu::PipelineData::Empty)
    }
}
