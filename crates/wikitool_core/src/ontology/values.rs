//! What an infobox value says once its markup is gone, and whether that
//! parses as the value type its property declares.
//!
//! A value's markup is read with the structural wikitext parser
//! (`mediawiki_wikitext`): a link reads as its label or title, a template,
//! parameter, comment or category reads as nothing, and `<ref>` tags read as
//! nothing. `<br>` separates values. A value made only of templates has no
//! display text that can be known without expanding them, so it is not
//! checked.

use chrono::NaiveDate;
use chrono::format::{Item, Parsed, StrftimeItems, parse};
use mediawiki_wikitext::{
    LinkKind, Node, NodeKind, ProfiledParseInput, ProtectedKind, SourceProfile, parse_profiled,
};

use crate::content_store::parsing::{find_closing_html_tag, parse_open_tag, starts_with_html_tag};

use super::vocabulary::{Term, Vocabulary};

/// A parsing profile for wikitext that Wikitool already holds in memory.
pub(crate) fn wikitext_profile(text: &str) -> SourceProfile {
    SourceProfile::mediawiki_defaults(
        "wikitool-ontology",
        "local",
        u64::try_from(text.len()).unwrap_or(u64::MAX).max(1),
    )
}

/// The display text of a value, split at `<br>`, with blank parts dropped.
/// `None` when the value's markup does not parse.
pub(crate) fn display_parts(raw: &str) -> Option<Vec<String>> {
    let profile = wikitext_profile(raw);
    let document = parse_profiled(ProfiledParseInput {
        source: &profile,
        wikitext: raw,
    })
    .ok()?;
    let mut top: Vec<&Node> = document
        .nodes()
        .iter()
        .filter(|node| node.parent().is_none())
        .collect();
    top.sort_by_key(|node| node.span().start());

    let mut parts = vec![String::new()];
    let bytes = raw.as_bytes();
    let mut pos = 0;
    let mut next = 0;
    while pos < raw.len() {
        while next < top.len() && top[next].span().start() < pos {
            next += 1;
        }
        let next_start = top.get(next).map_or(raw.len(), |node| node.span().start());
        if pos == next_start {
            let node = top[next];
            push_node_text(&document, node, &mut parts);
            pos = node.span().end().max(pos + 1);
            next += 1;
            continue;
        }
        pos = push_text_token(raw, bytes, pos, next_start, &mut parts);
    }
    Some(
        parts
            .into_iter()
            .map(|part| part.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|part| !part.is_empty())
            .collect(),
    )
}

fn push_node_text(document: &mediawiki_wikitext::Document<'_>, node: &Node, parts: &mut [String]) {
    let out = parts.last_mut().expect("at least one part");
    match node.kind() {
        NodeKind::InternalLink(link) => {
            let visible = match link.kind {
                LinkKind::Category | LinkKind::File if !link.leading_colon => return,
                _ => true,
            };
            if !visible {
                return;
            }
            if let Some(label) = link.components.last() {
                let label = document.text(*label);
                let text = display_parts(label).unwrap_or_default().join(" ");
                out.push_str(&text);
            } else {
                out.push_str(document.text(link.target).trim_start_matches(':'));
            }
        }
        NodeKind::Protected(region) if region.kind == ProtectedKind::Nowiki => {
            let text = document.text(node.span());
            let inner = text
                .find('>')
                .map(|open| &text[open + 1..])
                .and_then(|rest| rest.rfind("</").map(|close| &rest[..close]))
                .unwrap_or_default();
            out.push_str(inner);
        }
        // Templates and parameters render to text that is unknown without
        // expanding them; comments and literal blocks show nothing.
        _ => {}
    }
}

/// Consume one token of plain text at `pos`, before `limit` (the next node),
/// and return where the next token starts.
fn push_text_token(
    raw: &str,
    bytes: &[u8],
    pos: usize,
    limit: usize,
    parts: &mut Vec<String>,
) -> usize {
    let byte = bytes[pos];
    if byte == b'<' {
        if let Some((after_open, _, self_closing)) = parse_open_tag(raw, pos, "ref") {
            if self_closing {
                return after_open;
            }
            return find_closing_html_tag(raw, after_open, "ref").map_or(raw.len(), |(_, end)| end);
        }
        if starts_with_html_tag(bytes, pos, "br") || starts_with_html_tag(bytes, pos, "/br") {
            parts.push(String::new());
            return tag_end(bytes, pos, limit).unwrap_or(pos + 1);
        }
        let opens_tag = bytes
            .get(pos + 1)
            .is_some_and(|next| next.is_ascii_alphabetic() || *next == b'/');
        if opens_tag && let Some(end) = tag_end(bytes, pos, limit) {
            return end;
        }
    }
    if byte == b'\'' {
        let run = bytes[pos..limit]
            .iter()
            .take_while(|byte| **byte == b'\'')
            .count();
        if run >= 2 {
            if run == 4 {
                parts.last_mut().expect("part").push('\'');
            }
            return pos + run;
        }
    }
    if byte == b'['
        && let Some((label, end)) = external_link(raw, pos, limit)
    {
        parts.last_mut().expect("part").push_str(&label);
        return end;
    }
    if byte == b'&'
        && let Some((decoded, end)) = entity(raw, pos, limit)
    {
        parts.last_mut().expect("part").push_str(&decoded);
        return end;
    }
    let ch = raw[pos..].chars().next().expect("char at a boundary");
    parts.last_mut().expect("part").push(ch);
    pos + ch.len_utf8()
}

/// The end of an HTML tag that starts at `pos`, if it closes before `limit`.
fn tag_end(bytes: &[u8], pos: usize, limit: usize) -> Option<usize> {
    bytes[pos..limit]
        .iter()
        .position(|byte| *byte == b'>')
        .map(|offset| pos + offset + 1)
}

/// `[url label]`: the label, or the URL when there is none.
fn external_link(raw: &str, pos: usize, limit: usize) -> Option<(String, usize)> {
    let rest = &raw[pos + 1..limit];
    let lower = rest.to_ascii_lowercase();
    if !["http://", "https://", "//", "ftp://", "mailto:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme))
    {
        return None;
    }
    let close = rest.find(']')?;
    if rest[..close].contains('\n') {
        return None;
    }
    let inner = &rest[..close];
    let label = match inner.split_once(char::is_whitespace) {
        Some((_, label)) => display_parts(label).unwrap_or_default().join(" "),
        None => inner.to_string(),
    };
    Some((label, pos + 1 + close + 1))
}

/// A character reference such as `&nbsp;` or `&#8211;`.
fn entity(raw: &str, pos: usize, limit: usize) -> Option<(String, usize)> {
    let rest = &raw[pos + 1..limit];
    let end = rest.find(';')?;
    let name = &rest[..end];
    if name.is_empty() || name.len() > 10 {
        return None;
    }
    let decoded = match name {
        "nbsp" | "ensp" | "emsp" | "thinsp" => ' ',
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "ndash" => '\u{2013}',
        "mdash" => '\u{2014}',
        "minus" => '\u{2212}',
        _ => {
            let code =
                if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                    u32::from_str_radix(hex, 16).ok()?
                } else {
                    name.strip_prefix('#')?.parse().ok()?
                };
            char::from_u32(code)?
        }
    };
    Some((decoded.to_string(), pos + 1 + end + 1))
}

/// Whether a value parses as its property's value type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Verdict {
    Valid,
    /// Why the value does not parse, completing "'value' ...".
    Invalid(String),
    /// The value type is not one the lint checks.
    Unchecked,
}

pub(crate) fn check_value(
    value: &str,
    property: &Term<'_>,
    vocabulary: &Vocabulary,
    date_formats: &[String],
) -> Verdict {
    match property.str_field("valueType") {
        Some("text") => Verdict::Valid,
        Some("date") => {
            if is_iso_date(value)
                || date_formats
                    .iter()
                    .any(|format| matches_date(value, format))
            {
                Verdict::Valid
            } else if date_formats.is_empty() {
                Verdict::Invalid(
                    "is not an ISO 8601 date (2024, 2024-05 or 2024-05-12)".to_string(),
                )
            } else {
                Verdict::Invalid(format!(
                    "is not an ISO 8601 date (2024, 2024-05 or 2024-05-12) or a date in the formats {}",
                    date_formats.join(", ")
                ))
            }
        }
        Some("quantity") => check_quantity(value, property.ids("allowedUnits")),
        Some("controlled-term") => {
            let Some(scheme) = property.str_field("valueScheme") else {
                return Verdict::Unchecked;
            };
            let Some(members) = vocabulary.scheme_members(scheme) else {
                return Verdict::Unchecked;
            };
            let found = members.iter().any(|(id, member)| {
                id == value
                    || member
                        .get("label")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|label| label.eq_ignore_ascii_case(value))
            });
            if found {
                Verdict::Valid
            } else {
                Verdict::Invalid(format!("is not a member of the value list {scheme}"))
            }
        }
        Some("external-id") => {
            if value.is_empty() || value.chars().any(char::is_whitespace) {
                Verdict::Invalid("is not an identifier: it contains a space".to_string())
            } else {
                Verdict::Valid
            }
        }
        _ => Verdict::Unchecked,
    }
}

/// `2024`, `2024-05` or `2024-05-12`, a real calendar date.
pub(crate) fn is_iso_date(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    let digits =
        |part: &str, len: usize| part.len() == len && part.bytes().all(|b| b.is_ascii_digit());
    match parts.as_slice() {
        [year] => digits(year, 4),
        [year, month] => {
            digits(year, 4)
                && digits(month, 2)
                && (1..=12).contains(&month.parse::<u32>().unwrap_or(0))
        }
        [year, month, day] => {
            digits(year, 4)
                && digits(month, 2)
                && digits(day, 2)
                && NaiveDate::from_ymd_opt(
                    year.parse().unwrap_or(0),
                    month.parse().unwrap_or(0),
                    day.parse().unwrap_or(0),
                )
                .is_some()
        }
        _ => false,
    }
}

/// Whether a strftime format is one chrono can parse with.
pub(crate) fn is_valid_date_format(format: &str) -> bool {
    !format.trim().is_empty() && StrftimeItems::new(format).all(|item| !matches!(item, Item::Error))
}

/// Whether a value matches a strftime date format and names a real year,
/// month or day.
pub(crate) fn matches_date(value: &str, format: &str) -> bool {
    let mut parsed = Parsed::new();
    if parse(&mut parsed, value, StrftimeItems::new(format)).is_err() {
        return false;
    }
    match (parsed.year(), parsed.month(), parsed.day()) {
        (Some(year), Some(month), Some(day)) => NaiveDate::from_ymd_opt(year, month, day).is_some(),
        (Some(_), Some(_), None) | (Some(_), None, None) => true,
        _ => false,
    }
}

fn check_quantity(value: &str, allowed_units: Option<Vec<&str>>) -> Verdict {
    let Some(rest) = strip_number(value) else {
        return Verdict::Invalid("is not a number".to_string());
    };
    let unit = rest.trim();
    if unit.is_empty() {
        return Verdict::Valid;
    }
    let units: Vec<&str> = allowed_units
        .unwrap_or_default()
        .into_iter()
        .filter(|unit| *unit != "1")
        .collect();
    if units.iter().any(|code| unit_matches(code, unit)) {
        return Verdict::Valid;
    }
    if units.is_empty() {
        Verdict::Invalid(format!(
            "has the unit {unit:?}, but the property takes none"
        ))
    } else {
        Verdict::Invalid(format!(
            "has the unit {unit:?}, which is not one of {}",
            units.join(", ")
        ))
    }
}

/// The text after a leading decimal number, such as `1,024.5`, or `None`
/// when the value does not start with one.
fn strip_number(value: &str) -> Option<&str> {
    let bytes = value.as_bytes();
    let mut index = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        index += 1;
    }
    let digits_start = index;
    let mut group = 0;
    let mut grouped = false;
    while index < bytes.len() {
        match bytes[index] {
            b'0'..=b'9' => {
                group += 1;
                index += 1;
            }
            // A thousands separator must follow one to three digits (or a
            // full group) and precede exactly three digits.
            b',' if group > 0
                && (if grouped { group == 3 } else { group <= 3 })
                && bytes
                    .get(index + 1..index + 4)
                    .is_some_and(|next| next.iter().all(u8::is_ascii_digit))
                && !bytes.get(index + 4).is_some_and(u8::is_ascii_digit) =>
            {
                grouped = true;
                group = 0;
                index += 1;
            }
            _ => break,
        }
    }
    if index == digits_start {
        return None;
    }
    if bytes.get(index) == Some(&b'.') && bytes.get(index + 1).is_some_and(u8::is_ascii_digit) {
        index += 1;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
    }
    Some(&value[index..])
}

/// Whether a written unit is the UCUM code an ontology lists, or a common
/// way of writing it (`MB` for `MBy`, `in` for `[in_i]`, `px` for `{px}`).
fn unit_matches(code: &str, written: &str) -> bool {
    if code == written {
        return true;
    }
    let aliases: &[&str] = match code {
        "By" => &["B", "byte", "bytes"],
        "kBy" => &["kB", "KB"],
        "MBy" => &["MB"],
        "GBy" => &["GB"],
        "TBy" => &["TB"],
        "bit" => &["bits"],
        "Cel" => &["°C", "\u{2103}"],
        "[in_i]" => &["in", "inch", "inches", "\u{2033}"],
        "[ft_i]" => &["ft", "foot", "feet"],
        "[mi_i]" => &["mi", "mile", "miles"],
        "[lb_av]" => &["lb", "lbs", "pound", "pounds"],
        "[lb_av].[ft_i]" => &["lb\u{b7}ft", "lb-ft", "lb ft", "ft\u{b7}lb", "ft-lb"],
        "N.m" => &["N\u{b7}m", "N m", "Nm"],
        "a" => &["yr", "year", "years"],
        "h" => &["hr", "hour", "hours"],
        "min" => &["mins", "minute", "minutes"],
        "s" => &["sec", "second", "seconds"],
        "ms" => &["msec"],
        "L" => &["l", "liter", "liters", "litre", "litres"],
        "mL" => &["ml"],
        "cm3" => &["cm\u{b3}", "cc"],
        "g/cm3" => &["g/cm\u{b3}"],
        "g/m2" => &["g/m\u{b2}", "gsm"],
        "g/m3" => &["g/m\u{b3}"],
        "{px}" => &["px", "pixels"],
        _ => &[],
    };
    aliases.contains(&written)
}
#[cfg(test)]
mod tests {
    use super::{display_parts, is_iso_date, matches_date, strip_number};

    #[test]
    fn display_text_drops_markup_and_keeps_what_a_reader_sees() {
        for (raw, expected) in [
            ("[[Ethereum]]", vec!["Ethereum"]),
            ("[[Ethereum|ETH]] chain", vec!["ETH chain"]),
            (
                "2021<ref name=\"a\">{{Cite web|title=x}}</ref>",
                vec!["2021"],
            ),
            ("2021<ref name=\"a\" />", vec!["2021"]),
            ("{{Start date|2021|5|1}}", vec![]),
            ("A<br />B<br>C", vec!["A", "B", "C"]),
            ("'''Bold''' and ''it''", vec!["Bold and it"]),
            ("<!-- note -->10,000", vec!["10,000"]),
            ("[https://example.com Example site]", vec!["Example site"]),
            ("<small>3.2 GHz</small>", vec!["3.2 GHz"]),
            ("5&nbsp;MB", vec!["5 MB"]),
            ("[[Category:Things]]Value", vec!["Value"]),
            ("<nowiki>[[x]]</nowiki>", vec!["[[x]]"]),
        ] {
            assert_eq!(display_parts(raw).expect(raw), expected, "{raw}");
        }
    }

    #[test]
    fn iso_dates_are_years_months_or_real_days() {
        for valid in ["2024", "2024-05", "2024-02-29"] {
            assert!(is_iso_date(valid), "{valid}");
        }
        for invalid in [
            "24",
            "2024-13",
            "2023-02-29",
            "2024-5",
            "May 2024",
            "2024-05-12T00:00",
        ] {
            assert!(!is_iso_date(invalid), "{invalid}");
        }
    }

    #[test]
    fn human_dates_follow_the_listed_formats() {
        assert!(matches_date("May 12, 2019", "%B %-d, %Y"));
        assert!(matches_date("May 2019", "%B %Y"));
        assert!(!matches_date("May 32, 2019", "%B %-d, %Y"));
        assert!(!matches_date("12 May 2019", "%B %-d, %Y"));
    }

    #[test]
    fn quantities_accept_ucum_codes_and_their_common_spellings() {
        use super::{Verdict, check_quantity};
        assert_eq!(check_quantity("5 MB", Some(vec!["MBy"])), Verdict::Valid);
        assert_eq!(check_quantity("5 MBy", Some(vec!["MBy"])), Verdict::Valid);
        assert_eq!(
            check_quantity("12 in", Some(vec!["[in_i]"])),
            Verdict::Valid
        );
        assert_eq!(
            check_quantity("3.2 GHz", Some(vec!["Hz", "GHz"])),
            Verdict::Valid
        );
        assert_eq!(check_quantity("10,000", Some(vec!["1"])), Verdict::Valid);
        assert!(matches!(
            check_quantity("5 MB", Some(vec!["GBy"])),
            Verdict::Invalid(_)
        ));
        assert!(matches!(check_quantity("5000+", None), Verdict::Invalid(_)));
    }

    #[test]
    fn numbers_allow_thousands_groups_and_decimals() {
        assert_eq!(strip_number("10,000"), Some(""));
        assert_eq!(strip_number("1,234,567.5 Hz"), Some(" Hz"));
        assert_eq!(strip_number("-3.2GHz"), Some("GHz"));
        assert_eq!(strip_number("12,34"), Some(",34"));
        assert_eq!(strip_number("about 5"), None);
    }
}
