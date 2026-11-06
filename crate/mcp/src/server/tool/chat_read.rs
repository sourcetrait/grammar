use crate::*;

/// The most bytes a page of a chat's message takes.
const PAGE_BYTES: usize = 4096;

impl ChannelFace {
    /// `interface chat read`: a chat's `message.md` in the Channel's inbox, by page.
    pub(crate) async fn chat_read(
        &self,
        nonce: datum::Nonce,
        p: ChatReadParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.read_chat(p).await {
            Ok(page) => success_value(nonce, page),
            Err(error) => error_to_call_result(error, nonce),
        }
    }

    /// The page asked of the chat's message, with the count of its pages.
    async fn read_chat(
        &self,
        p: ChatReadParams,
    ) -> GrammarMcpResult<nu::Value> {
        let ChatReadParams { id, page } = p;
        let invalid = |reason: String| GrammarMcpError::RemoteInvalidParams { reason };
        if !valid_peer_id(&id) {
            return Err(invalid(format!("`{id}` is no packet id: base62 alone")));
        }
        let inbox = self.status().await?.inbox.ok_or(GrammarMcpError::ChannelNotOpen)?;
        let file = PathBuf::from(inbox).join(&id).join(CHAT_MESSAGE_FILE);
        let text = tk::spawn_blocking(move || fs::read_to_string(&file).map_err(|e| (file, e)))
            .await
            .map_err(|e| invalid(format!("the read of `{id}`'s message failed: {e}")))?
            .map_err(|(file, e)| invalid(format!("no chat message at {}: {e}", file.display())))?;
        let pages = page_bounds(&text);
        let index = usize::try_from(page)
            .ok()
            .filter(|index| *index < pages.len())
            .ok_or_else(|| invalid(format!("`{id}`'s message has {} pages, 0 first", pages.len())))?;
        let (start, end) = pages[index];
        let span = nu::Span::unknown();
        let mut record = nu::Record::new();
        record.insert("id", nu::Value::string(id, span));
        record.insert("page", nu::Value::int(index as i64, span));
        record.insert("pages", nu::Value::int(pages.len() as i64, span));
        record.insert("text", nu::Value::string(text[start..end].to_string(), span));
        Ok(nu::Value::record(record, span))
    }
}

/// Each page's byte range: at most `PAGE_BYTES`, cut after its last line break.
///
/// A page holding no line break is cut at the last character boundary within it;
/// an empty message is one empty page.
pub(crate) fn page_bounds(text: &str) -> Vec<(usize, usize)> {
    let mut pages = Vec::new();
    let mut start = 0;
    while text.len() - start > PAGE_BYTES {
        let mut end = start + PAGE_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let cut = match text[start..end].rfind('\n') {
            Some(at) => start + at + 1,
            None => end,
        };
        pages.push((start, cut));
        start = cut;
    }
    pages.push((start, text.len()));
    pages
}
