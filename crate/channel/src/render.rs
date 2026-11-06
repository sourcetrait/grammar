use crate::*;

/// The envelope's field holding the packet's event, or its spilled size.
const DATA: &str = "data";

/// The envelope's field holding a packet's attachment, or its spilled size.
const ATTACHED: &str = "attached";

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
    data_nuon: &str,
    attached_nuon: Option<&str>,
) -> MsgId {
    MsgId(nonce_gen.generate_with(&(from, model, data_nuon, attached_nuon)))
}

/// Render a value as compact NUON - what the wire and the hash both see.
pub fn render_nuon(value: &nu::Value) -> Result<String, String> {
    nu::to_nuon(&nu::EngineState::new(), value, nu::ToNuonConfig::default())
        .map_err(|e| e.to_string())
}

/// What a packet attaches, as its line first carries it.
#[derive(Debug, Clone, Copy)]
pub enum Attached<'a> {
    /// A relay's files in `<id>/`, and the size of `<id>.attached.nuon`, their table.
    Files {
        files: &'a [AttachedFile],
        bytes: u64,
    },
    /// A local send's value, by the size of `<id>.attached.nuon`.
    Value { bytes: u64 },
}

impl Attached<'_> {
    /// The value a line carries before any spill: the table, or the file's size.
    fn inline(self) -> nu::Value {
        match self {
            Self::Files { files, .. } => attached_table(files),
            Self::Value { bytes } => spill_size(bytes),
        }
    }

    /// The value a line carries once `attached` spilled: the file's size.
    fn spilled(self) -> nu::Value {
        match self {
            Self::Files { bytes, .. } | Self::Value { bytes } => spill_size(bytes),
        }
    }
}

/// A packet rendered: its line, and `<id>.data.nuon`'s text where `data` spilled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedPacket {
    /// The packet on one line, held to the cap.
    pub line: String,
    /// The spilled `data`, written as `<id>.data.nuon`.
    pub data: Option<String>,
}

/// Render one packet's line, spilling `attached`, then `data`, past the cap.
pub fn render_packet(
    id: MsgId,
    from: &str,
    model: &str,
    data: &nu::Value,
    attached: Option<Attached<'_>>,
) -> Result<RenderedPacket, String> {
    let id = id.to_string();
    let line = |data: nu::Value, attached: Option<nu::Value>| {
        render_nuon(&envelope(&id, from, model, data, attached)).map(|nuon| escape_line(&nuon))
    };
    let whole = line(data.clone(), attached.map(Attached::inline))?;
    if fits(&whole) {
        return Ok(RenderedPacket { line: whole, data: None });
    }
    let spilled = attached.map(Attached::spilled);
    if matches!(attached, Some(Attached::Files { .. })) {
        let lighter = line(data.clone(), spilled.clone())?;
        if fits(&lighter) {
            return Ok(RenderedPacket { line: lighter, data: None });
        }
    }
    let text = render_nuon(data)?;
    let bare = line(spill_size(text.len() as u64), spilled)?;
    if !fits(&bare) {
        return Err(format!(
            "a packet's envelope passes the {NOTIFICATION_CAP}-character cap with its data spilled",
        ));
    }
    Ok(RenderedPacket {
        line: bare,
        data: Some(text),
    })
}

/// Whether a line fits the client's notification cap.
fn fits(line: &str) -> bool {
    line.chars().count() <= NOTIFICATION_CAP
}

/// The most characters a minted id takes on a line: eleven base62 digits, quoted.
const ID_LINE_MAX: usize = 13;

/// Whether `data` stays on a packet's line, its id not yet minted.
pub fn line_fits(
    from: &str,
    model: &str,
    data: &nu::Value,
    attached: Option<Attached<'_>>,
) -> Result<bool, String> {
    let value = envelope("", from, model, data.clone(), attached.map(Attached::spilled));
    let line = escape_line(&render_nuon(&value)?);
    Ok(line.chars().count() - 2 + ID_LINE_MAX <= NOTIFICATION_CAP)
}

/// A packet as one record, its envelope, `attached` where it attaches anything.
fn envelope(
    id: &str,
    from: &str,
    model: &str,
    data: nu::Value,
    attached: Option<nu::Value>,
) -> nu::Value {
    let span = nu::Span::unknown();
    let mut record = nu::Record::new();
    record.insert("id", nu::Value::string(id.to_string(), span));
    record.insert("from", nu::Value::string(from.to_string(), span));
    record.insert("model", nu::Value::string(model.to_string(), span));
    record.insert(DATA, data);
    if let Some(attached) = attached {
        record.insert(ATTACHED, attached);
    }
    nu::Value::record(record, span)
}

/// The table a relay's `attached` is: each file in `<id>/` by its path and size.
pub fn attached_table(files: &[AttachedFile]) -> nu::Value {
    let span = nu::Span::unknown();
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
    nu::Value::list(rows, span)
}

/// A spilled field's value: the size of the file it went to.
pub fn spill_size(bytes: u64) -> nu::Value {
    let size = i64::try_from(bytes).unwrap_or(i64::MAX);
    nu::Value::filesize(size, nu::Span::unknown())
}

/// Re-escape raw newlines so one packet is always one line.
pub fn escape_line(rendered: &str) -> String {
    rendered.replace('\n', "\\n").replace('\r', "\\r")
}

/// The name a packet's spilled `data` takes in the Channel's directory.
pub fn data_file(id: impl Display) -> String {
    format!("{id}.data.nuon")
}

/// The name a packet's attachment takes in the Channel's directory.
pub fn attached_file(id: impl Display) -> String {
    format!("{id}.attached.nuon")
}

/// A NuVal back as the nushell value it carries, or why it can't be.
pub(crate) fn nu_value(value: NuVal) -> Result<nu::Value, String> {
    value.try_into_nu(nu::Span::unknown()).map_err(|e| e.to_string())
}
