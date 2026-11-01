use crate::*;

/// The server: MCP over stdio on the in-process engine, the namespace's lock held.
pub(crate) async fn run_server(
    config: Arc<Config>,
    lock: NamespaceLock,
) -> Result<(), String> {
    install_child_subreaper();
    ensure_cert_profile();
    ensure_substrate()
        .await
        .map_err(|e| format!("the equip substrate: {e}"))?;
    let controller = McpController::start(config.clone(), true, Some(lock)).await?;
    controller.sweep_signals();
    let server = NuSh::new(controller.clone(), &config.deny);
    let service = server
        .serve(mcp::stdio())
        .await
        .map_err(|e| format!("serve stdio: {e}"))?;
    let served = service.waiting().await;
    controller.shutdown().await;
    served
        .map(|_| ())
        .map_err(|e| format!("the stdio service: {e}"))
}
