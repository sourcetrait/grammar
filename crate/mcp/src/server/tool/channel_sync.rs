use crate::*;

/// The packet field naming an attachment, absent from a line without one.
const ATTACHED: &str = "attached";

impl ChannelFace {
    /// `interface channel sync`: the packets a sync channel queued since the last.
    pub(crate) async fn channel_sync(
        &self,
        nonce: datum::Nonce,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let synced = match self.channel.request(SyncRequest).await {
            Ok(Ok(synced)) => synced,
            Ok(Err(ChannelSyncError::NotOpen)) => {
                return error_to_call_result(GrammarMcpError::ChannelNotOpen, nonce);
            }
            Ok(Err(ChannelSyncError::WrongMode(open))) => {
                return error_to_call_result(
                    GrammarMcpError::ChannelWrongMode {
                        open,
                        asked: ChannelMode::Sync,
                    },
                    nonce,
                );
            }
            Err(reason) => return error_to_call_result(channel_failed(reason), nonce),
        };
        let packets = match synced.lines.iter().map(|line| packet_record(line)).collect() {
            Ok(packets) => packets,
            Err(reason) => {
                return error_to_call_result(
                    GrammarMcpError::Internal {
                        phase: "channel_sync::packet".to_string(),
                        reason,
                    },
                    nonce,
                );
            }
        };
        let span = nu::Span::unknown();
        let mut result = nu::Record::new();
        result.insert("packets", nu::Value::list(packets, span));
        result.insert("dropped", nu::Value::int(synced.dropped as i64, span));
        success_value(nonce, nu::Value::record(result, span))
    }
}

/// A queued line read back into its packet, `attached` null where it names none.
fn packet_record(line: &str) -> Result<nu::Value, String> {
    let span = nu::Span::unknown();
    let value = nu::from_nuon(line, None).map_err(|e| e.to_string())?;
    let mut packet = value
        .as_record()
        .map_err(|_| "a queued packet is not a record".to_string())?
        .clone();
    if !packet.contains(ATTACHED) {
        packet.insert(ATTACHED, nu::Value::nothing(span));
    }
    Ok(nu::Value::record(packet, span))
}
