//! The ontology that the wiki types its pages with.
//!
//! `export` reads an ontology directory in a git checkout at one commit and
//! writes a compact JSON copy that the wiki project vendors: the registry
//! (classes, families, properties, relations, value lists), the mappings, the
//! renames, and the labels of the outside terms that `product-uses.tsv` lists
//! for the wiki's product. `generate` turns that copy into
//! `Module:Ontology/data`, a Lua table that infoboxes load with `mw.loadData`.
//! The wiki's own terms are edited on the wiki, not here.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Map, Value, json};

use crate::runtime::ResolvedPaths;
use crate::support::normalize_path;

mod ntriples;

use self::ntriples::{LineParser, Literal, Term};

/// Where the wiki project keeps the vendored copy, relative to its root.
pub const EXPORT_PATH: &str = "vendor/ontology/ontology.json";
/// The wiki page that `generate` writes.
pub const MODULE_TITLE: &str = "Module:Ontology/data";
/// The folder under the templates directory that holds the ontology modules.
/// Sync reads a template file's title from its path alone, so the folder is a
/// layout choice; the title-to-path mapping takes it as a layout hint.
const MODULE_FOLDER: &str = "ontology";
const NO_TERM: &str = "sssom:NoTermFound";
const SECTIONS: [&str; 5] = ["classes", "families", "properties", "relations", "schemes"];

/// What `export` needs besides the ontology directory.
#[derive(Debug, Clone, Copy)]
pub struct ExportOptions<'a> {
    /// The wiki's product name in the ontology's `product-uses.tsv`.
    pub product: &'a str,
    /// The directory holding `product-uses.tsv` and the pinned extracts,
    /// relative to the ontology directory.
    pub imports: &'a Path,
    /// CURIE prefixes whose renames never reach the wiki, matched exactly.
    pub excluded_rename_prefixes: &'a [String],
    /// Where the ontology is published, recorded as `source.ontology`.
    pub published_at: Option<&'a str>,
    /// The prefix and namespace IRI of the wiki's own terms, recorded as
    /// `source.local`.
    pub local: Option<(&'a str, &'a str)>,
}

/// The project-relative path of `Module:Ontology/data`, derived from its
/// title by the same mapping that sync uses.
pub fn module_relative_path(paths: &ResolvedPaths) -> Result<String> {
    let sync_paths = wikitool_sync::SyncProjectPaths {
        project_root: paths.project_root.clone(),
        wiki_content_dir: paths.wiki_content_dir.clone(),
        templates_dir: paths.templates_dir.clone(),
        sync_store_path: paths.sync_store_path(),
        custom_namespaces: Vec::new(),
        template_category_mappings: vec![(MODULE_TITLE.to_string(), MODULE_FOLDER.to_string())],
    };
    wikitool_sync::title_to_relative_path(&sync_paths, MODULE_TITLE, false)
}

/// Build the vendored copy from an ontology directory inside a git checkout.
pub fn export(ontology_dir: &Path, options: &ExportOptions<'_>) -> Result<Value> {
    // Fail on a bad path before asking git anything about it.
    relative_inside(options.imports).with_context(|| {
        format!(
            "the imports directory {} must be relative to the ontology directory",
            normalize_path(options.imports)
        )
    })?;
    let checkout = Checkout::open(ontology_dir)?;
    let mut out = read_registry(&checkout)?;
    out.insert(
        "mappings".into(),
        read_mappings(&checkout, Path::new("mappings.sssom.tsv"))?,
    );
    out.insert(
        "renames".into(),
        read_renames(
            &checkout,
            Path::new("renames.sssom.tsv"),
            options.excluded_rename_prefixes,
        )?,
    );
    out.insert(
        "external".into(),
        Value::Object(external_terms(&checkout, options.imports, options.product)?),
    );
    let mut source = json!({ "commit": checkout.commit, "product": options.product });
    if let Some(published_at) = options.published_at {
        source["ontology"] = Value::String(published_at.to_string());
    }
    if let Some((prefix, namespace)) = options.local {
        source["local"] = json!({ prefix: namespace });
    }
    out.insert("source".into(), source);
    Ok(canonical(Value::Object(out)))
}

/// An ontology directory at one clean commit. Every file `export` reads goes
/// through [`Checkout::read`], which refuses files that the commit does not
/// contain: `git status` does not report ignored files, so an ignored file
/// would otherwise reach the export under a commit that lacks it.
struct Checkout<'a> {
    dir: &'a Path,
    commit: String,
    tracked: BTreeSet<String>,
}

impl<'a> Checkout<'a> {
    fn open(dir: &'a Path) -> Result<Self> {
        let dirty = git(dir, &["status", "--porcelain", "--", "."])?;
        if !dirty.trim().is_empty() {
            bail!(
                "{} has uncommitted changes; export from a clean checkout of one commit",
                normalize_path(dir)
            );
        }
        let commit = git(dir, &["rev-parse", "--verify", "HEAD"])?
            .trim()
            .to_string();
        // Paths are relative to `dir`, NUL-separated and never quoted.
        let tracked = git(dir, &["ls-files", "-z", "--", "."])?
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(str::to_string)
            .collect();
        Ok(Self {
            dir,
            commit,
            tracked,
        })
    }

    fn path(&self, relative: &Path) -> PathBuf {
        self.dir.join(relative)
    }

    /// Read a committed text file. A UTF-8 byte order mark is dropped; line
    /// endings are left to the parsers, which all accept LF and CRLF, so a
    /// checkout with `core.autocrlf` exports the same bytes.
    fn read(&self, relative: &Path) -> Result<String> {
        let path = self.path(relative);
        let key = relative_inside(relative).with_context(|| {
            format!(
                "{} is not a path inside the ontology directory",
                normalize_path(relative)
            )
        })?;
        if !self.tracked.contains(&key) {
            bail!(
                "{} is not committed at {}; export reads only files in the commit",
                normalize_path(&path),
                self.commit
            );
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", normalize_path(&path)))?;
        Ok(match text.strip_prefix('\u{feff}') {
            Some(rest) => rest.to_string(),
            None => text,
        })
    }
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .context("failed to run git")?;
    if !output.status.success() {
        bail!(
            "git {} failed in {}: {}",
            args.join(" "),
            normalize_path(dir),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The `/`-separated form of a relative path that stays inside its base, or
/// an error for an absolute path or one that climbs out with `..`.
fn relative_inside(path: &Path) -> Result<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(
                part.to_str()
                    .context("the path is not valid UTF-8")?
                    .to_string(),
            ),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("only a relative path without `..` is accepted")
            }
        }
    }
    ensure!(!parts.is_empty(), "the path is empty");
    Ok(parts.join("/"))
}

fn read_registry(checkout: &Checkout<'_>) -> Result<Map<String, Value>> {
    let mut sections: BTreeMap<&str, Map<String, Value>> =
        SECTIONS.iter().map(|kind| (*kind, Map::new())).collect();
    let registry = checkout.path(Path::new("registry"));
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(&registry)
        .with_context(|| format!("failed to read {}", normalize_path(&registry)))?
    {
        let entry =
            entry.with_context(|| format!("failed to read {}", normalize_path(&registry)))?;
        let name = PathBuf::from(entry.file_name());
        if name.extension().is_some_and(|ext| ext == "jsonld") {
            files.push(Path::new("registry").join(name));
        }
    }
    files.sort();
    for relative in files {
        let path = normalize_path(checkout.path(&relative));
        let text = checkout.read(&relative)?;
        let mut entry: Map<String, Value> =
            serde_json::from_str(&text).with_context(|| format!("failed to parse {path}"))?;
        entry.remove("@context");
        let id = take_string(&mut entry, "@id", &path)?;
        let kind = take_string(&mut entry, "@type", &path)?;
        let section = match kind.as_str() {
            "OntologyClass" | "OntologySubclass" => "classes",
            "OntologyFamily" => "families",
            "OntologyProperty" => "properties",
            "OntologyRelationType" => "relations",
            "OntologyScheme" => "schemes",
            other => bail!("{path}: unknown @type {other}"),
        };
        if let Some(Value::Array(members)) = entry.remove("members") {
            let mut keyed = Map::new();
            for member in members {
                let Value::Object(mut member) = member else {
                    bail!("{path}: a member is not an object");
                };
                let member_id = take_string(&mut member, "@id", &path)?;
                member.remove("@type");
                if keyed.contains_key(&member_id) {
                    bail!("{path}: member {member_id} is listed twice");
                }
                keyed.insert(member_id, Value::Object(member));
            }
            entry.insert("members".into(), Value::Object(keyed));
        }
        let section = sections.get_mut(section).expect("section exists");
        if section.contains_key(&id) {
            bail!("{path}: {id} is defined twice");
        }
        section.insert(id, Value::Object(entry));
    }
    Ok(sections
        .into_iter()
        .map(|(kind, entries)| (kind.to_string(), Value::Object(entries)))
        .collect())
}

fn read_mappings(checkout: &Checkout<'_>, relative: &Path) -> Result<Value> {
    let table = Tsv::parse(&checkout.path(relative), &checkout.read(relative)?)?;
    let [subject, predicate, object] =
        table.columns(["subject_id", "predicate_id", "object_id"])?;
    let mut mappings: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for row in &table.rows {
        let object = &row.cells[object];
        if object == NO_TERM || object.is_empty() {
            continue;
        }
        mappings
            .entry(row.cells[subject].clone())
            .or_default()
            .push(json!({ "predicate": row.cells[predicate], "object": object }));
    }
    Ok(Value::Object(
        mappings
            .into_iter()
            .map(|(key, rows)| (key, Value::Array(rows)))
            .collect(),
    ))
}

fn read_renames(
    checkout: &Checkout<'_>,
    relative: &Path,
    excluded_prefixes: &[String],
) -> Result<Value> {
    let table = Tsv::parse(&checkout.path(relative), &checkout.read(relative)?)?;
    let [subject, object] = table.columns(["subject_id", "object_id"])?;
    let mut renames = Map::new();
    for row in &table.rows {
        let subject = &row.cells[subject];
        let excluded = subject.split_once(':').is_some_and(|(prefix, _)| {
            excluded_prefixes
                .iter()
                .any(|excluded| excluded.as_str() == prefix)
        });
        if !excluded {
            renames.insert(subject.clone(), Value::String(row.cells[object].clone()));
        }
    }
    Ok(Value::Object(renames))
}

/// The labels and definitions of the outside terms the wiki uses, read from
/// the pinned extracts that `product-uses.tsv` names for the wiki.
fn external_terms(
    checkout: &Checkout<'_>,
    imports: &Path,
    product: &str,
) -> Result<Map<String, Value>> {
    let uses_relative = imports.join("product-uses.tsv");
    let uses_path = normalize_path(checkout.path(&uses_relative));
    let uses_text = checkout.read(&uses_relative)?;
    let curie_map = read_curie_map(&uses_text);
    let table = Tsv::parse(&checkout.path(&uses_relative), &uses_text)?;
    let [product_column, iri_column, pin_column] = table.columns(["product", "iri", "pin"])?;
    let mut wanted: BTreeMap<String, (String, String)> = BTreeMap::new();
    for row in &table.rows {
        if row.cells[product_column] != product {
            continue;
        }
        let curie = &row.cells[iri_column];
        let iri = expand(curie, &curie_map)
            .with_context(|| format!("{uses_path}:{}: no prefix for {curie}", row.line))?;
        wanted.insert(iri, (curie.clone(), row.cells[pin_column].clone()));
    }

    let mut by_pin: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (iri, (_, pin)) in &wanted {
        by_pin.entry(pin.as_str()).or_default().insert(iri.as_str());
    }
    let mut best: BTreeMap<(String, &'static str), Candidate> = BTreeMap::new();
    for (pin, iris) in by_pin {
        let relative = imports.join(pin);
        let path = normalize_path(checkout.path(&relative));
        let text = checkout.read(&relative)?;
        for (index, line) in text.lines().enumerate() {
            let at = |error: String| anyhow::anyhow!("{path}:{}: {error}", index + 1);
            let mut parser = LineParser::new(line);
            // An unreadable subject could be any term, so it always fails.
            let Some(Term::Iri(subject)) = parser.subject().map_err(at)? else {
                continue;
            };
            if !iris.contains(subject.as_str()) {
                continue;
            }
            let (predicate, object) = parser.predicate_object().map_err(at)?;
            let Some((field, predicate_rank)) = field_of(&predicate) else {
                continue;
            };
            let Term::Literal(literal) = object else {
                continue;
            };
            let Some(candidate) = Candidate::new(literal, predicate_rank) else {
                continue;
            };
            match best.entry((subject, field)) {
                std::collections::btree_map::Entry::Vacant(slot) => {
                    slot.insert(candidate);
                }
                std::collections::btree_map::Entry::Occupied(mut slot) => {
                    if candidate < *slot.get() {
                        slot.insert(candidate);
                    }
                }
            }
        }
    }

    let mut found: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    for ((iri, field), candidate) in best {
        found
            .entry(iri)
            .or_default()
            .insert(field.to_string(), Value::String(candidate.value));
    }
    let mut out = Map::new();
    for (iri, (curie, _)) in wanted {
        let mut entry = found.remove(&iri).unwrap_or_default();
        entry.insert("iri".into(), Value::String(iri));
        out.insert(curie, Value::Object(entry));
    }
    Ok(out)
}

/// The exported field a predicate supplies, and its rank among the
/// predicates for that field. The SKOS properties are the more specific
/// ones (`skos:prefLabel` is the preferred label, `skos:definition` a
/// definition rather than any comment), so they rank first.
fn field_of(predicate: &str) -> Option<(&'static str, u8)> {
    Some(match predicate {
        "http://www.w3.org/2004/02/skos/core#prefLabel" => ("label", 0),
        "http://www.w3.org/2000/01/rdf-schema#label" => ("label", 1),
        "http://www.w3.org/2004/02/skos/core#definition" => ("definition", 0),
        "http://www.w3.org/2000/01/rdf-schema#comment" => ("definition", 1),
        _ => return None,
    })
}

/// A literal that may supply a field. Candidates order by language (`@en`,
/// then another `@en-*` tag, then an untagged literal), then by predicate,
/// then by tag and text, so the choice never depends on line order. A literal
/// in any other language is never a candidate.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Candidate {
    language_rank: u8,
    predicate_rank: u8,
    language: String,
    value: String,
}

impl Candidate {
    fn new(literal: Literal, predicate_rank: u8) -> Option<Self> {
        let language = literal.language.unwrap_or_default().to_ascii_lowercase();
        let language_rank = if language == "en" {
            0
        } else if language.starts_with("en-") {
            1
        } else if language.is_empty() {
            2
        } else {
            return None;
        };
        Some(Self {
            language_rank,
            predicate_rank,
            language,
            value: literal.value,
        })
    }
}

/// The prefixes in the `# curie_map:` block of an SSSOM metadata header.
fn read_curie_map(text: &str) -> BTreeMap<String, String> {
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
    map
}

fn expand(curie: &str, map: &BTreeMap<String, String>) -> Option<String> {
    let (prefix, local) = curie.split_once(':')?;
    map.get(prefix).map(|base| format!("{base}{local}"))
}

/// A tab-separated table such as an SSSOM/TSV file: `#` lines (the SSSOM
/// metadata header) and blank lines are skipped, the first other line is the
/// header, and every later line is a row.
///
/// Every row must have exactly one cell per header column. IANA's
/// `text/tab-separated-values` requires each record to have the same number
/// of fields, and SSSOM tooling writes every column. A row that is short
/// because an editor stripped its trailing tabs cannot be told apart from one
/// that lost a tab in the middle, which would shift values into the wrong
/// columns, so a short row is an error rather than padded with empty cells.
#[derive(Debug)]
struct Tsv {
    path: String,
    header: Vec<String>,
    rows: Vec<TsvRow>,
}

#[derive(Debug)]
struct TsvRow {
    /// The 1-based line number in the file.
    line: usize,
    cells: Vec<String>,
}

impl Tsv {
    fn parse(path: &Path, text: &str) -> Result<Self> {
        let path = normalize_path(path);
        let mut header: Option<Vec<String>> = None;
        let mut rows = Vec::new();
        for (index, line) in text.lines().enumerate() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let cells: Vec<String> = line.split('\t').map(str::to_string).collect();
            let Some(columns) = &header else {
                let mut seen = BTreeSet::new();
                for column in &cells {
                    ensure!(
                        seen.insert(column.as_str()),
                        "{path}:{}: the header names column {column} twice",
                        index + 1
                    );
                }
                header = Some(cells);
                continue;
            };
            ensure!(
                cells.len() == columns.len(),
                "{path}:{}: the row has {} cells but the header has {} columns; \
                 an empty trailing cell still needs its tab",
                index + 1,
                cells.len(),
                columns.len()
            );
            rows.push(TsvRow {
                line: index + 1,
                cells,
            });
        }
        let header = header.with_context(|| format!("{path} has no header row"))?;
        Ok(Self { path, header, rows })
    }

    /// The indices of the named columns, which must all be present.
    fn columns<const N: usize>(&self, names: [&str; N]) -> Result<[usize; N]> {
        let mut indices = [0; N];
        for (slot, name) in indices.iter_mut().zip(names) {
            *slot = self
                .header
                .iter()
                .position(|column| column == name)
                .with_context(|| format!("{} has no {name} column", self.path))?;
        }
        Ok(indices)
    }
}

fn take_string(entry: &mut Map<String, Value>, key: &str, path: &str) -> Result<String> {
    match entry.remove(key) {
        Some(Value::String(value)) => Ok(value),
        _ => bail!("{path}: missing {key}"),
    }
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
///
/// `mw.loadData` accepts only tables, strings, numbers and booleans. A null
/// object member is left out (an absent key reads as `nil`); a null in an
/// array is an error, because it would leave a hole that breaks `#` and
/// `ipairs`.
pub fn generate_module(export: &Value) -> Result<String> {
    let commit = export
        .pointer("/source/commit")
        .and_then(Value::as_str)
        .context("the export has no source commit")?;
    let mut out = String::new();
    out.push_str(&format!(
        "-- Generated by `wikitool ontology generate` from {EXPORT_PATH}\n"
    ));
    out.push_str(&format!(
        "-- at ontology commit {commit}. Do not edit by hand.\n"
    ));
    out.push_str("return ");
    write_lua(&mut out, export, 0, &mut String::new())?;
    out.push('\n');
    Ok(out)
}

fn write_lua(out: &mut String, value: &Value, depth: usize, pointer: &mut String) -> Result<()> {
    match value {
        Value::Null => bail!(
            "the export has null at JSON pointer {pointer}; {MODULE_TITLE} cannot hold null in an array"
        ),
        Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Value::Number(number) => {
            // Unreachable unless serde_json's arbitrary_precision is enabled,
            // which lets a Value hold a number such as 1e400.
            ensure!(
                number.as_f64().is_some_and(f64::is_finite),
                "the export has the non-finite number {number} at JSON pointer {pointer}"
            );
            out.push_str(&number.to_string());
        }
        Value::String(text) => write_lua_string(out, text),
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("{}");
                return Ok(());
            }
            let scalar = items
                .iter()
                .all(|item| !item.is_object() && !item.is_array());
            out.push_str(if scalar { "{" } else { "{\n" });
            for (index, item) in items.iter().enumerate() {
                if scalar && index > 0 {
                    out.push_str(", ");
                }
                if !scalar {
                    indent(out, depth + 1);
                }
                let length = pointer.len();
                pointer.push('/');
                pointer.push_str(&index.to_string());
                write_lua(out, item, depth + 1, pointer)?;
                pointer.truncate(length);
                if !scalar {
                    out.push_str(",\n");
                }
            }
            if !scalar {
                indent(out, depth);
            }
            out.push('}');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().filter(|key| !map[*key].is_null()).collect();
            if keys.is_empty() {
                out.push_str("{}");
                return Ok(());
            }
            keys.sort();
            out.push_str("{\n");
            for key in keys {
                indent(out, depth + 1);
                out.push('[');
                write_lua_string(out, key);
                out.push_str("] = ");
                let length = pointer.len();
                pointer.push('/');
                pointer.push_str(&key.replace('~', "~0").replace('/', "~1"));
                write_lua(out, &map[key], depth + 1, pointer)?;
                pointer.truncate(length);
                out.push_str(",\n");
            }
            indent(out, depth);
            out.push('}');
        }
    }
    Ok(())
}

/// A Lua 5.1 double-quoted string. Lua strings are bytes, so UTF-8 passes
/// through unchanged; other control characters use a three-digit decimal
/// escape, which a following digit cannot extend.
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
mod tests;
