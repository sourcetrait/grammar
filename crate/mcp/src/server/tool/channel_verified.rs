use crate::*;

impl ChannelFace {
    /// `interface channel ready`: the channel/Open packet seen; verify it.
    pub(crate) async fn channel_ready(
        &self,
        nonce: datum::Nonce,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.channel.request(ReadyRequest).await {
            Ok(Ok(epoch)) => {
                self.hub.disarm_verify(epoch);
                success_void(nonce)
            }
            Ok(Err(ChannelVerifyError::NotOpen)) => {
                error_to_call_result(GrammarMcpError::ChannelNotOpen, nonce)
            }
            Ok(Err(ChannelVerifyError::NotClaimed(mode))) => {
                error_to_call_result(GrammarMcpError::ChannelNotClaimed { mode }, nonce)
            }
            Err(reason) => error_to_call_result(channel_failed(reason), nonce),
        }
    }
}
