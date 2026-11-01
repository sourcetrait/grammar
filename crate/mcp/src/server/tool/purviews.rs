use crate::*;

/// Success result of `interface purview list`.
#[derive(Debug, ser::Serialize)]
pub(crate) struct PurviewsResult {
    /// Every configured purview, as persisted.
    pub purviews: Vec<PurviewView>,
    /// The purview ids in view right now - the keys alone.
    pub current: Vec<String>,
}

impl NuSh {
    /// `interface purview list`: the routed engine's purviews, and what is in view.
    pub(crate) async fn purview_list(
        &self,
        nonce: datum::Nonce,
        route: &Route,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let purviews = match self.purviews_on(route).await {
            Ok(purviews) => purviews,
            Err(error) => return error_to_call_result(error, nonce),
        };
        success(
            nonce,
            &PurviewsResult {
                purviews,
                current: self.views.of(route).ids(),
            },
        )
    }
}
