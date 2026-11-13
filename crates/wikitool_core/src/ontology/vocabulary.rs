//! The terms a wiki can state: the vendored export of the shared ontology,
//! the wiki's own terms in `Module:Ontology/local`, and the presentation in
//! `Module:Ontology/display`. Lookups follow the wiki's `Module:Ontology`: a
//! shared term wins over a local one with the same id, and renames are
//! followed a bounded number of hops.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

/// The sections a term can be defined in, in the order `Module:Ontology`
/// searches them.
pub(crate) const TERM_SECTIONS: [&str; 5] =
    ["classes", "families", "properties", "relations", "schemes"];
/// The rename and subclass hops followed before giving up on a cycle.
const MAX_HOPS: usize = 32;

#[derive(Debug, Clone)]
pub(crate) struct Vocabulary {
    shared: Value,
    local: Value,
    display: Value,
}

/// A term found in one section of the shared ontology or the local module.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Term<'a> {
    pub(crate) entry: &'a Map<String, Value>,
    pub(crate) section: &'static str,
    pub(crate) local: bool,
}

impl<'a> Term<'a> {
    pub(crate) fn label(&self) -> Option<&'a str> {
        self.entry.get("label").and_then(Value::as_str)
    }

    pub(crate) fn str_field(&self, key: &str) -> Option<&'a str> {
        self.entry.get(key).and_then(Value::as_str)
    }

    /// A field that holds one id or a list of ids.
    pub(crate) fn ids(&self, key: &str) -> Option<Vec<&'a str>> {
        string_list(self.entry.get(key)?)
    }
}

impl Vocabulary {
    /// `local` and `display` are the parsed modules, or an empty object when
    /// a module is absent.
    pub(crate) fn new(shared: Value, local: Value, display: Value) -> Self {
        Self {
            shared,
            local,
            display,
        }
    }

    pub(crate) fn commit(&self) -> Option<&str> {
        self.shared
            .pointer("/source/commit")
            .and_then(Value::as_str)
    }

    pub(crate) fn term(&self, section: &'static str, id: &str) -> Option<Term<'_>> {
        if let Some(entry) = self.shared_entry(section, id) {
            return Some(Term {
                entry,
                section,
                local: false,
            });
        }
        self.local_entry(section, id).map(|entry| Term {
            entry,
            section,
            local: true,
        })
    }

    /// The term for an id in any section, searched in [`TERM_SECTIONS`] order.
    pub(crate) fn find(&self, id: &str) -> Option<Term<'_>> {
        TERM_SECTIONS
            .iter()
            .find_map(|section| self.term(section, id))
    }

    pub(crate) fn shared_entry(&self, section: &str, id: &str) -> Option<&Map<String, Value>> {
        self.shared.get(section)?.get(id)?.as_object()
    }

    pub(crate) fn local_entry(&self, section: &str, id: &str) -> Option<&Map<String, Value>> {
        self.local.get(section)?.get(id)?.as_object()
    }

    /// The ids one section of the local module defines.
    pub(crate) fn local_ids(&self, section: &str) -> Vec<&str> {
        self.local
            .get(section)
            .and_then(Value::as_object)
            .map(|entries| entries.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// The shared sections that define an id.
    pub(crate) fn shared_sections_of(&self, id: &str) -> Vec<&'static str> {
        TERM_SECTIONS
            .iter()
            .copied()
            .filter(|section| self.shared_entry(section, id).is_some())
            .collect()
    }

    pub(crate) fn local_has_mapping(&self, id: &str) -> bool {
        match self
            .local
            .get("mappings")
            .and_then(|mappings| mappings.get(id))
        {
            Some(Value::Array(rows)) => !rows.is_empty(),
            Some(Value::Object(row)) => !row.is_empty(),
            _ => false,
        }
    }

    /// The current id of a renamed id, or `None` when it was not renamed.
    pub(crate) fn renamed_to(&self, id: &str) -> Option<String> {
        let renames = self.shared.get("renames")?;
        let mut current = id;
        let mut hops = 0;
        while let Some(next) = renames.get(current).and_then(Value::as_str) {
            if next == current || hops >= MAX_HOPS {
                break;
            }
            current = next;
            hops += 1;
        }
        (current != id).then(|| current.to_string())
    }

    /// The id after following renames.
    pub(crate) fn current(&self, id: &str) -> String {
        self.renamed_to(id).unwrap_or_else(|| id.to_string())
    }

    /// The superclasses of a class, nearest first, through `subClassOf` in
    /// the shared ontology and the local module. A class with several
    /// superclasses contributes all of them, breadth first.
    pub(crate) fn ancestors(&self, class: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::from([class.to_string()]);
        let mut frontier = vec![class.to_string()];
        while !frontier.is_empty() && out.len() < MAX_HOPS {
            let mut next = Vec::new();
            for id in frontier {
                let Some(term) = self.term("classes", &id) else {
                    continue;
                };
                for parent in term.ids("subClassOf").unwrap_or_default() {
                    let parent = self.current(parent);
                    if seen.insert(parent.clone()) {
                        out.push(parent.clone());
                        next.push(parent);
                    }
                }
            }
            frontier = next;
        }
        out
    }

    /// A class followed by its ancestors.
    pub(crate) fn lineage(&self, class: &str) -> Vec<String> {
        let mut out = vec![class.to_string()];
        out.extend(self.ancestors(class));
        out
    }

    /// The label of any id: a term, an outside term, or a list member.
    pub(crate) fn label(&self, id: &str) -> Option<&str> {
        if let Some(label) = self.find(id).and_then(|term| term.label()) {
            return Some(label);
        }
        if let Some(label) = self
            .shared
            .get("external")
            .and_then(|external| external.get(id))
            .and_then(|entry| entry.get("label"))
            .and_then(Value::as_str)
        {
            return Some(label);
        }
        for source in [&self.shared, &self.local] {
            if let Some(schemes) = source.get("schemes").and_then(Value::as_object) {
                for scheme in schemes.values() {
                    if let Some(label) = scheme
                        .get("members")
                        .and_then(|members| members.get(id))
                        .and_then(|member| member.get("label"))
                        .and_then(Value::as_str)
                    {
                        return Some(label);
                    }
                }
            }
        }
        None
    }

    /// Every property and relation in the shared ontology and the local
    /// module, shared first, each id once.
    pub(crate) fn statement_terms(&self) -> Vec<(&str, Term<'_>)> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        for section in ["properties", "relations"] {
            for (source, local) in [(&self.shared, false), (&self.local, true)] {
                let Some(entries) = source.get(section).and_then(Value::as_object) else {
                    continue;
                };
                for (id, entry) in entries {
                    let Some(entry) = entry.as_object() else {
                        continue;
                    };
                    if seen.insert(id.as_str()) {
                        out.push((
                            id.as_str(),
                            Term {
                                entry,
                                section: if section == "properties" {
                                    "properties"
                                } else {
                                    "relations"
                                },
                                local,
                            },
                        ));
                    }
                }
            }
        }
        out
    }

    /// The members of a value list, by id, with their labels.
    pub(crate) fn scheme_members(&self, scheme: &str) -> Option<&Map<String, Value>> {
        self.term("schemes", scheme)?
            .entry
            .get("members")?
            .as_object()
    }

    pub(crate) fn display_field(&self, name: &str) -> Option<&Map<String, Value>> {
        self.display.get("fields")?.get(name)?.as_object()
    }

    pub(crate) fn display_fields(&self) -> Vec<(&str, &Map<String, Value>)> {
        self.display
            .get("fields")
            .and_then(Value::as_object)
            .map(|fields| {
                fields
                    .iter()
                    .filter_map(|(name, field)| Some((name.as_str(), field.as_object()?)))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn display_type(&self, key: &str) -> Option<&Map<String, Value>> {
        self.display.get("types")?.get(key)?.as_object()
    }

    pub(crate) fn display_types(&self) -> Vec<(&str, &Map<String, Value>)> {
        self.display
            .get("types")
            .and_then(Value::as_object)
            .map(|types| {
                types
                    .iter()
                    .filter_map(|(key, kind)| Some((key.as_str(), kind.as_object()?)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The field names a kind of page lists in its sections, in order. An
    /// entry is a field name or a table with a `field` key.
    pub(crate) fn type_field_names<'a>(&self, kind: &'a Map<String, Value>) -> Vec<&'a str> {
        let mut out = Vec::new();
        for section in kind
            .get("sections")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            for entry in section
                .get("fields")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let name = match entry {
                    Value::String(name) => Some(name.as_str()),
                    Value::Object(entry) => entry.get("field").and_then(Value::as_str),
                    _ => None,
                };
                if let Some(name) = name
                    && !out.contains(&name)
                {
                    out.push(name);
                }
            }
        }
        out
    }
}

/// One id or a list of ids.
pub(crate) fn string_list(value: &Value) -> Option<Vec<&str>> {
    match value {
        Value::String(id) => Some(vec![id.as_str()]),
        Value::Array(items) => Some(items.iter().filter_map(Value::as_str).collect()),
        _ => None,
    }
}
