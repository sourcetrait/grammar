use crate::*;

/// The `[supervisor]` lines: when a resource level is worth one warning.
#[cereal::derived(Data, Copy)]
pub struct SupervisorConfig {
    /// Fraction of ALL cores the host's own CPU must reach.
    pub cpu_warn_fraction: f64,
    /// Fraction of total system RAM the host's own RSS must reach.
    pub ram_warn_fraction: f64,
    /// Warn once GPU memory FREE falls below this many MiB.
    pub vram_warn_headroom_mib: u64,
    /// Fraction of a watched filesystem's capacity.
    pub disk_warn_fraction: f64,
}

/// A fraction of capacity held to (0, 1]; outside it, it warns always or never.
pub fn check_fraction(
    name: &str,
    value: f64,
) -> Result<f64, String> {
    if value > 0.0 && value <= 1.0 {
        Ok(value)
    } else {
        Err(format!("supervisor.{name} must be greater than 0 and at most 1"))
    }
}
