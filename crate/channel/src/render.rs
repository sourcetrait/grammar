use crate::*;

/// A channel message's id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MsgId(datum::Nonce);

impl Display for MsgId {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Mint a packet's id over the packet's own fields.
pub fn mint_msg_id(
    nonce_gen: &datum::NonceGenerator,
    from: &str,
    model: &str,
    event_nuon: &str,
    attached_nuon: Option<&str>,
) -> MsgId {
    MsgId(nonce_gen.generate_with(&(from, model, event_nuon, attached_nuon)))
}

/// Render a value as compact NUON - what the wire and the hash both see.
pub fn render_nuon(value: &nu::Value) -> Result<String, String> {
    nu::to_nuon(&nu::EngineState::new(), value, nu::ToNuonConfig::default())
        .map_err(|e| e.to_string())
}

/// Render one packet as the single NUON line the wire carries.
///
/// `attached` names the files in the packet's `<id>/`, none where it has none.
pub fn render_packet(
    id: MsgId,
    from: &str,
    model: &str,
    event: &nu::Value,
    attached: Option<&[String]>,
) -> Result<String, String> {
    let span = nu::Span::unknown();
    let mut record = nu::Record::new();
    record.insert("id", nu::Value::string(id.to_string(), span));
    record.insert("from", nu::Value::string(from.to_string(), span));
    record.insert("model", nu::Value::string(model.to_string(), span));
    record.insert("event", event.clone());
    if let Some(names) = attached.filter(|names| !names.is_empty()) {
        let names = names.iter().map(|name| nu::Value::string(name.clone(), span)).collect();
        record.insert("attached", nu::Value::list(names, span));
    }
    let rendered = render_nuon(&nu::Value::record(record, span))?;
    Ok(escape_line(&rendered))
}

/// Re-escape raw newlines so one packet is always one line.
pub fn escape_line(rendered: &str) -> String {
    rendered.replace('\n', "\\n").replace('\r', "\\r")
}

/// What the packet's wrapper takes of the cap around its event.
const NOTIFICATION_WRAPPER_RESERVE: usize = 120;

/// Would this rendered event NUON overflow the notification cap once wrapped?
pub fn event_overflows(event_nuon: &str) -> bool {
    event_nuon.chars().count() > NOTIFICATION_CAP - NOTIFICATION_WRAPPER_RESERVE
}

/// The name a local send's attachment takes in its packet's `<id>/`.
pub const ATTACHED_FILE: &str = "attached.nuon";

/// The name a packet's event takes in the Channel's directory, `<id>.nuon`.
pub fn event_file(id: impl Display) -> String {
    format!("{id}.nuon")
}

/// The event replacing an oversized one on the wire: its own file's size.
pub fn event_spill_pointer(event_bytes: usize) -> nu::Value {
    let span = nu::Span::unknown();
    let mut r = nu::Record::new();
    r.insert(EVENT_SIZE_KEY, nu::Value::filesize(event_bytes as i64, span));
    nu::Value::record(r, span)
}

/// A NuVal back as the nushell value it carries, or why it can't be.
pub(crate) fn nu_value(value: NuVal) -> Result<nu::Value, String> {
    value.try_into_nu(nu::Span::unknown()).map_err(|e| e.to_string())
}
