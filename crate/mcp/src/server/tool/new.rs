use crate::*;

/// Success result of `interface equip scaffold`: every path the batch made.
#[derive(Debug, ser::Serialize)]
pub(crate) struct NewResult {
    pub created: Vec<String>,
}

impl NuSh {
    /// `interface equip scaffold`: modules and calls scaffolded into rigs.
    pub(crate) async fn equip_scaffold(
        &self,
        nonce: datum::Nonce,
        route: &Route,
        p: NewParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let command = EngineCommand::EquipScaffold {
            namepaths: p.namepaths,
        };
        match self.control_on(route, command).await {
            Ok(ControlAnswer::Created(created)) => success(nonce, &NewResult { created }),
            Ok(_) => error_to_call_result(unexpected("equip_scaffold"), nonce),
            Err(e) => error_to_call_result(e, nonce),
        }
    }
}
