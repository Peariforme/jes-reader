//! JESJCL : JCL développé, instructions numérotées.
//!
//! ```text
//!         1 //JRS0C7   JOB (ACCT),'JES-READER S0C7',CLASS=A,          JOB00896
//!           //             MSGLEVEL=(1,1),NOTIFY=&SYSUID
//!           IEFC653I SUBSTITUTION JCL - (ACCT),'JES-READER S0C7',…
//!         4 XXCOBOL   EXEC PGM=IGYCRCTL,REGION=0M
//!           X/SYSIN    DD  DSNAME=&SYSUID..CBL(&SRC),DISP=SHR
//! ```
//!
//! Numéro cadré en col. 2-9, instruction à partir de la col. 11 (offset 10),
//! précédée de `//`, `XX`, `++`, `X/` ou `+/`. Les lignes sans numéro
//! prolongent l'instruction précédente ; `IEFC653I` est un message.

use super::Messages;
use crate::model::{Dd, Statement, StatementOrigin};
use crate::msgid::message_id_at;
use crate::source::is_job_id;

/// Offset du début de l'instruction (préfixe `//` compris).
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

    dd.messages = messages.finish();
    dd.statements = statements;
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
    })
}

/// La carte JOB donne le nom du job et, en fin de ligne, l'identifiant JES.
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
    fn instructions() {
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
}
