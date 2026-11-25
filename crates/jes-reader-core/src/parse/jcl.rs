//! JESJCL: expanded JCL, numbered statements.
//!
//! ```text
//!         1 //JRS0C7   JOB (ACCT),'JES-READER S0C7',CLASS=A,          JOB00896
//!           //             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//!           IEFC653I SUBSTITUTION JCL - (ACCT),'JES-READER S0C7',…
//!         4 XXCOBOL   EXEC PGM=IGYCRCTL,REGION=0M
//!           X/SYSIN    DD  DSNAME=&SYSUID..CBL(&SRC),DISP=SHR
//! ```
//!
//! Number right-aligned in columns 2-9, statement from column 11 (offset
//! 10), prefixed with `//`, `XX`, `++`, `X/` or `+/`. Lines without a number
//! continue the previous statement; `IEFC653I` is a message.

use super::Messages;
use crate::model::{Dd, Statement, StatementOrigin};
use crate::msgid::message_id_at;
use crate::source::is_job_id;

/// Offset where the statement starts (`//` prefix included).
const BODY: usize = 10;

pub(super) fn parse(lines: &[&str], dd: &mut Dd) {
    let mut messages = Messages::new();
    let mut statements: Vec<Statement> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        messages.close();
        if let Some(id) = message_id_at(line, BODY) {
            messages.start(i, line, id, None);
        } else if let Some(statement) = numbered_statement(line, i) {
            if statement.number == 1 && statement.operation.as_deref() == Some("JOB") {
                read_job_card(line, &statement, dd);
            }
            statements.push(statement);
            continue;
        }
        if let Some(last) = statements.last_mut() {
            last.lines.end = i + 1;
        }
    }

    for st in &mut statements {
        if st.operation.as_deref() == Some("EXEC") {
            st.program = program(&lines[st.lines.clone()]);
        }
    }
    dd.messages = messages.finish();
    dd.statements = statements;
}

/// `PGM=` value of an EXEC statement's lines. The `IEFC653I SUBSTITUTION
/// JCL` line, when present, wins: it has symbols resolved (`PGM=&PROG`).
fn program(lines: &[&str]) -> Option<String> {
    let mut found = None;
    for line in lines {
        let text = line.get(BODY..).unwrap_or("");
        if text.get(2..3) == Some("*") {
            continue; // `//*` comment
        }
        let substituted = text.starts_with("IEFC653I");
        if let Some(value) = pgm_value(text)
            && (substituted || found.is_none())
        {
            found = Some(value.to_owned());
        }
    }
    found
}

/// Value of the `PGM=` keyword in `text`, up to the next `,`, space or `)`.
fn pgm_value(text: &str) -> Option<&str> {
    let mut from = 0;
    while let Some(i) = text[from..].find("PGM=") {
        let at = from + i;
        let starts_keyword = at == 0 || matches!(text.as_bytes()[at - 1], b' ' | b',' | b'(');
        if starts_keyword {
            let value = &text[at + 4..];
            let end = value.find([',', ' ', ')']).unwrap_or(value.len());
            return (end > 0).then(|| &value[..end]);
        }
        from = at + 4;
    }
    None
}

fn numbered_statement(line: &str, line_no: usize) -> Option<Statement> {
    let number = line.get(1..9)?.trim_start().parse().ok()?;
    if line.as_bytes().get(9) != Some(&b' ') {
        return None;
    }
    let origin = match line.get(BODY..BODY + 2)? {
        "//" => StatementOrigin::Job,
        "XX" | "++" => StatementOrigin::Procedure,
        "X/" | "+/" => StatementOrigin::Override,
        _ => return None,
    };
    let fields = line.get(BODY + 2..).unwrap_or("");
    let (name, operation) = if fields.starts_with(' ') {
        (None, fields.split_whitespace().next())
    } else {
        let mut words = fields.split_whitespace();
        (words.next(), words.next())
    };
    Some(Statement {
        number,
        origin,
        name: name.map(str::to_owned),
        operation: operation.map(str::to_owned),
        lines: line_no..line_no + 1,
        program: None,
    })
}

/// The JOB card gives the job name and, at the end of the line, the JES job ID.
fn read_job_card(line: &str, statement: &Statement, dd: &mut Dd) {
    if dd.job_name.is_none() {
        dd.job_name.clone_from(&statement.name);
    }
    if dd.job_id.is_none() {
        dd.job_id = line
            .split_whitespace()
            .last()
            .filter(|w| is_job_id(w))
            .map(str::to_owned);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statements() {
        let s = numbered_statement("        4 XXCOBOL   EXEC PGM=IGYCRCTL,REGION=0M", 0).unwrap();
        assert_eq!(s.number, 4);
        assert_eq!(s.origin, StatementOrigin::Procedure);
        assert_eq!(s.name.as_deref(), Some("COBOL"));
        assert_eq!(s.operation.as_deref(), Some("EXEC"));

        let s = numbered_statement("        6 XX         DD  DSNAME=CEE.SCEERUN", 0).unwrap();
        assert_eq!(s.name, None);
        assert_eq!(s.operation.as_deref(), Some("DD"));

        let s = numbered_statement("       11 //COBOL.SYSIN DD *", 0).unwrap();
        assert_eq!(s.name.as_deref(), Some("COBOL.SYSIN"));

        assert!(numbered_statement("          //             REGION=65536K", 0).is_none());
        assert!(numbered_statement("          IEFC653I SUBSTITUTION JCL", 0).is_none());
    }

    #[test]
    fn programs() {
        assert_eq!(
            program(&["        4 XXCOBOL   EXEC PGM=IGYCRCTL,REGION=0M"]).as_deref(),
            Some("IGYCRCTL")
        );
        assert_eq!(
            program(&[
                "        5 //STEP1    EXEC PGM=&PROG",
                "          //*        PGM=NOTME",
                "          IEFC653I SUBSTITUTION JCL - PGM=IDCAMS",
            ])
            .as_deref(),
            Some("IDCAMS")
        );
        assert_eq!(program(&["        2 //COBRUN   EXEC IGYWCLG"]), None);
        assert_eq!(pgm_value("EXEC NOTPGM=X,PGM=IEFBR14"), Some("IEFBR14"));
        assert_eq!(pgm_value("EXEC PGM=,X"), None);
    }
}
