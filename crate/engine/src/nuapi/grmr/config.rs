//! The body-facing config surface: read what is in force, list it, pin a setting.

use crate::*;

/// The dotted key separator.
const KEY_SEP: char = '.';

/// The configuration in force, as the host answers it.
fn config_record(
    host: &HostLink,
    command: &str,
    span: nu::Span,
) -> Result<nu::Value, nu::ShellError> {
    let failed = |reason: String| -> nu::ShellError {
        nu::GenericError::new(format!("{command} failed"), reason, span).into()
    };
    match host.ask(HostAsk::Config).map_err(failed)? {
        HostAnswer::Config(record) => record
            .try_into_nu(span)
            .map_err(|e| failed(format!("the config record: {e}"))),
        HostAnswer::Refused(reason) => Err(failed(reason)),
        other => Err(failed(unexpected(&other))),
    }
}

/// Every dotted key the record holds, in order.
fn config_keys(root: &nu::Value) -> Vec<String> {
    let mut keys = Vec::new();
    let Ok(tables) = root.as_record() else {
        return keys;
    };
    for (table, value) in tables {
        match value.as_record() {
            Ok(fields) => {
                for (field, _) in fields {
                    keys.push(format!("{table}{KEY_SEP}{field}"));
                }
            }
            Err(_) => keys.push(table.clone()),
        }
    }
    keys
}

/// Walk the record by a dotted key.
fn lookup(
    root: &nu::Value,
    key: &str,
) -> Option<nu::Value> {
    let mut current = root.clone();
    for segment in key.split(KEY_SEP) {
        let next = current.as_record().ok()?.get(segment)?.clone();
        current = next;
    }
    Some(current)
}

/// The unknown-key error, its label listing the valid keys.
fn unknown_key(
    root: &nu::Value,
    key: &str,
    span: nu::Span,
) -> nu::ShellError {
    let keys = config_keys(root).join(", ");
    nu::GenericError::new(
        format!("grmr config get: `{key}` is not a config key"),
        format!("valid keys: {keys}"),
        span,
    )
    .into()
}

/// The value type a setting can hold, as a closed union.
fn value_type() -> nu::Type {
    nu::Type::one_of([
        nu::Type::Int,
        nu::Type::Float,
        nu::Type::String,
        nu::Type::Nothing,
    ])
}

/// `config get`'s answer: each key asked beside its value.
fn settings_type() -> nu::Type {
    nu::Type::Table(nu::CollectionColumns::from(vec![
        ("key".to_string(), nu::Type::String),
        ("value".to_string(), value_type()),
    ]))
}

/// `grmr config get <keys>` - settings by their dotted keys, each beside its value.
#[derive(Clone)]
pub(crate) struct GrmrConfigGet {
    pub(crate) host: Option<HostLink>,
}

impl nu::Command for GrmrConfigGet {
    fn name(&self) -> &str {
        "grmr config get"
    }

    fn signature(&self) -> nu::Signature {
        nu::Signature::build("grmr config get")
            .required(
                "keys",
                nu::SyntaxShape::List(Box::new(nu::SyntaxShape::String)),
                "the dotted keys, e.g. [supervisor.vram_warn_headroom_mib]",
            )
            .input_output_types(vec![(nu::Type::Nothing, settings_type())])
            .category(nu::Category::Custom("grmr".to_string()))
    }

    fn description(&self) -> &str {
        "Configuration settings by their dotted keys; errors on an unknown key."
    }

    fn run(
        &self,
        engine_state: &nu::EngineState,
        stack: &mut nu::Stack,
        call: &nu::Call<'_>,
        _input: nu::PipelineData,
    ) -> Result<nu::PipelineData, nu::ShellError> {
        let keys: Vec<String> = call.req(engine_state, stack, 0)?;
        let name = nu::Command::name(self);
        let host = asked_host(&self.host, name, call.head)?;
        let root = config_record(host, name, call.head)?;
        let rows = keys
            .into_iter()
            .map(|key| {
                let value = lookup(&root, &key).ok_or_else(|| unknown_key(&root, &key, call.head))?;
                let mut row = nu::Record::new();
                row.insert("key", nu::Value::string(key, call.head));
                row.insert("value", value);
                Ok(nu::Value::record(row, call.head))
            })
            .collect::<Result<Vec<_>, nu::ShellError>>()?;
        Ok(nu::PipelineData::Value(nu::Value::list(rows, call.head), None))
    }
}

/// `grmr config list` - every dotted key the config holds.
#[derive(Clone)]
pub(crate) struct GrmrConfigList {
    pub(crate) host: Option<HostLink>,
}

impl nu::Command for GrmrConfigList {
    fn name(&self) -> &str {
        "grmr config list"
    }

    fn signature(&self) -> nu::Signature {
        nu::Signature::build("grmr config list")
            .input_output_types(vec![(nu::Type::Nothing, nu::Type::List(Box::new(nu::Type::String)))])
            .category(nu::Category::Custom("grmr".to_string()))
    }

    fn description(&self) -> &str {
        "Every configuration key, dotted, in the config's order."
    }

    fn run(
        &self,
        _engine_state: &nu::EngineState,
        _stack: &mut nu::Stack,
        call: &nu::Call<'_>,
        _input: nu::PipelineData,
    ) -> Result<nu::PipelineData, nu::ShellError> {
        let name = nu::Command::name(self);
        let host = asked_host(&self.host, name, call.head)?;
        let root = config_record(host, name, call.head)?;
        let keys = config_keys(&root)
            .into_iter()
            .map(|key| nu::Value::string(key, call.head))
            .collect();
        Ok(nu::PipelineData::Value(nu::Value::list(keys, call.head), None))
    }
}

/// `grmr config pin` - hold a setting for as long as a process lives.
#[derive(Clone)]
pub(crate) struct GrmrConfigPin {
    pub(crate) host: Option<HostLink>,
}

impl nu::Command for GrmrConfigPin {
    fn name(&self) -> &str {
        "grmr config pin"
    }

    fn signature(&self) -> nu::Signature {
        nu::Signature::build("grmr config pin")
            .required(
                "key",
                nu::SyntaxShape::String,
                "the dotted key to pin; the supervisor warning lines only",
            )
            .required(
                "process_id",
                nu::SyntaxShape::Int,
                "the pid whose lifetime the pin follows",
            )
            .required("value", nu::SyntaxShape::Number, "the value to hold")
            .input_output_types(vec![(nu::Type::Nothing, nu::Type::Nothing)])
            .category(nu::Category::Custom("grmr".to_string()))
    }

    fn description(&self) -> &str {
        "Hold a supervisor setting at a value for the lifetime of a process."
    }

    fn run(
        &self,
        engine_state: &nu::EngineState,
        stack: &mut nu::Stack,
        call: &nu::Call<'_>,
        _input: nu::PipelineData,
    ) -> Result<nu::PipelineData, nu::ShellError> {
        let key: String = call.req(engine_state, stack, 0)?;
        let pid: i64 = call.req(engine_state, stack, 1)?;
        let raw: nu::Value = call.req(engine_state, stack, 2)?;
        let value = match &raw {
            nu::Value::Int { val, .. } => *val as f64,
            nu::Value::Float { val, .. } => *val,
            other => {
                return Err(nu::GenericError::new(
                    "grmr config pin: value must be a number",
                    format!("got {}", other.get_type()),
                    call.head,
                )
                .into());
            }
        };
        let pid = u32::try_from(pid).map_err(|_| {
            nu::GenericError::new(
                "grmr config pin: process_id must be a positive process id",
                format!("got {pid}"),
                call.head,
            )
        })?;
        let host = asked_host(&self.host, nu::Command::name(self), call.head)?;
        let failed = |reason: String| -> nu::ShellError {
            nu::GenericError::new("grmr config pin failed", reason, call.head).into()
        };
        match host.ask(HostAsk::Pin(PinAsk { key, pid, value })).map_err(failed)? {
            HostAnswer::Pinned => Ok(nu::PipelineData::Empty),
            HostAnswer::Refused(reason) => Err(failed(reason)),
            other => Err(failed(unexpected(&other))),
        }
    }
}
