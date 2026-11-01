use crate::*;

/// `new` summary: the paths scaffolded for the fresh equip's rig.
#[derive(Debug, ser::Serialize)]
pub(crate) struct NewSummary {
    pub created: Vec<String>,
}

/// The per-action summary, `install`'s its commit's; `uninstall` returns none.
#[derive(Debug, ser::Serialize)]
#[serde(untagged)]
pub(crate) enum EquipSummary {
    New(NewSummary),
    Install(CommitResult),
    Check(CheckSummary),
}

/// Success result of each `interface equip` action on a source.
#[derive(Debug, ser::Serialize)]
pub(crate) struct EquipResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<EquipSummary>,
}

impl NuSh {
    /// `interface equip new`: register an equip at its source and seed its rig.
    pub(crate) async fn equip_new(
        &self,
        nonce: datum::Nonce,
        route: &Route,
        p: EquipParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.control_on(route, EngineCommand::EquipNew { source: p.source }).await {
            Ok(ControlAnswer::Created(created)) => success(
                nonce,
                &EquipResult {
                    summary: Some(EquipSummary::New(NewSummary { created })),
                },
            ),
            Ok(_) => error_to_call_result(unexpected("equip_new"), nonce),
            Err(e) => error_to_call_result(e, nonce),
        }
    }

    /// `interface equip install`: validate an equip's whole source and install it.
    pub(crate) async fn equip_install(
        &self,
        nonce: datum::Nonce,
        route: &Route,
        p: EquipParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.control_on(route, EngineCommand::EquipInstall { source: p.source }).await {
            Ok(ControlAnswer::Installed { equip, result }) => {
                self.purview_rig(route, equip, true).await;
                success(
                    nonce,
                    &EquipResult {
                        summary: Some(EquipSummary::Install(result)),
                    },
                )
            }
            Ok(_) => error_to_call_result(unexpected("equip_install"), nonce),
            Err(e) => error_to_call_result(e, nonce),
        }
    }

    /// `interface equip check`: validate an installed equip's source, writing nothing.
    pub(crate) async fn equip_check(
        &self,
        nonce: datum::Nonce,
        route: &Route,
        p: EquipParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.control_on(route, EngineCommand::EquipCheck { source: p.source }).await {
            Ok(ControlAnswer::Checked(summary)) => success(
                nonce,
                &EquipResult {
                    summary: Some(EquipSummary::Check(summary)),
                },
            ),
            Ok(_) => error_to_call_result(unexpected("equip_check"), nonce),
            Err(e) => error_to_call_result(e, nonce),
        }
    }

    /// `interface equip uninstall`: remove an installed equip; its source stays.
    pub(crate) async fn equip_uninstall(
        &self,
        nonce: datum::Nonce,
        route: &Route,
        p: EquipParams,
    ) -> Result<mcp::CallToolResult, mcp::ErrorData> {
        match self.control_on(route, EngineCommand::EquipUninstall { source: p.source }).await {
            Ok(ControlAnswer::Uninstalled { equip }) => {
                if let Some(equip) = equip {
                    self.purview_rig(route, equip, false).await;
                }
                success(nonce, &EquipResult { summary: None })
            }
            Ok(_) => error_to_call_result(unexpected("equip_uninstall"), nonce),
            Err(e) => error_to_call_result(e, nonce),
        }
    }
}

impl NuSh {
    /// A rig installed or removed: the routed engine's purviews adjusted, view kept.
    async fn purview_rig(
        &self,
        route: &Route,
        rig: String,
        installed: bool,
    ) {
        let current = self.views.of(route);
        let adjust = PurviewRequest::Rig(PurviewRigRequest {
            rig: rig.clone(),
            installed,
            current: current.ids(),
        });
        match self.purview_rows_on(route, adjust).await {
            Ok(rows) if !installed => current.retain_known(&rows),
            Ok(_) => {}
            Err(e) => eprintln!("grammar: purview not updated for {rig}: {e:?}"),
        }
    }
}
