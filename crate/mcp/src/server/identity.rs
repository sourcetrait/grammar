//! A call's nonce, minted from the generator the host inherits from its engine.
use crate::*;

/// A call's nonce, minted first from its tool and its parameters' raw NUON.
pub(crate) fn mint_nonce<P: ser::Serialize>(
    nonce_gen: &datum::NonceGenerator,
    tool: &str,
    params: &P,
) -> datum::NoncePair {
    let payload = nuval::to_nuon(params, NuonStyle::Raw).unwrap_or_default();
    nonce_gen.generate_with(&(tool, payload)).into_pair()
}
