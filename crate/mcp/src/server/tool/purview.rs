use crate::*;

/// Success result of `interface purview set`.
#[derive(Debug, ser::Serialize)]
pub(crate) struct PurviewSetResult {
    /// `info`'s `signatures` for what this call brought into view, or null.
    pub revealed: Option<String>,
}

impl NuSh {
    /// `interface purview set`: the routed engine's current view set to these purviews.
    pub(crate) async fn purview_set(
        &self,
        nonce: datum::Nonce,
        route: &Route,
        p: PurviewParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let want: Vec<String> = p
            .purviews
            .iter()
            .map(|id| purview_ref(id).unwrap_or(id).to_string())
            .collect();
        let wanted = match self.resolve_purviews(route, want.clone()).await {
            Ok(wanted) => wanted,
            Err(error) => return error_to_call_result(error, nonce),
        };
        if let Some(id) = wanted.unknown.first() {
            return error_to_call_result(unknown_purview(id), nonce);
        }
        let current = self.views.of(route);
        let before = match self.resolve_purviews(route, current.ids()).await {
            Ok(before) => before,
            Err(error) => return error_to_call_result(error, nonce),
        };
        let ids = current.set(&want);
        let after = if ids == want {
            wanted
        } else {
            match self.resolve_purviews(route, ids).await {
                Ok(after) => after,
                Err(error) => return error_to_call_result(error, nonce),
            }
        };
        let (revealed, _hidden) = pattern_delta(&before.patterns, &after.patterns);
        let block = if revealed.is_empty() {
            None
        } else {
            match self.signatures(route, revealed).await {
                Ok(signatures) => Some(signatures),
                Err(error) => return error_to_call_result(error, nonce),
            }
        };
        success(nonce, &PurviewSetResult { revealed: block })
    }
}
