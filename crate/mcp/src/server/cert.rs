use crate::*;

/// The grammar cert profile shipped for the operator to run srcert against.
const GRAMMAR_PROFILE: &str = include_str!("../../../../include/config/grammar_profile.toml");

/// Ship the grammar cert profile when absent; ship-and-continue, never fatal.
pub(crate) fn ensure_cert_profile() {
    let config_home = match lib_cert::config_home() {
        Ok(home) => home,
        Err(e) => {
            eprintln!("grammar: cert profile not shipped: {e}");
            return;
        }
    };
    match lib_cert::install_profile(&config_home, consts::GRAMMAR, GRAMMAR_PROFILE) {
        Ok(done) if done.written => eprintln!(
            "grammar: shipped the `{name}` cert profile to {path}; run \
             `srcert generate all {name} <dir>`, then name its entity certificate and key \
             as [identity] `cert` and `key` in {AGENT_CONFIG_FILE} to enable \
             the channel",
            name = consts::GRAMMAR,
            path = done.live.display(),
        ),
        Ok(_) => {}
        Err(e) => eprintln!("grammar: cert profile not shipped: {e}"),
    }
}
