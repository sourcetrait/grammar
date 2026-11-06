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

/// A packet's two forms: its file, the packet whole, and the line the wire carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedPacket {
    /// The whole packet's NUON, written as `<id>.nuon`.
    pub file: String,
    /// The packet on one line, or its spill where that line passes the cap.
    pub line: String,
}

/// Render one packet whole, and the line the wire carries for it.
///
/// A line past `NOTIFICATION_CAP` spills whole: its event goes as `{size}`.
pub fn render_packet(
    id: MsgId,
    from: &str,
    model: &str,
    event: &nu::Value,
    attached: Option<&[AttachedFile]>,
) -> Result<RenderedPacket, String> {
    let file = render_nuon(&packet_value(id, from, model, event.clone(), attached))?;
    let whole = escape_line(&file);
    let line = if whole.chars().count() > NOTIFICATION_CAP {
        let spilled = packet_value(id, from, model, event_spill_pointer(file.len()), None);
        escape_line(&render_nuon(&spilled)?)
    } else {
        whole
    };
    Ok(RenderedPacket { file, line })
}

/// A packet as one record, `attached` a table of each file's path and size.
fn packet_value(
    id: MsgId,
    from: &str,
    model: &str,
    event: nu::Value,
    attached: Option<&[AttachedFile]>,
) -> nu::Value {
    let span = nu::Span::unknown();
    let mut record = nu::Record::new();
    record.insert("id", nu::Value::string(id.to_string(), span));
    record.insert("from", nu::Value::string(from.to_string(), span));
    record.insert("model", nu::Value::string(model.to_string(), span));
    record.insert("event", event);
    if let Some(files) = attached.filter(|files| !files.is_empty()) {
        let rows = files
            .iter()
            .map(|attached| {
                let mut row = nu::Record::new();
                row.insert("file", nu::Value::string(attached.file.clone(), span));
                let size = i64::try_from(attached.size).unwrap_or(i64::MAX);
                row.insert("size", nu::Value::filesize(size, span));
                nu::Value::record(row, span)
            })
            .collect();
        record.insert("attached", nu::Value::list(rows, span));
    }
    nu::Value::record(record, span)
}

/// Re-escape raw newlines so one packet is always one line.
pub fn escape_line(rendered: &str) -> String {
    rendered.replace('\n', "\\n").replace('\r', "\\r")
}

/// What a packet's wrapper takes of the cap around its event, as a sender guesses.
const NOTIFICATION_WRAPPER_RESERVE: usize = 120;

/// Whether a remote sender carries its event as a file rather than inline.
pub fn event_overflows(event_nuon: &str) -> bool {
    event_nuon.chars().count() > NOTIFICATION_CAP - NOTIFICATION_WRAPPER_RESERVE
}

/// The name a local send's attachment takes in its packet's `<id>/`.
pub const ATTACHED_FILE: &str = "attached.nuon";

/// The name a packet takes whole in the Channel's directory, `<id>.nuon`.
pub fn event_file(id: impl Display) -> String {
    format!("{id}.nuon")
}

/// The event of a spilled line: the size of the packet's own file.
pub fn event_spill_pointer(file_bytes: usize) -> nu::Value {
    let span = nu::Span::unknown();
    let mut r = nu::Record::new();
    let size = i64::try_from(file_bytes).unwrap_or(i64::MAX);
    r.insert(EVENT_SIZE_KEY, nu::Value::filesize(size, span));
    nu::Value::record(r, span)
}

/// A NuVal back as the nushell value it carries, or why it can't be.
pub(crate) fn nu_value(value: NuVal) -> Result<nu::Value, String> {
    value.try_into_nu(nu::Span::unknown()).map_err(|e| e.to_string())
}
