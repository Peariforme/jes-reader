//! Reconnaissance des identifiants de message IBM.
//!
//! Forme générale : 3 à 5 lettres de composant, 3 à 5 chiffres, une lettre
//! de type (`IEF142I`, `IEFA107I`, `IGD17101I`, `IGZ0035S`), ou
//! `$HASP` suivi de 3 ou 4 chiffres. Le parser ne cherche un identifiant
//! qu'aux positions où la DD en attend un, ce qui limite les faux positifs
//! (`STEP010`, `IGG0191B` et `IEFBR14` sont rejetés).

use std::ops::Range;

/// Lettres finales acceptées : information, avertissement, erreur, erreur
/// grave, action, décision, arrêt.
const TYPE_LETTERS: &[u8] = b"IWESADT";

/// Vrai si `token` a la forme d'un identifiant de message.
pub fn is_message_id(token: &str) -> bool {
    let b = token.as_bytes();
    if let Some(digits) = b.strip_prefix(b"$HASP") {
        return (3..=4).contains(&digits.len()) && digits.iter().all(u8::is_ascii_digit);
    }
    let letters = b.iter().take_while(|c| c.is_ascii_uppercase()).count();
    let digits = b[letters..]
        .iter()
        .take_while(|c| c.is_ascii_digit())
        .count();
    (3..=5).contains(&letters)
        && (3..=5).contains(&digits)
        && b.len() == letters + digits + 1
        && TYPE_LETTERS.contains(&b[b.len() - 1])
}

/// Colonnes d'un identifiant de message qui commence exactement en `start`
/// et se termine par une espace ou la fin de ligne.
pub fn message_id_at(line: &str, start: usize) -> Option<Range<usize>> {
    let rest = line.get(start..)?;
    let len = rest.find(' ').unwrap_or(rest.len());
    is_message_id(&rest[..len]).then_some(start..start + len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiants_valides() {
        for id in [
            "IEF142I",
            "IEFA107I",
            "IEFC001I",
            "IGD17101I",
            "ICH70001I",
            "IRR010I",
            "CSV003I",
            "IEC141I",
            "IGZ0035S",
            "IDC3012I",
            "CEE3207S",
            "DSNT408I",
            "$HASP373",
            "$HASP0395",
        ] {
            assert!(is_message_id(id), "{id}");
        }
    }

    #[test]
    fn identifiants_rejetes() {
        for token in [
            "STEP010",
            "IGG0191B",
            "IEFBR14",
            "JOB00823",
            "IEF142",
            "IE142I",
            "IEFABC12I",
            "$HASP",
            "$HASP12",
            "READY",
            "",
        ] {
            assert!(!is_message_id(token), "{token}");
        }
    }

    #[test]
    fn position() {
        let line = " IEF142I JRS0C7 COBOL COBRUN - STEP WAS EXECUTED";
        assert_eq!(message_id_at(line, 1), Some(1..8));
        assert_eq!(message_id_at(line, 0), None);
        assert_eq!(message_id_at(line, 9), None);
        assert_eq!(message_id_at(" IGZ0035S", 1), Some(1..9));
        assert_eq!(message_id_at("court", 40), None);
    }
}
