use crate::*;

impl NuSh {
    /// `interface process list`: every call in flight on the routed engine.
    pub(crate) async fn process_list(
        &self,
        nonce: datum::Nonce,
        route: &Route,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        let rows = match self.control_on(route, EngineCommand::Processes).await {
            Ok(ControlAnswer::Processes(rows)) => rows,
            Ok(_) => return error_to_call_result(unexpected("processes"), nonce),
            Err(error) => return error_to_call_result(error, nonce),
        };
        let span = nu::Span::unknown();
        let mut processes = Vec::with_capacity(rows.len());
        for row in rows {
            let args = match row.args.try_into_nu(span) {
                Ok(args) => args,
                Err(e) => {
                    return error_to_call_result(
                        GrammarMcpError::Internal {
                            phase: "process_list::args".to_string(),
                            reason: e.to_string(),
                        },
                        nonce,
                    );
                }
            };
            let mut record = nu::Record::new();
            record.insert("nonce", nu::Value::string(row.nonce, span));
            record.insert("tool", nu::Value::string(row.tool, span));
            record.insert("started_at", nu::Value::int(row.started_at as i64, span));
            record.insert("args", args);
            if let Some(source_nonce) = row.source_nonce {
                record.insert("source_nonce", nu::Value::string(source_nonce, span));
            }
            if let Some(path) = row.path {
                record.insert("path", nu::Value::string(path, span));
            }
            processes.push(nu::Value::record(record, span));
        }
        let mut result = nu::Record::new();
        result.insert("processes", nu::Value::list(processes, span));
        success_value(nonce, nu::Value::record(result, span))
    }
}
