//! The ontology that the wiki types its pages with.
//!
//! `export` reads an ontology directory in a git checkout at one commit and
//! writes a compact JSON copy that the wiki project vendors: the registry
//! (classes, families, properties, relations, value lists), the mappings, the
//! renames, and the labels of the outside terms that `product-uses.tsv` lists
//! for the wiki's product. `generate` turns that copy into
//! `Module:Ontology/data`, a Lua table that infoboxes load with `mw.loadData`.
//! The wiki's own terms are edited on the wiki, not here.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};

/// Where the wiki project keeps the vendored copy.
pub const EXPORT_PATH: &str = "vendor/ontology/ontology.json";
/// Where the generated module lives; it syncs as `Module:Ontology/data`.
pub const MODULE_PATH: &str = "templates/ontology/Module_Ontology/data.lua";
const NO_TERM: &str = "sssom:NoTermFound";
const SECTIONS: [&str; 5] = ["classes", "families", "properties", "relations", "schemes"];

/// Build the vendored copy from an ontology directory inside a git checkout.
/// `product` names the wiki in the ontology's `product-uses.tsv`.
pub fn export(ontology_dir: &Path, product: &str) -> Result<Value> {
    let commit = clean_commit(ontology_dir)?;
    let mut out = read_registry(&ontology_dir.join("registry"))?;
    out.insert(
        "mappings".into(),
        read_mappings(&ontology_dir.join("mappings.sssom.tsv"))?,
    );

    // Renames of legacy RemiliaNET names (rn:, rns:) never reach the wiki.
    let mut renames = Map::new();
    for row in read_tsv(&ontology_dir.join("renames.sssom.tsv"))? {
        let (Some(subject), Some(object)) = (row.get("subject_id"), row.get("object_id")) else {
            continue;
        };
        if subject.starts_with("rn:") || subject.starts_with("rns:") {
            continue;
        }
        renames.insert(subject.clone(), Value::String(object.clone()));
    }
    out.insert("renames".into(), Value::Object(renames));
    out.insert(
        "external".into(),
        Value::Object(external_terms(ontology_dir, product)?),
    );
    out.insert(
        "source".into(),
        json!({ "commit": commit, "product": product }),
    );
    Ok(canonical(Value::Object(out)))
}

fn read_registry(registry: &Path) -> Result<Map<String, Value>> {
    let mut sections: BTreeMap<&str, Map<String, Value>> =
        SECTIONS.iter().map(|kind| (*kind, Map::new())).collect();
    let mut files: Vec<PathBuf> = fs::read_dir(registry)
        .with_context(|| format!("failed to read {}", registry.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonld"))
        .collect();
    files.sort();
    for path in files {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let mut entry: Map<String, Value> = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        entry.remove("@context");
        let id = take_string(&mut entry, "@id", &path)?;
        let kind = take_string(&mut entry, "@type", &path)?;
        let section = match kind.as_str() {
            "OntologyClass" | "OntologySubclass" => "classes",
            "OntologyFamily" => "families",
            "OntologyProperty" => "properties",
            "OntologyRelationType" => "relations",
            "OntologyScheme" => "schemes",
            other => bail!("{}: unknown @type {other}", path.display()),
        };
        if let Some(Value::Array(members)) = entry.remove("members") {
            let mut keyed = Map::new();
            for member in members {
                let Value::Object(mut member) = member else {
                    bail!("{}: a member is not an object", path.display());
                };
                let member_id = take_string(&mut member, "@id", &path)?;
                member.remove("@type");
                keyed.insert(member_id, Value::Object(member));
            }
            entry.insert("members".into(), Value::Object(keyed));
        }
        let section = sections.get_mut(section).expect("section exists");
        if section.contains_key(&id) {
            bail!("{}: {id} is defined twice", path.display());
        }
        section.insert(id, Value::Object(entry));
    }
    Ok(sections
        .into_iter()
        .map(|(kind, entries)| (kind.to_string(), Value::Object(entries)))
        .collect())
}

fn read_mappings(path: &Path) -> Result<Value> {
    let mut mappings: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for row in read_tsv(path)? {
        let (Some(subject), Some(predicate), Some(object)) = (
            row.get("subject_id"),
            row.get("predicate_id"),
            row.get("object_id"),
        ) else {
            continue;
        };
        if object == NO_TERM || object.is_empty() {
            continue;
        }
        mappings
            .entry(subject.clone())
            .or_default()
            .push(json!({ "predicate": predicate, "object": object }));
    }
    Ok(Value::Object(
        mappings
            .into_iter()
            .map(|(key, rows)| (key, Value::Array(rows)))
            .collect(),
    ))
}

/// The labels and definitions of the outside terms the wiki uses, read from
/// the pinned extracts that `product-uses.tsv` names for the wiki.
fn external_terms(ontology_dir: &Path, product: &str) -> Result<Map<String, Value>> {
    let imports = ontology_dir.join("v0.1.0/imports");
    let uses_path = imports.join("product-uses.tsv");
    let curie_map = read_curie_map(&uses_path)?;
    let mut wanted: BTreeMap<String, (String, String)> = BTreeMap::new();
    for row in read_tsv(&uses_path)? {
        if row.get("product").map(String::as_str) != Some(product) {
            continue;
        }
        let (Some(curie), Some(pin)) = (row.get("iri"), row.get("pin")) else {
            continue;
        };
        let iri = expand(curie, &curie_map)
            .with_context(|| format!("{}: no prefix for {curie}", uses_path.display()))?;
        wanted.insert(iri, (curie.clone(), pin.clone()));
    }

    let mut by_pin: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (iri, (_, pin)) in &wanted {
        by_pin.entry(pin.as_str()).or_default().push(iri.as_str());
    }
    let mut found: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    for (pin, iris) in by_pin {
        let path = imports.join(pin);
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        for line in text.lines() {
            let Some((subject, rest)) = line.strip_prefix('<').and_then(|l| l.split_once("> <"))
            else {
                continue;
            };
            if !iris.contains(&subject) {
                continue;
            }
            let Some((predicate, object)) = rest.split_once("> ") else {
                continue;
            };
            let field = match predicate {
                "http://www.w3.org/2000/01/rdf-schema#label"
                | "http://www.w3.org/2004/02/skos/core#prefLabel" => "label",
                "http://www.w3.org/2000/01/rdf-schema#comment"
                | "http://www.w3.org/2004/02/skos/core#definition" => "definition",
                _ => continue,
            };
            let Some(literal) = english_literal(object) else {
                continue;
            };
            found
                .entry(subject.to_string())
                .or_default()
                .entry(field)
                .or_insert(Value::String(literal));
        }
    }

    let mut out = Map::new();
    for (iri, (curie, _)) in wanted {
        let mut entry = found.remove(&iri).unwrap_or_default();
        entry.insert("iri".into(), Value::String(iri));
        out.insert(curie, Value::Object(entry));
    }
    Ok(out)
}

/// Parse an N-Triples object that is an English (or untagged) literal.
fn english_literal(object: &str) -> Option<String> {
    let object = object.trim_end().strip_suffix('.')?.trim_end();
    let body = object.strip_prefix('"')?;
    let (text, tag) = body.rsplit_once('"')?;
    if !(tag.is_empty() || tag == "@en" || tag.starts_with("^^")) {
        return None;
    }
    serde_json::from_str::<String>(&format!("\"{text}\"")).ok()
}

fn read_curie_map(path: &Path) -> Result<BTreeMap<String, String>> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut map = BTreeMap::new();
    for line in text.lines().take_while(|line| line.starts_with('#')) {
        let line = line.trim_start_matches('#').trim();
        if let Some((prefix, iri)) = line.split_once(": ")
            && !prefix.contains(' ')
            && prefix != "curie_map"
        {
            map.insert(prefix.to_string(), iri.trim().to_string());
        }
    }
    Ok(map)
}

fn expand(curie: &str, map: &BTreeMap<String, String>) -> Option<String> {
    let (prefix, local) = curie.split_once(':')?;
    map.get(prefix).map(|base| format!("{base}{local}"))
}

fn read_tsv(path: &Path) -> Result<Vec<BTreeMap<String, String>>> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut lines = text.lines().filter(|line| !line.starts_with('#'));
    let Some(header) = lines.next() else {
        return Ok(Vec::new());
    };
    let columns: Vec<&str> = header.split('\t').collect();
    Ok(lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            columns
                .iter()
                .zip(line.split('\t'))
                .map(|(column, value)| (column.to_string(), value.to_string()))
                .collect()
        })
        .collect())
}

fn take_string(entry: &mut Map<String, Value>, key: &str, path: &Path) -> Result<String> {
    match entry.remove(key) {
        Some(Value::String(value)) => Ok(value),
        _ => bail!("{}: missing {key}", path.display()),
    }
}

/// The commit of the checkout, refusing uncommitted changes to the ontology.
fn clean_commit(ontology_dir: &Path) -> Result<String> {
    let git = |args: &[&str]| -> Result<String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(ontology_dir)
            .args(args)
            .output()
            .context("failed to run git")?;
        if !output.status.success() {
            bail!(
                "git {} failed in {}: {}",
                args.join(" "),
                ontology_dir.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    };
    let dirty = git(&["status", "--porcelain", "--", "."])?;
    if !dirty.is_empty() {
        bail!(
            "{} has uncommitted changes; export from a clean checkout of one commit",
            ontology_dir.display()
        );
    }
    git(&["rev-parse", "HEAD"])
}

/// Rebuild every object with its keys in sorted order.
fn canonical(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted: BTreeMap<String, Value> = map
                .into_iter()
                .map(|(key, value)| (key, canonical(value)))
                .collect();
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(items) => Value::Array(items.into_iter().map(canonical).collect()),
        other => other,
    }
}

/// Render the export as the source of `Module:Ontology/data`.
pub fn generate_module(export: &Value) -> Result<String> {
    let commit = export
        .pointer("/source/commit")
        .and_then(Value::as_str)
        .context("the export has no source commit")?;
    let mut out = String::new();
    out.push_str(
        "-- Generated by `wikitool ontology generate` from vendor/ontology/ontology.json\n",
    );
    out.push_str(&format!(
        "-- at ontology commit {commit} Do not edit by hand.\n"
    ));
    out.push_str("return ");
    write_lua(&mut out, export, 0);
    out.push('\n');
    Ok(out)
}

fn write_lua(out: &mut String, value: &Value, depth: usize) {
    match value {
        Value::Null => out.push_str("nil"),
        Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Value::Number(number) => out.push_str(&number.to_string()),
        Value::String(text) => write_lua_string(out, text),
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("{}");
                return;
            }
            let scalar = items
                .iter()
                .all(|item| !item.is_object() && !item.is_array());
            if scalar {
                out.push('{');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push_str(", ");
                    }
                    write_lua(out, item, depth + 1);
                }
                out.push('}');
                return;
            }
            out.push_str("{\n");
            for item in items {
                indent(out, depth + 1);
                write_lua(out, item, depth + 1);
                out.push_str(",\n");
            }
            indent(out, depth);
            out.push('}');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().filter(|key| !map[*key].is_null()).collect();
            if keys.is_empty() {
                out.push_str("{}");
                return;
            }
            keys.sort();
            out.push_str("{\n");
            for key in keys {
                indent(out, depth + 1);
                out.push('[');
                write_lua_string(out, key);
                out.push_str("] = ");
                write_lua(out, &map[key], depth + 1);
                out.push_str(",\n");
            }
            indent(out, depth);
            out.push('}');
        }
    }
}

fn write_lua_string(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 0x20 || ch == '\u{7f}' => {
                out.push_str(&format!("\\{:03}", ch as u32));
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push('\t');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_output_is_sorted_and_escaped() {
        let value = json!({
            "source": { "commit": "abc" },
            "b": ["x", "y\"z"],
            "a": { "label": "line\nbreak", "skip": null },
        });
        let lua = generate_module(&value).expect("module");
        let body = lua.split_once("return ").expect("return").1;
        assert!(body.find("[\"a\"]").unwrap() < body.find("[\"b\"]").unwrap());
        assert!(body.contains("[\"label\"] = \"line\\nbreak\""));
        assert!(body.contains("{\"x\", \"y\\\"z\"}"));
        assert!(!body.contains("skip"));
    }

    #[test]
    fn english_literal_reads_tagged_and_escaped_text() {
        assert_eq!(
            english_literal("\"human \\\"being\\\"\"@en .").as_deref(),
            Some("human \"being\"")
        );
        assert_eq!(english_literal("\"Mensch\"@de ."), None);
        assert_eq!(english_literal("<http://example.org/x> ."), None);
    }

    #[test]
    fn curie_map_reads_the_comment_header() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("uses.tsv");
        fs::write(
            &path,
            "# curie_map:\n#  wd: http://www.wikidata.org/entity/\niri\tproduct\n",
        )
        .expect("write");
        let map = read_curie_map(&path).expect("map");
        assert_eq!(
            expand("wd:Q5", &map).as_deref(),
            Some("http://www.wikidata.org/entity/Q5")
        );
    }
}
