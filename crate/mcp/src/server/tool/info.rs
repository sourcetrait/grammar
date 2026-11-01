use crate::*;

/// Success result of `interface info`.
#[derive(Debug, ser::Serialize)]
pub(crate) struct InfoResult {
    pub name: String,
    pub version: String,
    pub nu_version: String,
    /// The `--id` the engine's host was configured with.
    pub id: String,
    pub namespace: String,
    /// The engine's host process's nom, stable for its life: $env.EQUIP_MCP_NOM.
    pub nom: String,
    /// The engine's working directory, exported as $env.EQUIP_WORK_DIR.
    pub work_dir: String,
    /// The engine's bulk scratch directory, exported as $env.EQUIP_TMP.
    pub tmp: String,
    /// The engine's transient scratch directory, exported as $env.EQUIP_TRANSIENT.
    pub transient: String,
    pub plugins: Vec<PluginInfo>,
    /// The callable surface within the reported purview, as one block.
    pub signatures: String,
    /// The purview `signatures` was rendered for.
    pub purview: Vec<PurviewView>,
}

impl NuSh {
    /// `interface info`: the routed engine's versions, plugins, and signatures.
    pub(crate) async fn info(
        &self,
        nonce: datum::Nonce,
        route: &Route,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.info_result(route).await {
            Ok(result) => success(nonce, &result),
            Err(error) => error_to_call_result(error, nonce),
        }
    }

    /// The info of the engine `route` names, its signatures within its current view.
    async fn info_result(
        &self,
        route: &Route,
    ) -> GrammarMcpResult<InfoResult> {
        let view = self.resolve_purviews(route, self.views.of(route).ids()).await?;
        let signatures = self.signatures(route, view.patterns).await?;
        let result = match route {
            Route::Local => InfoResult {
                name: consts::GRAMMAR.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                nu_version: NU_VERSION.to_string(),
                id: self.controller.config.id.clone(),
                namespace: self.controller.config.namespace.clone(),
                nom: self.mcp_nom.as_str().to_string(),
                work_dir: self.controller.config.work_dir.display().to_string(),
                tmp: tmp_dir().map(|dir| dir.display().to_string()).unwrap_or_default(),
                transient: transient_dir().map(|dir| dir.display().to_string()).unwrap_or_default(),
                plugins: list_registered_plugins(),
                signatures,
                purview: view.views,
            },
            Route::Remote(alias) => {
                let worker = self.worker_info(alias).await?;
                InfoResult {
                    name: consts::GRAMMAR.to_string(),
                    version: worker.version,
                    nu_version: worker.nu_version,
                    id: worker.id,
                    namespace: worker.namespace,
                    nom: worker.nom,
                    work_dir: worker.work_dir,
                    tmp: worker.tmp,
                    transient: worker.transient,
                    plugins: worker.plugins,
                    signatures,
                    purview: view.views,
                }
            }
        };
        Ok(result)
    }
}
