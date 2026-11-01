use crate::*;

/// Success result of `interface purview configure`, the state after the write.
#[derive(Debug, ser::Serialize)]
pub(crate) struct PurviewConfigureResult {
    /// Everything this purview reveals; null when the call deleted it.
    pub signatures: Option<String>,
}

impl NuSh {
    /// `interface purview configure`: a purview's patterns set on the routed engine.
    ///
    /// An empty list deletes the purview.
    pub(crate) async fn purview_configure(
        &self,
        nonce: datum::Nonce,
        route: &Route,
        p: PurviewConfigureParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let configure = PurviewRequest::Configure(PurviewConfigureRequest {
            id: p.purview.clone(),
            patterns: p.namepaths,
        });
        let rows = match self.purview_rows_on(route, configure).await {
            Ok(rows) => rows,
            Err(error) => return error_to_call_result(error, nonce),
        };
        self.views.of(route).retain_known(&rows);
        let signatures = if rows.iter().any(|row| row.id == p.purview) {
            let view = match self.resolve_purviews(route, vec![p.purview]).await {
                Ok(view) => view,
                Err(error) => return error_to_call_result(error, nonce),
            };
            match self.signatures(route, view.patterns).await {
                Ok(signatures) => Some(signatures),
                Err(error) => return error_to_call_result(error, nonce),
            }
        } else {
            None
        };
        success(nonce, &PurviewConfigureResult { signatures })
    }
}
