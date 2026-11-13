//! Read a Lua module that only returns data, such as `Module:Ontology/local`,
//! into JSON.
//!
//! The module must be one `return` of a table built from string, number and
//! boolean literals and nested tables, with `name =` and `['quoted'] =` keys.
//! Anything else (a statement before the return, a variable, a call, an
//! operator, `nil`) is refused with its line, because the value it stands for
//! cannot be known without running the module.
//!
//! A table with only list items becomes an array and a table with only keys an
//! object; an empty table becomes an empty object. A table that mixes both is
//! refused. Strings use Lua 5.1 escapes, the version Scribunto runs.

use anyhow::{Result, bail};
use full_moon::ast::{Expression, Field, LastStmt, UnOp};
use full_moon::node::Node;
use full_moon::tokenizer::{StringLiteralQuoteType, TokenReference, TokenType};
use serde_json::{Map, Number, Value};

/// Parse a data-only Lua module. `origin` names it in errors, such as its
/// title or path.
pub(crate) fn parse_lua_data(source: &str, origin: &str) -> Result<Value> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let ast = match full_moon::parse(source) {
        Ok(ast) => ast,
        Err(errors) => {
            let first = errors
                .first()
                .map(ToString::to_string)
                .unwrap_or_else(|| "unknown error".to_string());
            bail!("{origin} is not valid Lua: {first}");
        }
    };
    let block = ast.nodes();
    if let Some(statement) = block.stmts().next() {
        bail!(
            "{origin}{}: a data module holds only `return {{ ... }}`; found a statement before it",
            line_suffix(statement.start_position())
        );
    }
    let Some(LastStmt::Return(ret)) = block.last_stmt() else {
        bail!("{origin} does not return a table");
    };
    let mut values = ret.returns().iter();
    let (Some(expression), None) = (values.next(), values.next()) else {
        bail!("{origin} must return exactly one table");
    };
    if !matches!(expression, Expression::TableConstructor(_)) {
        bail!(
            "{origin}{}: the module returns `{}`, not a table",
            line_suffix(expression.start_position()),
            excerpt(&expression.to_string())
        );
    }
    convert(expression, origin)
}

fn convert(expression: &Expression, origin: &str) -> Result<Value> {
    match expression {
        Expression::TableConstructor(table) => {
            let mut items = Vec::new();
            let mut keyed = Map::new();
            for field in table.fields() {
                let (key, value) = match field {
                    Field::NoKey(value) => {
                        items.push(convert(value, origin)?);
                        continue;
                    }
                    Field::NameKey { key, value, .. } => (key.token().to_string(), value),
                    Field::ExpressionKey { key, value, .. } => match key {
                        Expression::String(token) => (string_literal(token, origin)?, value),
                        other => bail!(
                            "{origin}{}: a table key must be a name or a quoted string, not `{}`",
                            line_suffix(other.start_position()),
                            excerpt(&other.to_string())
                        ),
                    },
                    other => bail!(
                        "{origin}{}: unsupported table field `{}`",
                        line_suffix(other.start_position()),
                        excerpt(&other.to_string())
                    ),
                };
                if keyed.contains_key(&key) {
                    bail!(
                        "{origin}{}: the key {key:?} appears twice in one table",
                        line_suffix(field.start_position())
                    );
                }
                keyed.insert(key, convert(value, origin)?);
            }
            if !items.is_empty() && !keyed.is_empty() {
                bail!(
                    "{origin}{}: a table mixes list items and keys",
                    line_suffix(table.start_position())
                );
            }
            Ok(if items.is_empty() {
                Value::Object(keyed)
            } else {
                Value::Array(items)
            })
        }
        Expression::String(token) => Ok(Value::String(string_literal(token, origin)?)),
        Expression::Number(token) => number_literal(token, false, origin),
        Expression::UnaryOperator {
            unop: UnOp::Minus(_),
            expression: inner,
        } => match inner.as_ref() {
            Expression::Number(token) => number_literal(token, true, origin),
            other => refuse(other, origin),
        },
        Expression::Symbol(token) => match token.token().to_string().as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => refuse(expression, origin),
        },
        other => refuse(other, origin),
    }
}

fn refuse(expression: &Expression, origin: &str) -> Result<Value> {
    bail!(
        "{origin}{}: only strings, numbers, booleans and tables are data, not `{}`",
        line_suffix(expression.start_position()),
        excerpt(&expression.to_string())
    )
}

fn string_literal(token: &TokenReference, origin: &str) -> Result<String> {
    let TokenType::StringLiteral {
        literal,
        quote_type,
        ..
    } = token.token_type()
    else {
        bail!(
            "{origin}{}: expected a string",
            line_suffix(token.start_position())
        );
    };
    match quote_type {
        StringLiteralQuoteType::Brackets => {
            // Lua drops a newline that directly follows the opening bracket.
            let text = literal.as_str();
            let text = text
                .strip_prefix("\r\n")
                .or_else(|| text.strip_prefix('\n'))
                .unwrap_or(text);
            Ok(text.to_string())
        }
        StringLiteralQuoteType::Double | StringLiteralQuoteType::Single => {
            unescape(literal.as_str()).map_err(|error| {
                anyhow::anyhow!("{origin}{}: {error}", line_suffix(token.start_position()))
            })
        }
        #[allow(unreachable_patterns)]
        _ => bail!(
            "{origin}{}: unsupported string quoting",
            line_suffix(token.start_position())
        ),
    }
}

/// Resolve Lua 5.1 escapes. Lua strings are bytes, so `\ddd` escapes may
/// spell out UTF-8; the result must be valid UTF-8.
fn unescape(raw: &str) -> std::result::Result<String, String> {
    let mut out: Vec<u8> = Vec::with_capacity(raw.len());
    let bytes = raw.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte != b'\\' {
            out.push(byte);
            index += 1;
            continue;
        }
        let Some(&next) = bytes.get(index + 1) else {
            return Err("a string ends with a lone backslash".to_string());
        };
        index += 2;
        let resolved = match next {
            b'a' => 0x07,
            b'b' => 0x08,
            b'f' => 0x0c,
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            b'v' => 0x0b,
            b'\\' => b'\\',
            b'"' => b'"',
            b'\'' => b'\'',
            b'\n' => b'\n',
            b'\r' => {
                if bytes.get(index) == Some(&b'\n') {
                    index += 1;
                }
                b'\n'
            }
            b'0'..=b'9' => {
                let mut value = u32::from(next - b'0');
                let mut digits = 1;
                while digits < 3 {
                    match bytes.get(index) {
                        Some(digit @ b'0'..=b'9') => {
                            value = value * 10 + u32::from(digit - b'0');
                            index += 1;
                            digits += 1;
                        }
                        _ => break,
                    }
                }
                u8::try_from(value)
                    .map_err(|_| format!("the escape \\{value} is larger than 255"))?
            }
            other => {
                return Err(format!(
                    "the escape \\{} is not a Lua 5.1 escape",
                    char::from(other)
                ));
            }
        };
        out.push(resolved);
    }
    String::from_utf8(out).map_err(|_| "a string's escapes do not spell valid UTF-8".to_string())
}

fn number_literal(token: &TokenReference, negative: bool, origin: &str) -> Result<Value> {
    let text = token.token().to_string();
    let sign = if negative { -1.0 } else { 1.0 };
    let parsed = if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16)
            .ok()
            .map(|value| Value::Number(Number::from(if negative { -value } else { value })))
    } else if text.bytes().all(|byte| byte.is_ascii_digit()) {
        text.parse::<i64>()
            .ok()
            .map(|value| Value::Number(Number::from(if negative { -value } else { value })))
    } else {
        text.parse::<f64>()
            .ok()
            .and_then(|value| Number::from_f64(sign * value))
            .map(Value::Number)
    };
    match parsed {
        Some(value) => Ok(value),
        None => bail!(
            "{origin}{}: the number {text} cannot be represented",
            line_suffix(token.start_position())
        ),
    }
}

fn line_suffix(position: Option<full_moon::tokenizer::Position>) -> String {
    position
        .map(|position| format!(":{}", position.line()))
        .unwrap_or_default()
}

fn excerpt(text: &str) -> String {
    let text = text.trim();
    let mut out: String = text.chars().take(60).collect();
    if out.len() < text.len() {
        out.push_str("...");
    }
    out.replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_lua_data;

    #[test]
    fn reads_nested_data_tables() {
        let source = "-- comment\nreturn {\n\tclasses = {\n\t\t['wd:Q5'] = { label = 'Human', subClassOf = \"prov:Agent\" },\n\t},\n\tlist = { 'a', \"b\\n\\65\", [[\nlong]] },\n\tcount = -3, ratio = 0.5, hex = 0x10, flag = true, off = false, empty = {},\n}\n";
        let value = parse_lua_data(source, "Module:Test").expect("data");
        assert_eq!(
            value,
            json!({
                "classes": { "wd:Q5": { "label": "Human", "subClassOf": "prov:Agent" } },
                "list": ["a", "b\nA", "long"],
                "count": -3, "ratio": 0.5, "hex": 16, "flag": true, "off": false, "empty": {},
            })
        );
    }

    #[test]
    fn decimal_escapes_may_spell_utf8() {
        let value = parse_lua_data("return { s = '\\195\\169' }", "m").expect("data");
        assert_eq!(value["s"], "é");
    }

    #[test]
    fn refuses_code_and_names_the_line() {
        for (source, needle) in [
            ("local x = 1\nreturn {}", "m:1: a data module holds only"),
            (
                "return {\n a = x }",
                "m:2: only strings, numbers, booleans and tables are data, not `x`",
            ),
            ("return { a = 'x' .. 'y' }", "not `'x' .. 'y'`"),
            ("return { a = nil }", "not `nil`"),
            ("return { a = f() }", "not `f()`"),
            (
                "return { [1] = 'x' }",
                "a table key must be a name or a quoted string",
            ),
            (
                "return { 'x', a = 'y' }",
                "a table mixes list items and keys",
            ),
            ("return { a = 1, a = 2 }", "the key \"a\" appears twice"),
            (
                "return { a = '\\x41' }",
                "the escape \\x is not a Lua 5.1 escape",
            ),
            ("return 'x'", "not a table"),
            ("return {", "is not valid Lua"),
            ("-- nothing", "does not return a table"),
        ] {
            let error = parse_lua_data(source, "m").expect_err(source).to_string();
            assert!(error.contains(needle), "{source}: {error}");
        }
    }
}
