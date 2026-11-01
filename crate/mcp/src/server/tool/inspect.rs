use crate::*;

/// Success result of `interface help`.
#[derive(Debug, ser::Serialize)]
pub(crate) struct InspectResult {
    /// An exact namepath's documentation, or a pattern's `signatures` block.
    pub doc: InspectDoc,
}

impl NuSh {
    /// `interface help`: documentation for a rig, a module, a call, or a pattern.
    pub(crate) async fn help(
        &self,
        nonce: datum::Nonce,
        route: &Route,
        p: InspectParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let words = match p.namepath.as_str() {
            INTERFACE => Some(render_signatures(&self.controller.config.deny)),
            GRIMM => Some(grimm_signatures()),
            _ => None,
        };
        if let Some(signatures) = words {
            return success(
                nonce,
                &InspectResult {
                    doc: InspectDoc::Signatures(SignaturesDoc { signatures }),
                },
            );
        }
        let pattern = match NamepathStr::parse(&p.namepath) {
            Ok(NamepathStr::Pattern(pattern)) => pattern,
            Ok(NamepathStr::Namepath(_)) => {
                let command = EngineCommand::Help {
                    namepath: p.namepath,
                };
                return match self.control_on(route, command).await {
                    Ok(ControlAnswer::Help(doc)) => success(nonce, &InspectResult { doc }),
                    Ok(_) => error_to_call_result(unexpected("help"), nonce),
                    Err(error) => error_to_call_result(error, nonce),
                };
            }
            Err(e) => return error_to_call_result(e, nonce),
        };
        let patterns = match pattern {
            NamepathPattern::Current => match self.resolve_purviews(route, self.views.of(route).ids()).await {
                Ok(view) => view.patterns,
                Err(error) => return error_to_call_result(error, nonce),
            },
            _ => vec![p.namepath],
        };
        match self.signatures(route, patterns).await {
            Ok(signatures) => success(
                nonce,
                &InspectResult {
                    doc: InspectDoc::Signatures(SignaturesDoc { signatures }),
                },
            ),
            Err(error) => error_to_call_result(error, nonce),
        }
    }
}
