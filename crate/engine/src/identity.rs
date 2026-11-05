//! An engine instance's identity: its nonce generator and the nom drawn from it.
use crate::*;

/// An engine instance's identity, drawn at its start, its controller inheriting it.
#[derive(Clone)]
pub struct EngineIdentity {
    nonce_gen: Arc<datum::NonceGenerator>,
    nom: datum::NomPair,
}

impl fmt::Debug for EngineIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EngineIdentity").field("nom", &self.nom.as_str()).finish_non_exhaustive()
    }
}

impl EngineIdentity {
    /// A fresh identity: its generator, and the nom drawn first from it.
    pub fn draw() -> Self {
        let nonce_gen = Arc::new(datum::NonceGenerator::new());
        let nom = datum::Nom::generate(&nonce_gen).into_pair();
        Self { nonce_gen, nom }
    }

    /// The instance's nom, its evals' `$env.EQUIP_MCP_NOM` and its host's nom.
    pub fn nom(&self) -> &datum::NomPair {
        &self.nom
    }

    /// The generator its nom came from, which mints its controller's nonces.
    pub fn nonce_gen(&self) -> &Arc<datum::NonceGenerator> {
        &self.nonce_gen
    }
}
