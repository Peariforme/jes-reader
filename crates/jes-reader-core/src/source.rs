//! Identifies a DD from its path or name, falling back on its content.
//!
//! Recognized path forms (see `docs/FORMAT.md`):
//!
//! - Zowe Explorer download: `…/<jobid>/JES2/<DD>.txt`,
//!   `…/<jobid>/<step>/<DD>.txt`, `…/<jobid>/<procstep>/<step>/<DD>.txt`,
//!   sometimes with a `(1)` suffix before the extension;
//! - document opened by Zowe without downloading:
//!   `<jobname>.<jobid>.<step>.<DD>.<n>`.

use crate::model::DdKind;

/// What a document's path tells about the DD it contains.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DdSource {
    /// JES job ID (`JOB00823`).
    pub job_id: Option<String>,
    /// Job name (`<jobname>.<jobid>…` form only).
    pub job_name: Option<String>,
    /// DD name.
    pub dd_name: Option<String>,
}

impl DdSource {
    /// Reads a path or URI. Never fails: whatever is not recognized stays
    /// `None`.
    pub fn from_path(path: &str) -> Self {
        let components: Vec<&str> = path.split(['/', '\\']).filter(|c| !c.is_empty()).collect();
        let Some(file_name) = components.last() else {
            return Self::default();
        };

        // Zowe document, not downloaded: JRIDC12.JOB00922.JES2.JESMSGLG.2
        let segments: Vec<&str> = file_name.split('.').collect();
        if let Some(pos) = segments.iter().position(|s| is_job_id(s)) {
            let rest = &segments[pos + 1..];
            // <step>… <DD> <n>: the DD is the second-to-last segment.
            if rest.len() >= 3 && rest.last().is_some_and(|n| is_number(n)) {
                return Self {
                    job_id: Some(segments[pos].to_owned()),
                    job_name: (pos > 0).then(|| segments[pos - 1].to_owned()),
                    dd_name: Some(rest[rest.len() - 2].to_owned()),
                };
            }
        }

        // Whole spool saved by BMC AMI DevX: JRIDC8_JOB00868.jes
        let stem = file_name
            .rsplit_once('.')
            .map_or(*file_name, |(stem, _)| stem);
        if let Some((job_name, job_id)) = stem.rsplit_once('_')
            && is_job_id(job_id)
            && !job_name.is_empty()
            && job_name.len() <= 8
        {
            return Self {
                job_id: Some(job_id.to_owned()),
                job_name: Some(job_name.to_owned()),
                dd_name: None,
            };
        }

        // Downloaded job: …/JOB00823/JES2/JESMSGLG.txt
        let job_id = components[..components.len() - 1]
            .iter()
            .rev()
            .find(|c| is_job_id(c))
            .map(|c| (*c).to_owned());
        let stem = strip_copy_suffix(stem);
        let dd_name = is_dd_name(stem).then(|| stem.to_owned());
        Self {
            job_id,
            job_name: None,
            dd_name,
        }
    }
}

impl DdKind {
    /// Kind of a DD from its name.
    pub fn from_dd_name(name: &str) -> Self {
        match name {
            "JESMSGLG" => Self::JesMsgLg,
            "JESJCL" => Self::JesJcl,
            "JESYSMSG" => Self::JesYsMsg,
            _ => Self::Other,
        }
    }

    /// Recognizes a JES2 system DD from its first non-empty line.
    pub fn sniff(text: &str) -> Option<Self> {
        let first = text.lines().find(|l| !l.trim().is_empty())?;
        if first.contains("J E S 2  J O B  L O G") {
            return Some(Self::JesMsgLg);
        }
        if first.trim_start().starts_with("STMT NO. MESSAGE") {
            return Some(Self::JesYsMsg);
        }
        // `        1 //NAME     JOB …`: number in column 9, `//` in column 11;
        // one column to the left without the ASA column, one to the right
        // with an extra column (BMC AMI DevX).
        let card = |number: Option<&str>, slashes: Option<&str>| {
            number.map(str::trim) == Some("1") && slashes == Some("//")
        };
        if (card(first.get(1..9), first.get(10..12))
            || card(first.get(..8), first.get(9..11))
            || card(first.get(2..10), first.get(11..13)))
            && first.contains(" JOB ")
        {
            return Some(Self::JesJcl);
        }
        None
    }
}

/// Standard DD name of a JES2 system DD.
pub(crate) fn jes_dd_name(kind: DdKind) -> Option<&'static str> {
    match kind {
        DdKind::JesMsgLg => Some("JESMSGLG"),
        DdKind::JesJcl => Some("JESJCL"),
        DdKind::JesYsMsg => Some("JESYSMSG"),
        DdKind::Other => None,
    }
}

/// `JOB00823`, `TSU00812`, `STC01234`, or the 7-digit form `J0123456`.
pub fn is_job_id(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 8 {
        return false;
    }
    let five_digits =
        matches!(&b[..3], b"JOB" | b"TSU" | b"STC") && b[3..].iter().all(u8::is_ascii_digit);
    let seven_digits = matches!(b[0], b'J' | b'T' | b'S') && b[1..].iter().all(u8::is_ascii_digit);
    five_digits || seven_digits
}

fn is_number(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit())
}

/// Removes the `(1)` suffix Zowe sometimes adds to file names.
fn strip_copy_suffix(stem: &str) -> &str {
    if let Some(open) = stem.rfind('(')
        && stem.ends_with(')')
        && is_number(&stem[open + 1..stem.len() - 1])
    {
        return &stem[..open];
    }
    stem
}

/// Valid DD name: 1 to 8 characters, starting with a letter or national
/// character.
fn is_dd_name(s: &str) -> bool {
    let b = s.as_bytes();
    (1..=8).contains(&b.len())
        && (b[0].is_ascii_uppercase() || matches!(b[0], b'$' | b'#' | b'@'))
        && b.iter().all(|c| {
            c.is_ascii_uppercase() || c.is_ascii_digit() || matches!(c, b'$' | b'#' | b'@')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zowe_download() {
        let src = DdSource::from_path(r"C:\corpus\sysout\JOB00823\JES2\JESMSGLG.txt");
        assert_eq!(src.job_id.as_deref(), Some("JOB00823"));
        assert_eq!(src.dd_name.as_deref(), Some("JESMSGLG"));

        let src = DdSource::from_path("corpus/sysout/JOB00896/LKED/COBRUN/SYSPRINT(1).txt");
        assert_eq!(src.job_id.as_deref(), Some("JOB00896"));
        assert_eq!(src.dd_name.as_deref(), Some("SYSPRINT"));
    }

    #[test]
    fn devx_spool() {
        let src = DdSource::from_path(
            r"c:\Users\me\.devx-workbench\templocaldatasetsfiles\1-2\SYSA\JRIDC8_JOB00868.jes",
        );
        assert_eq!(src.job_id.as_deref(), Some("JOB00868"));
        assert_eq!(src.job_name.as_deref(), Some("JRIDC8"));
        assert_eq!(src.dd_name, None);
        // Not a job ID after the underscore.
        assert_eq!(DdSource::from_path("MY_FILE.txt").job_id, None);
    }

    #[test]
    fn zowe_document() {
        let src = DdSource::from_path("/zosmf/JOB00922/JRIDC12.JOB00922.JES2.JESMSGLG.2");
        assert_eq!(src.job_id.as_deref(), Some("JOB00922"));
        assert_eq!(src.job_name.as_deref(), Some("JRIDC12"));
        assert_eq!(src.dd_name.as_deref(), Some("JESMSGLG"));

        let src = DdSource::from_path("Z99999.TSU00812.JES2.JESYSMSG.4");
        assert_eq!(src.job_id.as_deref(), Some("TSU00812"));
        assert_eq!(src.dd_name.as_deref(), Some("JESYSMSG"));
    }

    #[test]
    fn unknown_path() {
        let src = DdSource::from_path("final report.txt");
        assert_eq!(src, DdSource::default());
    }

    #[test]
    fn jes_job_ids() {
        for id in ["JOB00823", "TSU00812", "STC01234", "J0123456"] {
            assert!(is_job_id(id), "{id}");
        }
        for id in ["JOB0082", "STEP0100", "JES2", "JOBABCDE"] {
            assert!(!is_job_id(id), "{id}");
        }
    }

    #[test]
    fn content_detection() {
        let msglg = "1                   J E S 2  J O B  L O G  --  S Y S T E M  S0W1\r\n";
        assert_eq!(DdKind::sniff(msglg), Some(DdKind::JesMsgLg));
        assert_eq!(
            DdKind::sniff("  STMT NO. MESSAGE\n"),
            Some(DdKind::JesYsMsg)
        );
        assert_eq!(
            DdKind::sniff("        1 //@VSC1     JOB ,MSGLEVEL=(0,0)\n"),
            Some(DdKind::JesJcl)
        );
        assert_eq!(DdKind::sniff("1READY\n"), None);
    }
}
