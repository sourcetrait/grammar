//! The await suite's contract: one module, its four exports held to it.
use crate::*;

/// The export that prepares a job, within the call.
pub const PREPARE: &str = "prepare";

/// The export a job runs.
pub const EXECUTE_EXPORT: &str = EXECUTE;

/// The export each beat runs while the job does.
pub const STATUS: &str = "status";

/// The export an interruption runs before the job is killed.
pub const STOP: &str = "stop";

/// The suite's exports, in the order a job runs them.
pub const SUITE_EXPORTS: [&str; 4] = [PREPARE, EXECUTE_EXPORT, STATUS, STOP];

/// The name a suite is parsed and cached under, which positions its diagnostics.
pub const SUITE_FILE: &str = "suite.nu";

/// The module a suite is parsed inside.
const SUITE_WRAPPER: &str = "__suite";

/// The required parameter's name.
const DATA_PARAM: &str = "data";

/// prepare's first optional parameter, the call's timeouts.
const TIMEOUTS_PARAM: &str = "timeouts";

/// prepare's second optional parameter, the call's heartbeats.
const HEARTBEATS_PARAM: &str = "heartbeats";

/// The flag nushell gives every custom command.
const HELP_FLAG: &str = "help";

/// The kind every suite violation is reported under.
const KIND_SUITE: &str = "module::invalid";

/// The kind a module-resolution cycle is reported under, as the eval reports it.
const KIND_CIRCULAR_IMPORT: &str = "module::circular_import";

/// What a suite holding to its contract declares.
#[derive(Debug, Clone)]
pub struct SuiteDef {
    /// prepare's data, the call's.
    pub data_type: NuType,
    /// prepare's return type, the PREPARED record's `prepared`.
    pub prepared_type: NuType,
    /// Whether prepare takes `timeouts?`.
    pub takes_timeouts: bool,
    /// Whether prepare takes `heartbeats?`.
    pub takes_heartbeats: bool,
    pub execute: PhaseDef,
    pub status: PhaseDef,
    pub stop: PhaseDef,
}

/// A definition run on the PREPARED record: its return type, and its `timeout?`.
#[derive(Debug, Clone)]
pub struct PhaseDef {
    pub result_type: NuType,
    pub takes_timeout: bool,
}

/// The job's record: the id the jobs table holds it under, and its start.
pub fn job_type() -> NuType {
    NuType::Record(vec![
        NuField::new("id", NuType::Int),
        NuField::new("started", NuType::Date),
    ])
}

/// The PREPARED record the other three take: the job beside prepare's value.
pub fn prepared_record_type(prepared: &NuType) -> NuType {
    NuType::Record(vec![
        NuField::new("job", job_type()),
        NuField::new("prepared", prepared.clone()),
    ])
}

/// The type prepare's `timeouts?` declares: each definition's, null or a duration.
pub fn timeouts_type() -> NuType {
    NuType::Record(
        SUITE_EXPORTS
            .iter()
            .map(|name| NuField::new(*name, NuType::OneOf(vec![NuType::Nothing, NuType::Duration])))
            .collect(),
    )
}

/// The type prepare's `heartbeats?` declares.
pub fn heartbeats_type() -> NuType {
    NuType::List(Box::new(NuType::Duration))
}

/// prepare's signature held to its contract.
struct PrepareContract {
    data_type: NuType,
    result_type: NuType,
    takes_timeouts: bool,
    takes_heartbeats: bool,
}

/// Hold a suite module to the contract, and the whole module to the lint.
pub fn analyze_suite(
    parse_engine: &ParseEngine,
    module: &str,
) -> Result<SuiteDef, Vec<Diagnostic>> {
    let (wrapped, prefix_len) = wrap_as_module(module, SUITE_WRAPPER);
    let mut ws = nu::StateWorkingSet::new(parse_engine.engine_state());
    let files_before = ws.num_files();
    let block = nu::parse(&mut ws, Some(SUITE_FILE), wrapped.as_bytes(), false);
    let start = file_start(&ws, SUITE_FILE) + prefix_len;
    let violation = |span: Option<nu::Span>, message: String| -> Diagnostic {
        let position = match span {
            Some(span) => {
                let (line, col) = span_to_line_col(module, span.start.saturating_sub(start));
                [wide(line), wide(col)]
            }
            None => [0, 0],
        };
        Diagnostic::error(
            KIND_SUITE,
            Some(Source {
                path: None,
                position,
            }),
            message,
        )
    };
    if !ws.parse_errors.is_empty() {
        if let Some(cycle) = detect_import_cycle(&ws, files_before) {
            return Err(vec![Diagnostic::error(
                KIND_CIRCULAR_IMPORT,
                Some(Source {
                    path: None,
                    position: [0, 0],
                }),
                cycle,
            )]);
        }
        let mut rows: Vec<Diagnostic> = Vec::new();
        for error in &ws.parse_errors {
            let row = violation(
                Some(error.span()),
                format!("parse error: {}", nuval::NuValError::from(error).short()),
            );
            if !rows.iter().any(|seen| seen.message == row.message) {
                rows.push(row);
            }
        }
        rows.truncate(LINT_VIOLATION_CAP);
        return Err(rows);
    }
    let Some(module_id) = ws.find_module(SUITE_WRAPPER.as_bytes()) else {
        return Err(vec![violation(None, "the module registered nothing".to_string())]);
    };
    let suite = ws.get_module(module_id);
    let mut violations = Vec::new();
    let only = format!("a suite exports `{}` alone", SUITE_EXPORTS.join("`, `"));
    for name in suite.decls.keys() {
        let name = String::from_utf8_lossy(name);
        if !SUITE_EXPORTS.contains(&name.as_ref()) {
            violations.push(violation(None, format!("{only}; `{name}` stays unexported")));
        }
    }
    if suite.main.is_some() {
        violations.push(violation(None, format!("{only}; `export def main` is refused")));
    }
    if !suite.submodules.is_empty() {
        violations.push(violation(None, format!("{only}; it exports no module")));
    }
    if !suite.constants.is_empty() {
        violations.push(violation(None, format!("{only}; it exports no constant")));
    }
    if suite.env_block.is_some() {
        violations.push(violation(None, format!("{only}; it takes no `export-env` block")));
    }
    let mut decls = Vec::new();
    for name in SUITE_EXPORTS {
        match suite.decls.get(name.as_bytes()) {
            Some(id) => decls.push((name, *id)),
            None => violations.push(violation(None, format!("a suite exports `{name}`"))),
        }
    }
    if !violations.is_empty() {
        violations.truncate(LINT_VIOLATION_CAP);
        return Err(violations);
    }
    let at = |id: nu::DeclId| -> Option<nu::Span> {
        ws.get_decl(id).block_id().and_then(|block| ws.get_block(block).span)
    };
    for (name, id) in &decls {
        let env = ws
            .get_decl(*id)
            .block_id()
            .is_some_and(|block| ws.get_block(block).redirect_env);
        if env {
            violations.push(violation(
                at(*id),
                format!("`{name}` is a plain `export def`; `--env` is refused in a suite"),
            ));
        }
    }
    let signature = |name: &str| -> nu::Signature {
        let id = decls
            .iter()
            .find(|(export, _)| *export == name)
            .map(|(_, id)| *id)
            .expect("every export was found");
        ws.get_decl(id).signature()
    };
    let span_of = |name: &str| -> Option<nu::Span> {
        decls.iter().find(|(export, _)| *export == name).and_then(|(_, id)| at(*id))
    };
    let prepare = match prepare_contract(&signature(PREPARE)) {
        Ok(contract) => Some(contract),
        Err(reasons) => {
            violations.extend(reasons.into_iter().map(|reason| violation(span_of(PREPARE), reason)));
            None
        }
    };
    let mut phases = Vec::new();
    for name in [EXECUTE_EXPORT, STATUS, STOP] {
        match contract_signature(&signature(name), name) {
            Ok(contract) => phases.push((name, contract)),
            Err(reasons) => {
                violations.extend(reasons.into_iter().map(|reason| violation(span_of(name), reason)));
            }
        }
    }
    if let Some(prepare) = &prepare {
        let prepared = prepared_record_type(&prepare.result_type);
        for (name, contract) in &phases {
            if !same_shape(&contract.data_type, &prepared) {
                violations.push(violation(
                    span_of(name),
                    format!(
                        "`{name}`'s `{DATA_PARAM}` is the PREPARED record, `{}`, not `{}`",
                        prepared.to_inline(),
                        contract.data_type.to_inline(),
                    ),
                ));
            }
        }
    }
    if !violations.is_empty() {
        violations.truncate(LINT_VIOLATION_CAP);
        return Err(violations);
    }
    let findings = lint_block(&block, &ws, module, start);
    if !findings.is_empty() {
        return Err(findings);
    }
    let (Some(prepare), [(_, execute), (_, status), (_, stop)]) = (prepare, phases.as_slice()) else {
        return Err(vec![violation(None, "the signatures did not resolve".to_string())]);
    };
    let phase = |contract: &SignatureContract| PhaseDef {
        result_type: contract.result_type.clone(),
        takes_timeout: contract.takes_timeout,
    };
    Ok(SuiteDef {
        data_type: prepare.data_type,
        prepared_type: prepare.result_type,
        takes_timeouts: prepare.takes_timeouts,
        takes_heartbeats: prepare.takes_heartbeats,
        execute: phase(execute),
        status: phase(status),
        stop: phase(stop),
    })
}

/// Hold prepare's signature to its contract: `data`, `timeouts?`, `heartbeats?`.
fn prepare_contract(signature: &nu::Signature) -> Result<PrepareContract, Vec<String>> {
    let mut violations: Vec<String> = Vec::new();
    let data_type = match signature.required_positional.as_slice() {
        [data] if data.name == DATA_PARAM => match suite_data_typedef(&data.shape) {
            Ok(typedef) => Some(typedef),
            Err(reason) => {
                violations.push(format!("`{PREPARE}`'s `{DATA_PARAM}`: {reason}"));
                None
            }
        },
        _ => {
            violations.push(format!("`{PREPARE}` takes one required parameter, `{DATA_PARAM}`"));
            None
        }
    };
    let names: Vec<&str> = signature
        .optional_positional
        .iter()
        .map(|param| param.name.as_str())
        .collect();
    let (takes_timeouts, takes_heartbeats) = match names.as_slice() {
        [] => (false, false),
        [TIMEOUTS_PARAM] => (true, false),
        [TIMEOUTS_PARAM, HEARTBEATS_PARAM] => (true, true),
        _ => {
            violations.push(format!(
                "`{PREPARE}`'s optional parameters are `{TIMEOUTS_PARAM}?`, then \
                 `{HEARTBEATS_PARAM}?`; the last may be left off, or both"
            ));
            (false, false)
        }
    };
    for param in &signature.optional_positional {
        let expected = match param.name.as_str() {
            TIMEOUTS_PARAM => timeouts_type(),
            HEARTBEATS_PARAM => heartbeats_type(),
            _ => continue,
        };
        match NuType::try_from(&param.shape) {
            Ok(declared) if same_shape(&declared, &expected) => {}
            Ok(declared) => violations.push(format!(
                "`{}?` is typed `{}`, not `{}`",
                param.name,
                expected.to_inline(),
                declared.to_inline(),
            )),
            Err(_) => violations.push(format!(
                "`{}?` is typed `{}`",
                param.name,
                expected.to_inline(),
            )),
        }
        if param.default_value.is_some() {
            violations.push(format!("`{}?` takes no default; the call fills it", param.name));
        }
    }
    if signature.rest_positional.is_some() {
        violations.push(format!("`{PREPARE}` takes no rest parameter"));
    }
    if signature.named.iter().any(|flag| flag.long != HELP_FLAG) {
        violations.push(format!("`{PREPARE}` takes no flags"));
    }
    let result_type = match signature.input_output_types.as_slice() {
        [(nu::Type::Nothing, output)] => match result_typedef(output) {
            Ok(typedef) => Some(typedef),
            Err(reason) => {
                violations.push(format!("`{PREPARE}`'s return type: {reason}"));
                None
            }
        },
        _ => {
            violations.push(format!(
                "`{PREPARE}` declares one infix signature, its input `nothing`: \
                 `: nothing -> record<...>`"
            ));
            None
        }
    };
    match (data_type, result_type) {
        (Some(data_type), Some(result_type)) if violations.is_empty() => Ok(PrepareContract {
            data_type,
            result_type,
            takes_timeouts,
            takes_heartbeats,
        }),
        _ => Err(violations),
    }
}
