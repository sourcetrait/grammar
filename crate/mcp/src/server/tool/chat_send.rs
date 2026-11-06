use crate::*;

/// The event field a chat's message rides in, or its size where it spilled.
const MESSAGE: &str = "message";

impl ChannelFace {
    /// `interface chat send`: a chat to a linked host, its message as `message.md`.
    pub(crate) async fn chat_send(
        &self,
        nonce: datum::Nonce,
        p: ChatSendParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.send_chat(p).await {
            Ok(id) => {
                let span = nu::Span::unknown();
                let mut record = nu::Record::new();
                record.insert("event_id", nu::Value::string(id, span));
                success_value(nonce, nu::Value::record(record, span))
            }
            Err(error) => error_to_call_result(error, nonce),
        }
    }

    /// The chat's checks, its message spilled ahead of the line's spill, and its send.
    async fn send_chat(
        &self,
        p: ChatSendParams,
    ) -> GrammarMcpResult<String> {
        let ChatSendParams {
            mcp_nom,
            summary,
            message,
        } = p;
        check_summary(&summary)?;
        let from = format!("{REMOTE_RESERVED_PREFIX}channel/{}", self.mcp_nom.as_str());
        let attached = [AttachedFile {
            file: CHAT_MESSAGE_FILE.to_string(),
            size: message.len() as u64,
        }];
        let span = nu::Span::unknown();
        let inline = chat_event(&summary, nu::Value::string(message.clone(), span));
        let event = if line_fits(&from, CHAT_MODEL, &inline, Some(&attached)).map_err(internal)? {
            inline
        } else {
            chat_event(&summary, event_spill_pointer(message.len()))
        };
        let send = RemoteSendRequest {
            mcp_nom,
            model: CHAT_MODEL.to_string(),
            event_nuon: render_nuon(&event).map_err(internal)?,
            payloads: vec![Payload {
                dest: CHAT_MESSAGE_FILE.to_string(),
                bytes: message.into_bytes(),
            }],
        };
        match self.peer.request(HostSendRequest(send)).await {
            Ok(Ok(id)) => Ok(id),
            Ok(Err(reason)) => Err(GrammarMcpError::RemoteInvalidParams { reason }),
            Err(reason) => Err(peer_failed(reason)),
        }
    }
}

/// A summary the packet always carries: one line, not blank, 80 characters at most.
fn check_summary(summary: &str) -> GrammarMcpResult<()> {
    let refused = |why: &str| GrammarMcpError::RemoteInvalidParams {
        reason: format!("a chat's summary is {why}"),
    };
    if summary.trim().is_empty() {
        return Err(refused("one line, never blank"));
    }
    if summary.contains(['\n', '\r']) {
        return Err(refused("one line"));
    }
    if summary.chars().count() > CHAT_SUMMARY_MAX_CHARS {
        return Err(refused(&format!("{CHAT_SUMMARY_MAX_CHARS} characters at most")));
    }
    Ok(())
}

/// A chat's event: its summary, and its message inline or as its size.
fn chat_event(
    summary: &str,
    message: nu::Value,
) -> nu::Value {
    let span = nu::Span::unknown();
    let mut record = nu::Record::new();
    record.insert("summary", nu::Value::string(summary.to_string(), span));
    record.insert(MESSAGE, message);
    nu::Value::record(record, span)
}

/// A render that failed, which a chat's own fields can't make happen.
fn internal(reason: String) -> GrammarMcpError {
    GrammarMcpError::Internal {
        phase: "chat_send::render".to_string(),
        reason,
    }
}
