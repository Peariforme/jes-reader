//! Conversion from the `jes-reader-core` model to LSP types: diagnostics,
//! symbols (Outline) and folding ranges.
//!
//! The core counts columns in bytes; LSP expects UTF-16 code units. The
//! conversion happens here, from the text of each line.

use std::ops::Range;

use jes_reader_core::{
    Dd, DdKind, Finding, FindingSeverity, Options, Statement, StatementOrigin, Step, StepStatus,
    analyze, duration_text,
};
use lsp_types::{
    CodeLens, Command, Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity,
    DocumentSymbol, FoldingRange, Location, NumberOrString, Position, Range as LspRange,
    SemanticToken, SemanticTokenType, SemanticTokens, SemanticTokensLegend, SymbolKind, Uri,
};

/// Text of a document split into lines, the way the parser does it.
pub struct Lines<'a>(Vec<&'a str>);

impl<'a> Lines<'a> {
    pub fn new(text: &'a str) -> Self {
        Self(text.lines().collect())
    }

    pub fn line(&self, n: usize) -> &str {
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

/// Diagnostic of a finding in the document `uri`; related lines (cause of an
/// abend, step a message made fail) become related information.
pub fn diagnostic(finding: &Finding, lines: &Lines<'_>, uri: &Uri) -> Diagnostic {
    let range = match &finding.columns {
        Some(columns) => lines.span(finding.line, columns.clone()),
        None => lines.content(finding.line),
    };
    let related: Vec<DiagnosticRelatedInformation> = finding
        .related
        .iter()
        .map(|r| DiagnosticRelatedInformation {
            location: Location::new(uri.clone(), lines.content(r.line)),
            message: r.message.clone(),
        })
        .collect();
    Diagnostic {
        related_information: (!related.is_empty()).then_some(related),
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

/// Outline of a document: the symbols of its DD, or for a whole spool one
/// symbol per DD with the DD's symbols under it.
pub fn document_symbols(dds: &[Dd], lines: &Lines<'_>) -> Vec<DocumentSymbol> {
    if let [dd] = dds {
        return symbols(dd, lines);
    }
    dds.iter()
        .map(|dd| {
            let block = dd.first_line..dd.first_line + dd.line_count;
            symbol(
                dd.name.clone().unwrap_or_else(|| "Step output".to_owned()),
                Some(format!("{} lines", dd.line_count)),
                SymbolKind::FILE,
                lines.block(&block),
                lines.content(dd.first_line),
                symbols(dd, lines),
            )
        })
        .collect()
}

/// Folding ranges of a document: those of each DD, and each DD of a whole
/// spool.
pub fn document_folding_ranges(dds: &[Dd]) -> Vec<FoldingRange> {
    let mut ranges: Vec<FoldingRange> = dds.iter().flat_map(folding_ranges).collect();
    if dds.len() > 1 {
        ranges.extend(
            dds.iter()
                .filter(|dd| dd.line_count > 1)
                .map(|dd| FoldingRange {
                    start_line: to_u32(dd.first_line),
                    end_line: to_u32(dd.first_line + dd.line_count - 1),
                    ..FoldingRange::default()
                }),
        );
        ranges.sort_by_key(|r| (r.start_line, std::cmp::Reverse(r.end_line)));
        ranges.dedup_by_key(|r| r.start_line);
    }
    ranges
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

/// Short outcome of a step: `RC 8`, `abend S0C7`.
fn outcome(status: &StepStatus) -> String {
    match status {
        StepStatus::Completed { return_code } => format!("RC {return_code}"),
        StepStatus::Abended { code, .. } => format!("abend {code}"),
        StepStatus::NotRun => "not run".into(),
        StepStatus::Unknown => "running or unknown".into(),
    }
}

fn step_symbol(step: &Step, lines: &Lines<'_>) -> DocumentSymbol {
    let mut detail = outcome(&step.status);
    if let Some(program) = &step.program {
        detail = format!("{detail} · {program}");
    }
    symbol(
        step.display_name(),
        Some(detail),
        SymbolKind::FUNCTION,
        lines.block(&step.lines),
        lines.content(step.status_line),
        Vec::new(),
    )
}

/// Semantic token types, in legend order. The VS Code extension maps them
/// to the TextMate scopes of its grammar (`semanticTokenScopes`), so that
/// themes color them like the grammar does.
const TOKEN_TYPES: [&str; 4] = ["jesError", "jesWarning", "jesInformation", "jesMessage"];

/// Legend announced in the server capabilities.
pub fn semantic_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TOKEN_TYPES.map(SemanticTokenType::new).to_vec(),
        token_modifiers: Vec::new(),
    }
}

/// One token per message ID, typed by the severity the analysis gives it:
/// the catalog and the house rules decide, not the suffix the grammar sees.
/// A message the analysis does not report (an `ignore` rule, an
/// information message) is a plain `jesMessage`.
pub fn semantic_tokens(dds: &[Dd], options: &Options, lines: &Lines<'_>) -> SemanticTokens {
    let mut data = Vec::new();
    let mut previous = Position::new(0, 0);
    for dd in dds {
        let findings = analyze(dd, options);
        for m in &dd.messages {
            let severity = findings
                .iter()
                .find(|f| f.line == m.lines.start && f.columns.as_ref() == Some(&m.id_columns))
                .map(|f| f.severity);
            let token_type = match severity {
                Some(FindingSeverity::Error) => 0,
                Some(FindingSeverity::Warning) => 1,
                Some(FindingSeverity::Information) => 2,
                None => 3,
            };
            let range = lines.span(m.lines.start, m.id_columns.clone());
            let start = range.start;
            let delta_line = start.line - previous.line;
            let delta_start = if delta_line == 0 {
                start.character - previous.character
            } else {
                start.character
            };
            data.push(SemanticToken {
                delta_line,
                delta_start,
                length: range.end.character - start.character,
                token_type,
                token_modifiers_bitset: 0,
            });
            previous = start;
        }
    }
    SemanticTokens {
        result_id: None,
        data,
    }
}

/// Code lenses: a summary above each step of JESYSMSG,
/// `STEP010 · IDCAMS · RC 8 · CPU 0.01 s · < 1 min`. JESMSGLG has none: its
/// step table already gives the same figures on one line per step.
pub fn code_lenses(dd: &Dd, lines: &Lines<'_>) -> Vec<CodeLens> {
    if dd.kind != DdKind::JesYsMsg {
        return Vec::new();
    }
    dd.steps
        .iter()
        .map(|step| CodeLens {
            range: lines.content(step.lines.start),
            // No command: the lens is a label, not a link.
            command: Some(Command::new(step_summary(step), String::new(), None)),
            data: None,
        })
        .collect()
}

/// One-line summary of a step.
fn step_summary(step: &Step) -> String {
    let mut parts = vec![step.display_name()];
    parts.extend(step.program.clone());
    parts.push(outcome(&step.status));
    if let Some(cpu) = step.cpu {
        parts.push(format!("CPU {}", duration_text(cpu)));
    }
    if let Some(elapsed) = step.elapsed_text() {
        parts.push(format!("elapsed {elapsed}"));
    }
    parts.join(" · ")
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
fn folding_ranges(dd: &Dd) -> Vec<FoldingRange> {
    let jcl_blocks = jcl_blocks(&dd.statements);
    let blocks = dd
        .steps
        .iter()
        .map(|s| &s.lines)
        .chain(&jcl_blocks)
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
    // One range per start line, the longest: an EXEC block hides the
    // range of the EXEC statement alone.
    ranges.sort_by_key(|r| (r.start_line, std::cmp::Reverse(r.end_line)));
    ranges.dedup_by_key(|r| r.start_line);
    ranges
}

/// JESJCL blocks, as in the Outline: a JOB up to the next JOB, a job-level
/// EXEC up to the next job-level EXEC (its procedure and DDs included), a
/// procedure EXEC up to the next EXEC. Each ends where the next one starts,
/// or with the last statement.
fn jcl_blocks(statements: &[Statement]) -> Vec<Range<usize>> {
    // 0: JOB, 1: job-level EXEC, 2: procedure EXEC.
    let level = |st: &Statement| match st.operation.as_deref() {
        Some("JOB") => Some(0),
        Some("EXEC") if st.origin == StatementOrigin::Job => Some(1),
        Some("EXEC") => Some(2),
        _ => None,
    };
    let Some(last) = statements.last() else {
        return Vec::new();
    };
    statements
        .iter()
        .enumerate()
        .filter_map(|(i, st)| {
            let own = level(st)?;
            let end = statements[i + 1..]
                .iter()
                .find(|next| level(next).is_some_and(|l| l <= own))
                .map_or(last.lines.end, |next| next.lines.start);
            Some(st.lines.start..end)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jcl_folding() {
        // JOB, EXEC calling a procedure (PROC, EXEC COBOL, DDs, EXEC LKED,
        // DD), a DD override, then a second job-level EXEC with a DD.
        let text = "        1 //JRS0C7   JOB (ACCT)\n\
                    \x20       2 //COBRUN   EXEC IGYWCLG\n\
                    \x20       3 XXIGYWCLG PROC LNGPRFX='IGY'\n\
                    \x20       4 XXCOBOL   EXEC PGM=IGYCRCTL,\n\
                    \x20         XX             REGION=0M\n\
                    \x20       5 XXSTEPLIB DD DSN=IGY.SIGYCOMP,\n\
                    \x20         XX             DISP=SHR\n\
                    \x20       6 XXSYSPRINT DD SYSOUT=*\n\
                    \x20       7 XXLKED    EXEC PGM=IEWBLINK\n\
                    \x20       8 XXSYSLIB  DD DSN=CEE.SCEELKED,DISP=SHR\n\
                    \x20       9 //COBOL.SYSIN DD *\n\
                    \x20      10 //STEP2    EXEC PGM=IEFBR14\n\
                    \x20      11 //DD1      DD DUMMY\n";
        let dd = jes_reader_core::parse_file("JOB00896/JES2/JESJCL.txt", text);
        let ranges: Vec<(u32, u32)> = folding_ranges(&dd)
            .iter()
            .map(|r| (r.start_line, r.end_line))
            .collect();
        assert_eq!(
            ranges,
            [
                (0, 12),  // JOB: whole job
                (1, 10),  // COBRUN: procedure and override, up to STEP2
                (3, 7),   // COBOL: up to LKED
                (5, 6),   // STEPLIB, continued
                (8, 10),  // LKED: up to STEP2, the override included
                (11, 12), // STEP2: its DD
            ]
        );
    }

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
    fn semantic_tokens_follow_the_analysis() {
        let text = " IEF142I JRS013 STEP010 - STEP WAS EXECUTED - COND CODE 0000\n\
                    \x20IEC141I 013-18,IGG0191B,JRS013,STEP010,SYSUT1,1001,VTMVSC,\n\
                    \x20IDI0078E Open of fault history file failed\n";
        let dd = jes_reader_core::parse_file("JOB00901/JES2/JESYSMSG.txt", text);
        let lines = Lines::new(text);
        // Absolute (line, column, length, type) of each token.
        let decode = |tokens: SemanticTokens| {
            let (mut line, mut column) = (0, 0);
            tokens
                .data
                .iter()
                .map(|t| {
                    if t.delta_line > 0 {
                        column = 0;
                    }
                    line += t.delta_line;
                    column += t.delta_start;
                    (line, column, t.length, TOKEN_TYPES[t.token_type as usize])
                })
                .collect::<Vec<_>>()
        };
        let options = Options::default();
        assert_eq!(
            decode(semantic_tokens(std::slice::from_ref(&dd), &options, &lines)),
            [
                (0, 1, 7, "jesMessage"),
                (1, 1, 7, "jesError"),
                (2, 1, 8, "jesError"),
            ]
        );
        // A house rule silences IDI0078E: its ID is no longer red.
        let options = Options {
            rules: jes_reader_core::Rules::parse(
                "[[messages]]\nid = \"IDI*\"\nseverity = \"ignore\"\n",
            )
            .unwrap(),
            ..Options::default()
        };
        let tokens = decode(semantic_tokens(&[dd], &options, &lines));
        assert_eq!(tokens[2].3, "jesMessage");
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
