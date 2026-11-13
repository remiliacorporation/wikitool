//! A line parser for W3C RDF 1.1 N-Triples, enough to read the labels and
//! definitions of terms from a pinned extract.
//!
//! The grammar is small: a subject (IRI or blank node), a predicate (IRI), an
//! object (IRI, blank node or literal with an optional language tag or
//! datatype IRI), a terminating `.`, and an optional comment. Strings decode
//! the N-Triples escapes (`ECHAR` and `UCHAR`), which are not JSON escapes:
//! `\U0001F600` is valid here and invalid JSON.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Term {
    Iri(String),
    Blank(String),
    Literal(Literal),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Literal {
    pub(super) value: String,
    /// The language tag as written, without the `@`.
    pub(super) language: Option<String>,
    pub(super) datatype: Option<String>,
}

/// One line of an N-Triples document. Read the subject first, then the rest
/// only when the subject matters, so a caller can skip lines cheaply while
/// still failing on a malformed line about a term it wants.
pub(super) struct LineParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> LineParser<'a> {
    pub(super) fn new(line: &'a str) -> Self {
        Self {
            input: line,
            pos: 0,
        }
    }

    /// The subject, or `None` for a blank or comment-only line.
    pub(super) fn subject(&mut self) -> Result<Option<Term>, String> {
        self.skip_space();
        if self.at_end_or_comment() {
            return Ok(None);
        }
        match self.peek() {
            Some('<') => Ok(Some(Term::Iri(self.iri()?))),
            Some('_') => Ok(Some(Term::Blank(self.blank_node()?))),
            _ => Err(self.unexpected("a subject IRI or blank node")),
        }
    }

    /// The predicate and object, consuming the terminating `.` and anything
    /// after it, which may only be space or a comment.
    pub(super) fn predicate_object(&mut self) -> Result<(String, Term), String> {
        self.skip_space();
        if self.peek() != Some('<') {
            return Err(self.unexpected("a predicate IRI"));
        }
        let predicate = self.iri()?;
        self.skip_space();
        let object = match self.peek() {
            Some('<') => Term::Iri(self.iri()?),
            Some('_') => Term::Blank(self.blank_node()?),
            Some('"') => Term::Literal(self.literal()?),
            _ => return Err(self.unexpected("an object IRI, blank node or literal")),
        };
        self.skip_space();
        if self.bump() != Some('.') {
            return Err("the triple does not end with `.`".to_string());
        }
        self.skip_space();
        if !self.at_end_or_comment() {
            return Err(self.unexpected("the end of the line"));
        }
        Ok((predicate, object))
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
    }

    fn at_end_or_comment(&self) -> bool {
        matches!(self.peek(), None | Some('#'))
    }

    fn unexpected(&self, wanted: &str) -> String {
        match self.peek() {
            Some(ch) => format!("expected {wanted} at column {}, found {ch:?}", self.pos + 1),
            None => format!("expected {wanted}, found the end of the line"),
        }
    }

    /// `IRIREF ::= '<' ([^#x00-#x20<>"{}|^`\] | UCHAR)* '>'`
    fn iri(&mut self) -> Result<String, String> {
        self.bump();
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err("an IRI is not closed with `>`".to_string()),
                Some('>') => return Ok(out),
                Some('\\') => out.push(self.uchar()?),
                Some(ch) if ch <= ' ' || matches!(ch, '<' | '"' | '{' | '}' | '|' | '^' | '`') => {
                    return Err(format!("{ch:?} is not allowed in an IRI"));
                }
                Some(ch) => out.push(ch),
            }
        }
    }

    /// `UCHAR ::= '\u' HEX HEX HEX HEX | '\U' HEX HEX HEX HEX HEX HEX HEX HEX`,
    /// read after the backslash.
    fn uchar(&mut self) -> Result<char, String> {
        let digits = match self.bump() {
            Some('u') => 4,
            Some('U') => 8,
            Some(ch) => return Err(format!("`\\{ch}` is not an N-Triples escape here")),
            None => return Err("a `\\` escape is cut off".to_string()),
        };
        let mut value: u32 = 0;
        for _ in 0..digits {
            let digit = self
                .bump()
                .and_then(|ch| ch.to_digit(16))
                .ok_or_else(|| format!("a `\\u` escape needs {digits} hex digits"))?;
            value = value * 16 + digit;
        }
        char::from_u32(value).ok_or_else(|| format!("U+{value:X} is not a Unicode scalar value"))
    }

    /// `STRING_LITERAL_QUOTE ::= '"' ([^#x22#x5C#xA#xD] | ECHAR | UCHAR)* '"'`
    /// followed by an optional `LANGTAG` or `'^^' IRIREF`.
    fn literal(&mut self) -> Result<Literal, String> {
        self.bump();
        let mut value = String::new();
        loop {
            match self.bump() {
                None => return Err("a string is not closed with `\"`".to_string()),
                Some('"') => break,
                Some('\\') => match self.peek() {
                    Some('u' | 'U') => value.push(self.uchar()?),
                    Some(ch) => {
                        self.bump();
                        value.push(match ch {
                            't' => '\t',
                            'b' => '\u{8}',
                            'n' => '\n',
                            'r' => '\r',
                            'f' => '\u{c}',
                            '"' => '"',
                            '\'' => '\'',
                            '\\' => '\\',
                            other => {
                                return Err(format!("`\\{other}` is not an N-Triples escape"));
                            }
                        });
                    }
                    None => return Err("a `\\` escape is cut off".to_string()),
                },
                Some('\r' | '\n') => {
                    return Err("a string may not contain a raw line break".to_string());
                }
                Some(ch) => value.push(ch),
            }
        }
        let mut literal = Literal {
            value,
            language: None,
            datatype: None,
        };
        match self.peek() {
            Some('@') => {
                self.bump();
                literal.language = Some(self.language_tag()?);
            }
            Some('^') => {
                self.bump();
                if self.bump() != Some('^') || self.peek() != Some('<') {
                    return Err("a datatype is written `^^<iri>`".to_string());
                }
                literal.datatype = Some(self.iri()?);
            }
            _ => {}
        }
        Ok(literal)
    }

    /// `LANGTAG ::= '@' [a-zA-Z]+ ('-' [a-zA-Z0-9]+)*`, read after the `@`.
    fn language_tag(&mut self) -> Result<String, String> {
        let start = self.pos;
        let mut subtag_start = true;
        let mut primary = true;
        loop {
            match self.peek() {
                Some(ch) if ch.is_ascii_alphabetic() || (!primary && ch.is_ascii_digit()) => {
                    self.bump();
                    subtag_start = false;
                }
                Some('-') if !subtag_start => {
                    self.bump();
                    subtag_start = true;
                    primary = false;
                }
                _ => break,
            }
        }
        if subtag_start {
            return Err(format!(
                "`@{}` is not a language tag",
                &self.input[start..self.pos]
            ));
        }
        Ok(self.input[start..self.pos].to_string())
    }

    /// `BLANK_NODE_LABEL ::= '_:' (PN_CHARS_U | [0-9]) ((PN_CHARS | '.')* PN_CHARS)?`
    ///
    /// Characters are checked loosely (any letter, digit, `_`, `-`, `.` or
    /// non-ASCII character); blank nodes are skipped, never exported.
    fn blank_node(&mut self) -> Result<String, String> {
        self.bump();
        if self.bump() != Some(':') {
            return Err("a blank node starts with `_:`".to_string());
        }
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || matches!(ch, '_' | '-' | '.') || !ch.is_ascii() {
                self.bump();
            } else {
                break;
            }
        }
        // A label cannot end with `.`; a trailing one ends the triple.
        while self.pos > start && self.input[..self.pos].ends_with('.') {
            self.pos -= 1;
        }
        let label = &self.input[start..self.pos];
        if label.is_empty() || label.starts_with(['-', '.']) {
            return Err("a blank node needs a label after `_:`".to_string());
        }
        Ok(label.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(line: &str) -> Result<Option<(Term, String, Term)>, String> {
        let mut parser = LineParser::new(line);
        let Some(subject) = parser.subject()? else {
            return Ok(None);
        };
        let (predicate, object) = parser.predicate_object()?;
        Ok(Some((subject, predicate, object)))
    }

    fn literal(line: &str) -> Literal {
        match parse(line).expect("parses").expect("a triple").2 {
            Term::Literal(literal) => literal,
            other => panic!("not a literal: {other:?}"),
        }
    }

    fn iri(value: &str) -> Term {
        Term::Iri(value.to_string())
    }

    #[test]
    fn reads_iri_blank_and_literal_terms() {
        assert_eq!(
            parse("<http://a/s> <http://a/p> <http://a/o> .").unwrap(),
            Some((iri("http://a/s"), "http://a/p".into(), iri("http://a/o")))
        );
        assert_eq!(
            parse("_:b1 <http://a/p> _:b2.").unwrap(),
            Some((
                Term::Blank("b1".into()),
                "http://a/p".into(),
                Term::Blank("b2".into())
            ))
        );
        // Space between terms is optional where tokens cannot run together.
        assert_eq!(
            parse("<http://a/s><http://a/p>\"x\".").unwrap().unwrap().2,
            Term::Literal(Literal {
                value: "x".into(),
                language: None,
                datatype: None
            })
        );
    }

    #[test]
    fn blank_and_comment_lines_have_no_triple() {
        assert_eq!(parse("").unwrap(), None);
        assert_eq!(parse("   \t").unwrap(), None);
        assert_eq!(parse("# a comment <http://a/s>").unwrap(), None);
        assert!(
            parse("<http://a/s> <http://a/p> \"x\" . # trailing")
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn decodes_echar_and_uchar_escapes() {
        let decoded = literal(
            r#"<http://a/s> <http://a/p> "tab\t nl\n cr\r bs\b ff\f q\" a\' sl\\ é \U0001F600" ."#,
        );
        assert_eq!(
            decoded.value,
            "tab\t nl\n cr\r bs\u{8} ff\u{c} q\" a' sl\\ \u{e9} \u{1F600}"
        );
        // UCHAR is also decoded inside IRIs.
        assert_eq!(
            parse(r"<http://a/s> <http://a/p> <http://a/o> .")
                .unwrap()
                .unwrap()
                .0,
            iri("http://a/s")
        );
    }

    #[test]
    fn reads_language_tags_and_datatypes() {
        let tagged = literal(r#"<http://a/s> <http://a/p> "colour"@en-GB ."#);
        assert_eq!(tagged.language.as_deref(), Some("en-GB"));
        assert_eq!(tagged.datatype, None);

        let typed = literal(
            r#"<http://a/s> <http://a/p> "say \"hi\""^^<http://www.w3.org/2001/XMLSchema#string> ."#,
        );
        assert_eq!(typed.value, "say \"hi\"");
        assert_eq!(
            typed.datatype.as_deref(),
            Some("http://www.w3.org/2001/XMLSchema#string")
        );
        assert_eq!(typed.language, None);
    }

    #[test]
    fn rejects_malformed_lines() {
        for (line, why) in [
            (r#"<http://a/s> <http://a/p> "x" "#, "no terminating dot"),
            (r#"<http://a/s> <http://a/p> "x"#, "unclosed string"),
            (r#"<http://a/s> <http://a/p> "\x" ."#, "JSON-only escape"),
            (r#"<http://a/s> <http://a/p> "\u12" ."#, "short UCHAR"),
            (
                r#"<http://a/s> <http://a/p> "\uD800" ."#,
                "surrogate code point",
            ),
            (r#"<http://a/s> <http://a/p> "x"@ ."#, "empty language tag"),
            (
                r#"<http://a/s> <http://a/p> "x"@1a ."#,
                "digit in primary subtag",
            ),
            (r#"<http://a/s> <http://a/p> "x"@en- ."#, "dangling subtag"),
            (
                r#"<http://a/s> <http://a/p> "x"^<http://t> ."#,
                "single caret",
            ),
            (
                "<http://a/s> <http://a/p> <http://a/o> . junk",
                "trailing junk",
            ),
            ("<http://a/s p> <http://a/p> <http://a/o> .", "space in IRI"),
            ("<http://a/s> \"p\" <http://a/o> .", "literal predicate"),
            ("<http://a/s> <http://a/p> .", "missing object"),
            ("_: <http://a/p> <http://a/o> .", "empty blank label"),
            ("ex:s ex:p ex:o .", "Turtle prefixed names"),
        ] {
            assert!(parse(line).is_err(), "accepted {why}: {line}");
        }
    }

    #[test]
    fn subject_can_be_read_before_the_rest_is_checked() {
        let mut parser = LineParser::new("<http://a/s> this is not a triple");
        assert_eq!(parser.subject().unwrap(), Some(iri("http://a/s")));
        assert!(parser.predicate_object().is_err());
    }
}
