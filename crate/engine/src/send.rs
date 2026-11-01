//! A body's remote send prepared by its host, its event rendered and files read.
use crate::*;

/// A body's remote send as its host sends it on, its event's NUON and files read.
pub fn prepare_send(data: RemoteSendData) -> Result<PreparedSend, String> {
    let RemoteSendData {
        mcp_nom,
        model,
        event,
        attached,
    } = data;
    let span = nu::Span::unknown();
    let files = match attached {
        Some(table) => read_files(&table.try_into_nu(span).map_err(|e| e.to_string())?)?,
        None => Vec::new(),
    };
    let event = event.try_into_nu(span).map_err(|e| e.to_string())?;
    Ok(PreparedSend {
        mcp_nom,
        model,
        event_nuon: render_nuon(&event)?,
        files,
    })
}

/// Each `{src, dest}` row read into its file, its dest checked first.
fn read_files(value: &nu::Value) -> Result<Vec<SendFile>, String> {
    let rows = match value {
        nu::Value::List { vals, .. } => vals,
        _ => return Err("attached must be a table<src: path, dest: path>".to_string()),
    };
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let rec = match row {
            nu::Value::Record { val, .. } => val,
            _ => return Err("each attached row must be a record with `src` and `dest`".to_string()),
        };
        let src = rec
            .get("src")
            .and_then(|v| v.as_str().ok())
            .ok_or_else(|| "each attached row needs a `src` path".to_string())?;
        let dest = rec
            .get("dest")
            .and_then(|v| v.as_str().ok())
            .ok_or_else(|| "each attached row needs a `dest` name".to_string())?;
        if !safe_dest(dest) {
            return Err(format!("dest `{dest}` must be a relative path with no `..`"));
        }
        let bytes = fs::read(src).map_err(|e| format!("read src `{src}`: {e}"))?;
        out.push(SendFile {
            dest: dest.to_string(),
            bytes,
        });
    }
    Ok(out)
}
