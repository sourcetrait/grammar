use crate::*;

impl NuSh {
    /// `interface equip commit`: an equip's source promoted into the routed namespace.
    pub(crate) async fn equip_commit(
        &self,
        nonce: datum::Nonce,
        route: &Route,
        p: CommitParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let command = EngineCommand::EquipCommit {
            namepath: p.namepath,
        };
        match self.control_on(route, command).await {
            Ok(ControlAnswer::Committed(result)) => success(nonce, &result),
            Ok(_) => error_to_call_result(unexpected("equip_commit"), nonce),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}
