//! Each host's current purview, the ids in view, resolved by that host's engine.
use crate::*;

/// What changed between two pattern sets, as `(added, removed)`.
pub(crate) fn pattern_delta(
    before: &[String],
    after: &[String],
) -> (Vec<String>, Vec<String>) {
    let added: Vec<String> = after
        .iter()
        .filter(|p| !before.contains(p))
        .cloned()
        .collect();
    let removed: Vec<String> = before
        .iter()
        .filter(|p| !after.contains(p))
        .cloned()
        .collect();
    (added, removed)
}

/// The current purview: which ids this host process has in view on one engine.
pub(crate) struct CurrentPurview {
    ids: std::sync::Mutex<Vec<String>>,
}

impl Default for CurrentPurview {
    fn default() -> Self {
        Self::new()
    }
}

impl CurrentPurview {
    /// A fresh view, `default` alone.
    pub(crate) fn new() -> Self {
        Self {
            ids: std::sync::Mutex::new(vec![PURVIEW_DEFAULT.to_string()]),
        }
    }

    /// The view's lock, a poisoned one taken as it stands.
    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<String>> {
        self.ids.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The ids in view.
    pub(crate) fn ids(&self) -> Vec<String> {
        self.lock().clone()
    }

    /// Add ids to the current view; additive and order-preserving.
    pub(crate) fn extend(
        &self,
        add: &[String],
    ) -> Vec<String> {
        let mut ids = self.lock();
        for id in add {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        ids.clone()
    }

    /// Replace what is in view; an empty list means `default`.
    pub(crate) fn set(
        &self,
        want: &[String],
    ) -> Vec<String> {
        let mut ids = self.lock();
        *ids = if want.is_empty() {
            vec![PURVIEW_DEFAULT.to_string()]
        } else {
            want.to_vec()
        };
        ids.clone()
    }

    /// Drop ids the rows no longer hold.
    pub(crate) fn retain_known(
        &self,
        rows: &[PurviewRow],
    ) {
        let mut ids = self.lock();
        ids.retain(|id| id == PURVIEW_ALL || rows.iter().any(|row| &row.id == id));
        if ids.is_empty() {
            *ids = vec![PURVIEW_DEFAULT.to_string()];
        }
    }
}

/// Each engine's current view: the MCP's own, a remote engine's from its first use.
#[derive(Default)]
pub(crate) struct Views {
    local: Arc<CurrentPurview>,
    remote: std::sync::Mutex<HashMap<String, Arc<CurrentPurview>>>,
}

impl Views {
    /// The current view on the engine `route` names.
    pub(crate) fn of(
        &self,
        route: &Route,
    ) -> Arc<CurrentPurview> {
        match route {
            Route::Local => self.local.clone(),
            Route::Remote(alias) => self
                .remote
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .entry(alias.clone())
                .or_default()
                .clone(),
        }
    }
}

/// The refusal of a view naming an id no purview answers to.
pub(crate) fn unknown_purview(id: &str) -> GrammarMcpError {
    GrammarMcpError::PurviewInvalidId {
        id: id.to_string(),
        reason: "no such purview; configure it first".to_string(),
    }
}

impl NuSh {
    /// A purview request's answer from the engine `route` names.
    pub(crate) async fn purview_on(
        &self,
        route: &Route,
        request: PurviewRequest,
    ) -> GrammarMcpResult<PurviewAnswer> {
        match route {
            Route::Local => Ok(self.controller.request(request).await?),
            Route::Remote(alias) => match self.controller.remote(alias, EngineRequest::Purview(request)).await? {
                EngineResponse::Purview(answer) => Ok(answer),
                EngineResponse::Refused(refusal) => Err(GrammarMcpError::Transport(refusal)),
                _ => Err(unexpected("purview")),
            },
        }
    }

    /// What a set of ids puts in view, as the engine `route` names resolves it.
    pub(crate) async fn resolve_purviews(
        &self,
        route: &Route,
        ids: Vec<String>,
    ) -> GrammarMcpResult<PurviewResolution> {
        match self.purview_on(route, PurviewRequest::Resolve(PurviewResolveRequest { ids })).await? {
            PurviewAnswer::Resolved(resolution) => Ok(resolution?),
            _ => Err(unexpected("purview resolve")),
        }
    }

    /// Every purview the engine `route` names holds.
    pub(crate) async fn purviews_on(
        &self,
        route: &Route,
    ) -> GrammarMcpResult<Vec<PurviewView>> {
        match self.purview_on(route, PurviewRequest::List(PurviewListRequest)).await? {
            PurviewAnswer::Listed(purviews) => Ok(purviews?),
            _ => Err(unexpected("purview list")),
        }
    }

    /// The purview rows a configure or a rig's change leaves on the routed engine.
    pub(crate) async fn purview_rows_on(
        &self,
        route: &Route,
        request: PurviewRequest,
    ) -> GrammarMcpResult<Vec<PurviewRow>> {
        match self.purview_on(route, request).await? {
            PurviewAnswer::Rows(rows) => Ok(rows?),
            _ => Err(unexpected("purview rows")),
        }
    }
}
