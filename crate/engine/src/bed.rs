//! A bed call: one embedded command held to the interface's rules, then run.
use crate::*;

/// The kind every refusal is reported under.
pub const KIND_BED: &str = "bed::invalid";

/// The name a bed call's command is parsed under, which positions its rows.
const BED_FILE: &str = "bed";

/// The engine's embedded commands outside `interface`, a bed call's `command`.
pub const EMBEDDED: [&str; 1] = [GRMR];

/// A bed call's command text, held to the rules and ready to run.
///
/// One call of an embedded command, its word prepended to `subcommand` where
/// it doesn't open with it, its arguments literals: no pipe, no other command,
/// no variable, and no expression. A refusal is positioned in `subcommand`.
pub fn analyze_bed(
    parse_engine: &ParseEngine,
    command: &str,
    subcommand: &str,
) -> Result<String, Vec<Diagnostic>> {
    let Some(head) = EMBEDDED.iter().copied().find(|head| *head == command.trim()) else {
        return Err(vec![Diagnostic::error(
            KIND_BED,
            None,
            format!(
                "`{}` is no embedded command; the one is `{}`",
                command.trim(),
                EMBEDDED.join("`, `"),
            ),
        )]);
    };
    let (text, prefix) = with_head(subcommand, head);
    let mut ws = nu::StateWorkingSet::new(parse_engine.engine_state());
    let block = nu::parse(&mut ws, Some(BED_FILE), text.as_bytes(), false);
    let start = file_start(&ws, BED_FILE);
    let at = |span: Option<nu::Span>| -> [u64; 2] {
        match span {
            Some(span) => {
                let offset = span.start.saturating_sub(start).saturating_sub(prefix);
                let (line, col) = span_to_line_col(subcommand, offset);
                [wide(line), wide(col)]
            }
            None => [0, 0],
        }
    };
    let row = |span: Option<nu::Span>, message: String| {
        Diagnostic::error(KIND_BED, Some(Source { path: None, position: at(span) }), message)
    };
    if !ws.parse_errors.is_empty() {
        let mut rows: Vec<Diagnostic> = Vec::new();
        for error in &ws.parse_errors {
            let refused = row(
                Some(error.span()),
                format!("parse error: {}", nuval::NuValError::from(error).short()),
            );
            if !rows.iter().any(|seen| seen.message == refused.message) {
                rows.push(refused);
            }
        }
        rows.truncate(LINT_VIOLATION_CAP);
        return Err(rows);
    }
    match held_call(&ws, &block, head, subcommand) {
        Ok(()) => Ok(text),
        Err(refusals) => {
            let mut rows: Vec<Diagnostic> =
                refusals.into_iter().map(|(span, message)| row(span, message)).collect();
            rows.truncate(LINT_VIOLATION_CAP);
            Err(rows)
        }
    }
}

/// The one call the block holds, `head`'s, its arguments literals.
fn held_call(
    ws: &nu::StateWorkingSet,
    block: &nu::Block,
    head: &str,
    sent: &str,
) -> Result<(), Vec<(Option<nu::Span>, String)>> {
    let refuse = |span: Option<nu::Span>, message: String| vec![(span, message)];
    let [pipeline] = block.pipelines.as_slice() else {
        return Err(refuse(
            None,
            format!("one `{head}` command, alone; found {} statements", block.pipelines.len()),
        ));
    };
    let element = match pipeline.elements.as_slice() {
        [element] => element,
        [_, next, ..] => {
            return Err(refuse(Some(next.expr.span), format!("one `{head}` command, never piped")));
        }
        [] => return Err(refuse(None, format!("one `{head}` command, alone"))),
    };
    if element.redirection.is_some() {
        return Err(refuse(
            Some(element.expr.span),
            format!("a `{head}` command takes no redirection"),
        ));
    }
    let nu::Expr::Call(call) = &element.expr.expr else {
        return Err(refuse(
            Some(element.expr.span),
            format!(
                "`{}` is no `{head}` command; the commands are in the tool's description",
                sent.trim(),
            ),
        ));
    };
    let name = ws.get_decl(call.decl_id).name();
    if !name.strip_prefix(head).is_some_and(|rest| rest.starts_with(' ')) {
        return Err(refuse(
            Some(element.expr.span),
            format!("`{name}` is no `{head}` command; the commands are in the tool's description"),
        ));
    }
    let mut refusals = Vec::new();
    for argument in &call.arguments {
        match argument {
            nu::Argument::Positional(expr) | nu::Argument::Named((_, _, Some(expr))) => {
                if !literal(expr) {
                    refusals.push((
                        Some(expr.span),
                        "an argument is a literal, never a variable, an expression, or a spread"
                            .to_string(),
                    ));
                }
            }
            nu::Argument::Named((_, _, None)) => {}
            nu::Argument::Unknown(expr) | nu::Argument::Spread(expr) => {
                refusals.push((Some(expr.span), "a literal argument, never spread".to_string()));
            }
        }
    }
    match refusals.is_empty() {
        true => Ok(()),
        false => Err(refusals),
    }
}

/// Whether an expression is a literal: a scalar, or a list, record, or table.
fn literal(expr: &nu::Expression) -> bool {
    match &expr.expr {
        nu::Expr::FullCellPath(path) => path.tail.is_empty() && literal(&path.head),
        nu::Expr::Bool(_)
        | nu::Expr::Int(_)
        | nu::Expr::Float(_)
        | nu::Expr::Binary(_)
        | nu::Expr::String(_)
        | nu::Expr::RawString(_)
        | nu::Expr::DateTime(_)
        | nu::Expr::Nothing => true,
        nu::Expr::ValueWithUnit(value) => literal(&value.expr),
        nu::Expr::List(items) => items
            .iter()
            .all(|item| matches!(item, nu::ListItem::Item(item) if literal(item))),
        nu::Expr::Record(items) => items.iter().all(
            |item| matches!(item, nu::RecordItem::Pair(key, value) if literal(key) && literal(value)),
        ),
        nu::Expr::Table(table) => {
            table.columns.iter().all(literal) && table.rows.iter().all(|row| row.iter().all(literal))
        }
        _ => false,
    }
}
