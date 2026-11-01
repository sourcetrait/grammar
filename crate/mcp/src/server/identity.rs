//! The host process's identity: its nonce generator and the nom drawn from it.
use crate::*;

/// This process's nonce generator and the nom drawn from it, minted once.
static PROCESS_IDENTITY: OnceLock<(Arc<datum::NonceGenerator>, datum::NomPair)> = OnceLock::new();

/// The process's nonce generator and its nom, the host's id for its life.
pub(crate) fn process_identity() -> (Arc<datum::NonceGenerator>, datum::NomPair) {
    let (nonce_gen, mcp_nom) = PROCESS_IDENTITY.get_or_init(|| {
        let nonce_gen = Arc::new(datum::NonceGenerator::new());
        let mcp_nom = datum::Nom::generate(&nonce_gen).into_pair();
        (nonce_gen, mcp_nom)
    });
    (nonce_gen.clone(), mcp_nom.clone())
}

/// A call's nonce, minted first from its tool and its parameters' raw NUON.
pub(crate) fn mint_nonce<P: ser::Serialize>(
    nonce_gen: &datum::NonceGenerator,
    tool: &str,
    params: &P,
) -> datum::NoncePair {
    let payload = nuval::to_nuon(params, NuonStyle::Raw).unwrap_or_default();
    nonce_gen.generate_with(&(tool, payload)).into_pair()
}
