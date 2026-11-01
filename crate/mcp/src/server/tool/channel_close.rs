use crate::*;

/// RFC 6455 normal closure.
const CLOSE_PLANNED: u16 = 1000;
const PLANNED_REASON: &str = "channel closed";

impl ChannelFace {
    /// `interface channel close`: close the packet channel and its inbox.
    pub(crate) async fn channel_close(
        &self,
        nonce: datum::Nonce,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let close = CloseRequest {
            code: CLOSE_PLANNED,
            reason: PLANNED_REASON.to_string(),
            prune: true,
        };
        let closed = match self.channel.request(close).await {
            Ok(closed) => closed,
            Err(reason) => return error_to_call_result(channel_failed(reason), nonce),
        };
        if let Some(inbox) = closed.inbox {
            self.tell_inbox(None).await;
            remove_channel_dir(Path::new(&inbox));
        }
        success_void(nonce)
    }
}
