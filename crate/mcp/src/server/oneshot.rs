use crate::*;

/// One tool call on an unsupervised controller, its envelope printed.
pub(crate) async fn run_oneshot(
    config: Arc<Config>,
    tool: CliTool,
    lock: NamespaceLock,
) -> process::ExitCode {
    let exit_code = async move {
        install_child_subreaper();
        if let Err(e) = ensure_substrate().await {
            eprintln!("grammar: the equip substrate: {e}");
            return 1;
        }
        let controller = match McpController::start(config, false, Some(lock)).await {
            Ok(controller) => controller,
            Err(e) => {
                eprintln!("{e}");
                return 1;
            }
        };
        let server = NuSh::new(controller.clone(), &DenySet::default());
        let host = LOCAL_HOST.to_string();
        let result = match tool {
            CliTool::Call {
                namepath,
                data,
                timeout,
            } => {
                server
                    .call(mcp::Parameters(CallParams {
                        host,
                        namepath,
                        data,
                        timeout,
                    }))
                    .await
            }
            CliTool::Execute {
                definition,
                data,
                timeout,
            } => {
                server
                    .execute(mcp::Parameters(ExecuteParams {
                        host,
                        definition,
                        data,
                        timeout,
                    }))
                    .await
            }
            CliTool::Rerun {
                nonce,
                data,
                timeout,
            } => {
                server
                    .rerun(mcp::Parameters(RerunParams {
                        host,
                        nonce,
                        data,
                        timeout,
                    }))
                    .await
            }
            CliTool::Interface { command } => {
                server
                    .interface(mcp::Parameters(InterfaceParams { host, command }))
                    .await
            }
        };
        let code = match result {
            Ok(r) => match r.content.first() {
                Some(mcp::ContentBlock::Text(text)) => {
                    println!("{}", text.text);
                    if envelope_failed(&text.text) { 1 } else { 0 }
                }
                _ => {
                    eprintln!("grammar: the result carries no envelope text");
                    1
                }
            },
            Err(e) => {
                eprintln!("{e}");
                1
            }
        };
        drop(server);
        controller.shutdown().await;
        code
    }
    .await;
    process::ExitCode::from(exit_code as u8)
}

/// Whether an envelope's NUON text is the failure shape, holding `error`.
fn envelope_failed(text: &str) -> bool {
    nu::from_nuon(text, None)
        .ok()
        .and_then(|value| value.as_record().ok().map(|record| record.contains("error")))
        .unwrap_or(true)
}
