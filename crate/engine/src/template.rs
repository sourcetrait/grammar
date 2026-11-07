use crate::*;
use indoc::formatdoc;

const CALL_ALIAS: &str = "__call";

/// A value as compact NUON, the literal a source carries.
pub(crate) fn render_nuon(value: &nu::Value) -> Result<String, String> {
    nu::to_nuon(&nu::EngineState::new(), value, nu::ToNuonConfig::default())
        .map_err(|e| e.to_string())
}

/// The source of a call: its target's module overlaid, then driven on its data.
///
/// The target is named by its absolute path under the equip root.
pub fn build_call_source(
    equip_root: &Path,
    rig: &str,
    module_path: &str,
    name: &str,
    data_literal: &str,
    timeout_literal: Option<&str>,
) -> Result<String, String> {
    let mut target = equip_root.join(rig).join("rig");
    if !module_path.is_empty() {
        target = target.join(module_path);
    }
    let target = render_nuon(&nu::Value::string(
        target.join(name).to_string_lossy().into_owned(),
        nu::Span::unknown(),
    ))?;
    let timeout = timeout_literal
        .map(|literal| format!(" {literal}"))
        .unwrap_or_default();
    Ok(formatdoc!(
        r#"
        overlay use --prefix {target} as {alias}
        {alias} {data_literal}{timeout}
    "#,
        target = target,
        alias = CALL_ALIAS,
        data_literal = data_literal,
        timeout = timeout,
    ))
}

/// The alias a suite's module is overlaid under in each of its evals.
const SUITE_ALIAS: &str = "__suite";

/// The variable an absent argument rides as, null typed `any`.
const ABSENT: &str = "__absent";

/// A suite definition's eval source: its module inline, then the export called.
pub fn build_suite_source(
    module: &str,
    export: &str,
    args: &[Option<String>],
) -> Result<String, String> {
    let (wrapped, _) = wrap_as_module(module, SUITE_ALIAS);
    let absent = if args.iter().any(Option::is_none) {
        format!("let {ABSENT}: any = null\n")
    } else {
        String::new()
    };
    let args: Vec<String> = args
        .iter()
        .map(|arg| match arg {
            Some(literal) => literal.clone(),
            None => format!("${ABSENT}"),
        })
        .collect();
    Ok(format!(
        "{wrapped}overlay use --prefix {SUITE_ALIAS}\n{absent}{SUITE_ALIAS} {export} {}\n",
        args.join(" "),
    ))
}

/// The source of an `execute` call: its definition, then `execute` on its data.
pub fn build_execute_source(
    definition: &str,
    data_literal: &str,
    timeout_literal: Option<&str>,
) -> String {
    let timeout = timeout_literal
        .map(|literal| format!(" {literal}"))
        .unwrap_or_default();
    format!("{definition}\n{EXECUTE} {data_literal}{timeout}\n")
}
