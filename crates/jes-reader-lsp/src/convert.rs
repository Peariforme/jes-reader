//! Conversion from the `jes-reader-core` model to LSP types: diagnostics,
//! symbols (Outline) and folding ranges.
//!
//! The core counts columns in bytes; LSP expects UTF-16 code units. The
//! conversion happens here, from the text of each line.

use std::ops::Range;

use jes_reader_core::{
    Dd, DdKind, Finding, FindingSeverity, Statement, StatementOrigin, Step, StepStatus,
};
use lsp_types::{
    Diagnostic, DiagnosticSeverity, DocumentSymbol, FoldingRange, NumberOrString, Position,
    Range as LspRange, SymbolKind,
};

/// Text of a document split into lines, the way the parser does it.
pub struct Lines<'a>(Vec<&'a str>);

impl<'a> Lines<'a> {
    pub fn new(text: &'a str) -> Self {
        Self(text.lines().collect())
    }

    fn line(&self, n: usize) -> &str {
        self.0.get(n).copied().unwrap_or("")
    }

    /// LSP position of a byte offset in line `n`.
    fn position(&self, n: usize, byte: usize) -> Position {
        let line = self.line(n);
        let mut byte = byte.min(line.len());
        while !line.is_char_boundary(byte) {
            byte -= 1;
        }
        Position::new(to_u32(n), to_u32(line[..byte].encode_utf16().count()))
    }

    /// Columns `columns` of line `n`.
    pub fn span(&self, n: usize, columns: Range<usize>) -> LspRange {
        LspRange::new(
            self.position(n, columns.start),
            self.position(n, columns.end),
        )
    }

    /// Content of line `n`, without the ASA character and leading spaces.
    pub fn content(&self, n: usize) -> LspRange {
        let line = self.line(n);
        let start = line
            .char_indices()
            .skip(1)
            .find(|(_, c)| !c.is_whitespace())
            .map_or(0, |(i, _)| i);
        self.span(n, start..line.len())
    }

    /// Byte offset of the UTF-16 column `character` in line `n`.
    pub fn byte_column(&self, n: usize, character: u32) -> usize {
        let mut units = 0;
        for (i, c) in self.line(n).char_indices() {
            if units >= character as usize {
                return i;
            }
            units += c.len_utf16();
        }
        self.line(n).len()
    }

    /// Whole lines `lines` (exclusive upper bound).
    fn block(&self, lines: &Range<usize>) -> LspRange {
        let last = lines.end.max(lines.start + 1) - 1;
        LspRange::new(
            Position::new(to_u32(lines.start), 0),
            self.position(last, usize::MAX),
        )
    }
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

pub fn diagnostic(finding: &Finding, lines: &Lines<'_>) -> Diagnostic {
    let range = match &finding.columns {
        Some(columns) => lines.span(finding.line, columns.clone()),
        None => lines.content(finding.line),
    };
    Diagnostic {
        range,
        severity: Some(match finding.severity {
            FindingSeverity::Error => DiagnosticSeverity::ERROR,
            FindingSeverity::Warning => DiagnosticSeverity::WARNING,
            FindingSeverity::Information => DiagnosticSeverity::INFORMATION,
        }),
        code: Some(NumberOrString::String(finding.code.clone())),
        source: Some("jes-reader".into()),
        message: finding.message.clone(),
        ..Diagnostic::default()
    }
}

#[allow(deprecated)] // `DocumentSymbol::deprecated` is required in the struct literal.
fn symbol(
    name: String,
    detail: Option<String>,
    kind: SymbolKind,
    range: LspRange,
    selection_range: LspRange,
    children: Vec<DocumentSymbol>,
) -> DocumentSymbol {
    DocumentSymbol {
        name,
        detail,
        kind,
        tags: None,
        deprecated: None,
        range,
        selection_range,
        children: (!children.is_empty()).then_some(children),
    }
}

/// Outline: steps for JESMSGLG and JESYSMSG, a JOB → EXEC → DD tree for
/// JESJCL, messages for other DDs.
pub fn symbols(dd: &Dd, lines: &Lines<'_>) -> Vec<DocumentSymbol> {
    match dd.kind {
        DdKind::JesMsgLg | DdKind::JesYsMsg => {
            dd.steps.iter().map(|s| step_symbol(s, lines)).collect()
        }
        DdKind::JesJcl => jcl_symbols(&dd.statements, lines),
        DdKind::Other => dd
            .messages
            .iter()
            .map(|m| {
                symbol(
                    m.id.clone(),
                    Some(m.text.clone()),
                    SymbolKind::EVENT,
                    lines.block(&m.lines),
                    lines.span(m.lines.start, m.id_columns.clone()),
                    Vec::new(),
                )
            })
            .collect(),
    }
}

fn step_symbol(step: &Step, lines: &Lines<'_>) -> DocumentSymbol {
    let detail = match &step.status {
        StepStatus::Completed { return_code } => format!("RC {return_code}"),
        StepStatus::Abended { code, .. } => format!("abend {code}"),
        StepStatus::NotRun => "not run".into(),
        StepStatus::Unknown => "running or unknown".into(),
    };
    symbol(
        step.display_name(),
        Some(detail),
        SymbolKind::FUNCTION,
        lines.block(&step.lines),
        lines.content(step.status_line),
        Vec::new(),
    )
}

/// Statement tree: JOB, then job EXEC, then procedure EXEC, with the DDs
/// under the nearest EXEC.
fn jcl_symbols(statements: &[Statement], lines: &Lines<'_>) -> Vec<DocumentSymbol> {
    // (statement, children), from outermost to innermost.
    let mut job: Option<(&Statement, Vec<DocumentSymbol>)> = None;
    let mut exec: Option<(&Statement, Vec<DocumentSymbol>)> = None;
    let mut proc_exec: Option<(&Statement, Vec<DocumentSymbol>)> = None;
    let mut roots = Vec::new();

    let close = |node: Option<(&Statement, Vec<DocumentSymbol>)>| {
        node.map(|(st, children)| statement_symbol(st, children, lines))
    };

    for st in statements {
        match st.operation.as_deref() {
            Some("JOB") => {
                let proc_sym = close(proc_exec.take());
                attach(&mut exec, proc_sym, &mut roots);
                let exec_sym = close(exec.take());
                attach(&mut job, exec_sym, &mut roots);
                roots.extend(close(job.take()));
                job = Some((st, Vec::new()));
            }
            Some("EXEC") if st.origin == StatementOrigin::Job => {
                let proc_sym = close(proc_exec.take());
                attach(&mut exec, proc_sym, &mut roots);
                let exec_sym = close(exec.take());
                attach(&mut job, exec_sym, &mut roots);
                exec = Some((st, Vec::new()));
            }
            Some("EXEC") => {
                let proc_sym = close(proc_exec.take());
                attach(&mut exec, proc_sym, &mut roots);
                proc_exec = Some((st, Vec::new()));
            }
            Some("DD") if st.name.is_some() => {
                let dd_sym = statement_symbol(st, Vec::new(), lines);
                let parent = if proc_exec.is_some() {
                    &mut proc_exec
                } else {
                    &mut exec
                };
                attach(parent, Some(dd_sym), &mut roots);
            }
            _ => {}
        }
    }
    let proc_sym = close(proc_exec);
    attach(&mut exec, proc_sym, &mut roots);
    let exec_sym = close(exec);
    attach(&mut job, exec_sym, &mut roots);
    roots.extend(close(job));
    roots
}

/// Adds `child` to the `parent` node if there is one, otherwise to the roots.
fn attach(
    parent: &mut Option<(&Statement, Vec<DocumentSymbol>)>,
    child: Option<DocumentSymbol>,
    roots: &mut Vec<DocumentSymbol>,
) {
    let Some(child) = child else { return };
    match parent {
        Some((_, children)) => children.push(child),
        None => roots.push(child),
    }
}

fn statement_symbol(
    st: &Statement,
    children: Vec<DocumentSymbol>,
    lines: &Lines<'_>,
) -> DocumentSymbol {
    let operation = st.operation.clone().unwrap_or_default();
    let kind = match operation.as_str() {
        "JOB" => SymbolKind::MODULE,
        "EXEC" => SymbolKind::FUNCTION,
        _ => SymbolKind::FIELD,
    };
    let detail = match st.origin {
        StatementOrigin::Job => format!("{operation} · stmt {}", st.number),
        StatementOrigin::Procedure => format!("{operation} · stmt {} · procedure", st.number),
        StatementOrigin::Override => format!("{operation} · stmt {} · override", st.number),
    };
    // The symbol covers the statement and all its children.
    let mut range = lines.block(&st.lines);
    if let Some(last) = children.last() {
        range.end = range.end.max(last.range.end);
    }
    let selection_range = lines.content(st.lines.start);
    symbol(
        st.name.clone().unwrap_or_else(|| operation.clone()),
        Some(detail),
        kind,
        range,
        selection_range,
        children,
    )
}

/// Folding ranges: steps, statements and multi-line messages.
pub fn folding_ranges(dd: &Dd) -> Vec<FoldingRange> {
    let blocks = dd
        .steps
        .iter()
        .map(|s| &s.lines)
        .chain(dd.statements.iter().map(|s| &s.lines))
        .chain(dd.messages.iter().map(|m| &m.lines));
    let mut ranges: Vec<FoldingRange> = blocks
        .filter(|lines| lines.len() > 1)
        .map(|lines| FoldingRange {
            start_line: to_u32(lines.start),
            end_line: to_u32(lines.end - 1),
            ..FoldingRange::default()
        })
        .collect();
    ranges.sort_by_key(|r| (r.start_line, std::cmp::Reverse(r.end_line)));
    ranges.dedup_by_key(|r| (r.start_line, r.end_line));
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_columns() {
        let lines = Lines::new(" é IEF142I\n");
        // `é` is 2 bytes but 1 UTF-16 code unit.
        assert_eq!(lines.position(0, 4), Position::new(0, 3));
        assert_eq!(lines.position(0, 2), Position::new(0, 1));
        assert_eq!(lines.position(0, usize::MAX), Position::new(0, 10));
        assert_eq!(lines.position(5, 3), Position::new(5, 0));
    }

    #[test]
    fn content_without_asa() {
        let lines = Lines::new("0  IDC3012I ENTRY\n");
        assert_eq!(
            lines.content(0),
            LspRange::new(Position::new(0, 3), Position::new(0, 17))
        );
    }
}
