//! `article retype` against a small synthetic ontology: dry-run by default, `--apply draft`
//! writes only the draft, refusals carry their reason (an unknown class with the typing
//! check's code), a wiki without `[ontology]` gets a clear refusal, and no command contacts
//! the wiki.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use wikitest::harness::{FakeWiki, Project};

const EXPORT: &str = "vendor/ontology/ontology.json";
const LOCAL: &str = "templates/ontology/Module_Ontology/local.lua";
const DISPLAY: &str = "templates/ontology/Module_Ontology/display.lua";
const PLAN: &str = "plans/typing.json";

fn export() -> Value {
    json!({
        "classes": {
            "test:Thing": { "label": "Thing" },
            "test:InternetMeme": { "label": "Internet meme", "subClassOf": "test:Thing" }
        },
        "families": {},
        "properties": {},
        "relations": {},
        "schemes": {},
        "mappings": {},
        "renames": { "test:Meme": "test:InternetMeme" },
        "external": {},
        "source": { "commit": "fixture-commit", "product": "test-wiki" }
    })
}

const LOCAL_MODULE: &str = "return {\n\tclasses = {},\n\tmappings = {},\n}\n";

const DISPLAY_MODULE: &str = r#"return {
	fields = {
		movement = { property = 'wdt:P135', label = 'Movement', category = '%s' },
	},
	types = {},
	classes = {
		['test:Thing'] = { category = 'Things' },
		['test:InternetMeme'] = { category = 'Internet memes' },
	},
	existingCategories = {},
}
"#;

const WOJAK: &str = "\
{{SHORTDESC:A synthetic meme}}
'''Wojak''' is a synthetic page.

[[Category:Memes]]
[[Category:Internet memes]]
[[Category:Topics]]
";

const WOJAK_RETYPED: &str = "\
{{SHORTDESC:A synthetic meme}}
{{Instance of|test:InternetMeme|wikidata=Q2622436}}
'''Wojak''' is a synthetic page.

[[Category:Topics]]
";

const PEPE: &str = "\
{{SHORTDESC:Another synthetic meme}}
'''Pepe''' is a synthetic page.
";

fn plan() -> Value {
    json!({
        "schema": "wikitool.article-retype-plan.v1",
        "items": [
            {
                "title": "Wojak",
                "classes": ["test:InternetMeme"],
                "relations": { "movement": [] },
                "wikidata": "Q2622436",
                "remove_categories": ["Memes"],
                "add_categories": [],
                "drop_generated": true
            },
            { "title": "Pepe", "classes": ["test:Nope"] },
            { "title": "Old Wojak", "classes": ["test:InternetMeme"] },
            { "title": "Missing page", "add_categories": ["Topics"] }
        ]
    })
}

fn generic_adapter() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../site_adapters/generic/site-adapter.toml"),
    )
    .expect("generic adapter")
}

fn write(wt: &Project<'_>, relative: &str, text: &str) {
    let path = wt.path().join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(path, text).expect("write fixture");
}

/// A project against `wiki`, with `[ontology]` in its adapter when `ontology` is set.
fn project(wiki: &FakeWiki, ontology: bool) -> Project<'_> {
    let wt = Project::with_wiki(env!("CARGO_BIN_EXE_wikitool"), wiki);
    let mut adapter = generic_adapter();
    if ontology {
        adapter.push_str(
            "\n[ontology]\n\
             product = \"test-wiki\"\n\
             source = \"https://ontology.example/\"\n\
             local_prefix = \"test\"\n\
             local_namespace = \"https://wiki.example/ns/\"\n\
             class_template = \"Template:Instance of\"\n",
        );
    }
    write(&wt, "site-adapter/site-adapter.toml", &adapter);
    wt.run(&[
        "init",
        "--no-network",
        "--api-url",
        wiki.api_url(),
        "--adapter-path",
        "site-adapter/site-adapter.toml",
    ])
    .assert_code(0);
    write(
        &wt,
        EXPORT,
        &serde_json::to_string_pretty(&export()).expect("json"),
    );
    write(&wt, LOCAL, LOCAL_MODULE);
    write(&wt, DISPLAY, DISPLAY_MODULE);
    write(&wt, "wiki_content/Main/Wojak.wiki", WOJAK);
    write(&wt, "wiki_content/Main/Pepe.wiki", PEPE);
    write(
        &wt,
        "wiki_content/Main/_redirects/Old_Wojak.wiki",
        "#REDIRECT [[Wojak]]\n",
    );
    write(
        &wt,
        PLAN,
        &serde_json::to_string_pretty(&plan()).expect("json"),
    );
    wt
}

fn wiki() -> FakeWiki {
    FakeWiki::from_file(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../wikitest/scenarios/mechanical/mediawiki-sync/fixtures/mediawiki.json"),
    )
}

/// Every file in the project, by relative path, except the harness's own process output.
fn snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory).expect("read dir") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                if path
                    .file_name()
                    .is_some_and(|name| name == ".wikitest-private")
                {
                    continue;
                }
                stack.push(path);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .expect("relative")
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(relative, fs::read(&path).expect("read file"));
            }
        }
    }
    out
}

fn changed_files(
    before: &BTreeMap<String, Vec<u8>>,
    after: &BTreeMap<String, Vec<u8>>,
) -> Vec<String> {
    let mut out: Vec<String> = after
        .iter()
        .filter(|(path, bytes)| before.get(*path) != Some(*bytes))
        .map(|(path, _)| path.clone())
        .collect();
    out.extend(
        before
            .keys()
            .filter(|path| !after.contains_key(*path))
            .cloned(),
    );
    out
}

fn sha256(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn assert_no_requests(wiki: &FakeWiki) {
    let observed = wiki.observe();
    assert!(
        observed.requests.is_empty(),
        "article retype contacted the wiki: {:#?}",
        observed.requests
    );
}

#[test]
fn dry_run_reports_each_edit_and_writes_nothing() {
    let wiki = wiki();
    let wt = project(&wiki, true);
    write(&wt, "plans/titles.txt", "Wojak\n");
    let before = snapshot(wt.path());
    let output = wt.run(&[
        "article",
        "retype",
        "--plan",
        PLAN,
        "--titles-file",
        "plans/titles.txt",
        "--format",
        "json",
    ]);
    output
        .assert_code(0)
        .assert_json("/schema", "wikitool.article-retype.v1")
        .assert_json("/status", "complete")
        .assert_json("/apply_mode", "none")
        .assert_json("/ontology_commit", "fixture-commit")
        .assert_json("/class_template", "Template:Instance of")
        .assert_json("/page_count", 1)
        .assert_json("/changed_count", 1)
        .assert_json("/drafts_written", 0)
        .assert_json("/pages/0/title", "Wojak")
        .assert_json("/pages/0/status", "changed")
        .assert_json("/pages/0/source_kind", "mirror")
        .assert_json("/pages/0/source_path", "wiki_content/Main/Wojak.wiki")
        .assert_json("/pages/0/draft_path", ".wikitool/drafts/Wojak.wiki")
        .assert_json("/pages/0/draft_written", false)
        .assert_json("/pages/0/source_sha256", sha256(WOJAK))
        .assert_json("/pages/0/result_sha256", sha256(WOJAK_RETYPED))
        .assert_json("/pages/0/refusals", json!([]));
    let report = output.json();
    let edits = report["pages"][0]["edits"].as_array().expect("edits");
    let kinds: Vec<&str> = edits
        .iter()
        .map(|edit| edit["kind"].as_str().expect("kind"))
        .collect();
    assert_eq!(
        kinds,
        vec![
            "class_statement_insert",
            "category_remove",
            "category_drop_generated"
        ]
    );
    // The edits are exact: swapping each new text back for its old one gives the source.
    let mut restored = WOJAK_RETYPED.to_string();
    for edit in edits.iter().rev() {
        let start = edit["result_start"].as_u64().expect("start") as usize;
        let end = edit["result_end"].as_u64().expect("end") as usize;
        assert_eq!(&restored[start..end], edit["new"].as_str().expect("new"));
        restored.replace_range(start..end, edit["old"].as_str().expect("old"));
    }
    assert_eq!(restored, WOJAK);
    assert_eq!(edits[1]["old"], "[[Category:Memes]]\n");
    assert_eq!(edits[2]["categories"], json!(["Internet memes"]));
    let diff = report["pages"][0]["diff"].as_str().expect("diff");
    assert!(diff.contains("+{{Instance of|test:InternetMeme|wikidata=Q2622436}}"));
    assert!(diff.contains("-[[Category:Memes]]"));
    assert_eq!(
        report["pages"][0]["typing"]["checks_run"],
        json!(["class_statements", "term_definitions"])
    );
    assert_eq!(report["pages"][0]["typing"]["findings"], json!([]));

    assert_eq!(
        changed_files(&before, &snapshot(wt.path())),
        Vec::<String>::new()
    );
    wt.assert_missing(".wikitool/drafts");

    // The text form reports the same and also writes nothing.
    wt.run(&[
        "article",
        "retype",
        "wiki_content/Main/Wojak.wiki",
        "--plan",
        PLAN,
    ])
    .assert_code(0)
    .assert_stdout_contains("status: changed")
    .assert_stdout_contains("edit: category_remove [Memes]")
    .assert_stdout_contains("+{{Instance of|test:InternetMeme|wikidata=Q2622436}}");
    assert_eq!(
        changed_files(&before, &snapshot(wt.path())),
        Vec::<String>::new()
    );
    assert_no_requests(&wiki);
}

#[test]
fn apply_draft_writes_only_the_draft_and_a_rerun_reads_it() {
    let wiki = wiki();
    let wt = project(&wiki, true);
    let before = snapshot(wt.path());
    wt.run(&[
        "article",
        "retype",
        "wiki_content/Main/Wojak.wiki",
        "--plan",
        PLAN,
        "--apply",
        "draft",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/apply_mode", "draft")
    .assert_json("/drafts_written", 1)
    .assert_json("/pages/0/draft_written", true);
    assert_eq!(
        changed_files(&before, &snapshot(wt.path())),
        vec![".wikitool/drafts/Wojak.wiki".to_string()]
    );
    assert_eq!(wt.read(".wikitool/drafts/Wojak.wiki"), WOJAK_RETYPED);
    assert_eq!(wt.read("wiki_content/Main/Wojak.wiki"), WOJAK);

    // The draft is now the page's source. Its removal is done, so the same plan refuses it
    // and changes nothing; a plan without the removal leaves it as it is.
    let after_apply = snapshot(wt.path());
    wt.run(&[
        "article",
        "retype",
        ".wikitool/drafts/Wojak.wiki",
        "--plan",
        PLAN,
        "--apply",
        "draft",
        "--format",
        "json",
    ])
    .assert_code(1)
    .assert_json("/pages/0/source_kind", "draft")
    .assert_json("/pages/0/source_path", ".wikitool/drafts/Wojak.wiki")
    .assert_json("/pages/0/status", "refused")
    .assert_json("/pages/0/refusals/0/code", "removal_missing")
    .assert_json("/pages/0/draft_written", false);
    let mut settled = plan();
    settled["items"][0]["remove_categories"] = json!([]);
    write(
        &wt,
        "plans/settled.json",
        &serde_json::to_string_pretty(&settled).expect("json"),
    );
    let after_settled_plan = snapshot(wt.path());
    wt.run(&[
        "article",
        "retype",
        ".wikitool/drafts/Wojak.wiki",
        "--plan",
        "plans/settled.json",
        "--apply",
        "draft",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/pages/0/status", "unchanged")
    .assert_json("/pages/0/edits", json!([]))
    .assert_json("/pages/0/diff", "");
    assert_eq!(
        changed_files(&after_settled_plan, &snapshot(wt.path())),
        Vec::<String>::new()
    );
    assert_eq!(after_apply.len() + 1, after_settled_plan.len());
    assert_no_requests(&wiki);
}

#[test]
fn refusals_carry_their_reason_and_change_nothing() {
    let wiki = wiki();
    let wt = project(&wiki, true);
    let before = snapshot(wt.path());
    let output = wt.run(&[
        "article", "retype", "--plan", PLAN, "--apply", "draft", "--format", "json",
    ]);
    output
        .assert_code(1)
        .assert_json("/page_count", 4)
        .assert_json("/changed_count", 1)
        .assert_json("/refused_count", 3)
        .assert_json("/drafts_written", 1)
        .assert_json("/pages/1/title", "Pepe")
        .assert_json("/pages/1/status", "refused")
        .assert_json("/pages/1/refusals/0/code", "ontology.unknown_class")
        .assert_json("/pages/1/refusals/0/term", "test:Nope")
        .assert_json("/pages/1/result_sha256", sha256(PEPE))
        .assert_json("/pages/1/edits", json!([]))
        .assert_json("/pages/1/draft_written", false)
        .assert_json("/pages/2/title", "Old Wojak")
        .assert_json("/pages/2/refusals/0/code", "redirect")
        .assert_json("/pages/3/title", "Missing page")
        .assert_json("/pages/3/refusals/0/code", "page_missing");
    assert!(
        output
            .json_str("/pages/1/refusals/0/message")
            .contains("test:Nope")
    );
    output.assert_stderr_contains("article retype refused 3 of 4 page(s)");
    assert_eq!(
        changed_files(&before, &snapshot(wt.path())),
        vec![".wikitool/drafts/Wojak.wiki".to_string()]
    );

    // A renamed id is refused with the typing check's code too, and a title the plan does
    // not list is reported rather than skipped.
    let mut renamed = plan();
    renamed["items"][1]["classes"] = json!(["test:Meme"]);
    write(
        &wt,
        "plans/renamed.json",
        &serde_json::to_string_pretty(&renamed).expect("json"),
    );
    write(&wt, "plans/titles.txt", "Pepe\nNot planned\n");
    wt.run(&[
        "article",
        "retype",
        "--plan",
        "plans/renamed.json",
        "--titles-file",
        "plans/titles.txt",
        "--format",
        "json",
    ])
    .assert_code(1)
    .assert_json("/pages/0/refusals/0/code", "ontology.renamed_term")
    .assert_json("/pages/1/title", "Not planned")
    .assert_json("/pages/1/refusals/0/code", "not_in_plan");
    assert_no_requests(&wiki);
}

#[test]
fn a_wiki_without_ontology_gets_a_clear_refusal() {
    let wiki = wiki();
    let wt = project(&wiki, false);
    let before = snapshot(wt.path());
    let output = wt.run(&["article", "retype", "--plan", PLAN, "--format", "json"]);
    output
        .assert_code(1)
        .assert_json("/status", "not_ready")
        .assert_json("/pages", json!([]))
        .assert_stderr_contains("has no [ontology] section");
    assert!(output.json_str("/reason").contains("class_template"));
    wt.run(&["article", "retype", "--plan", PLAN])
        .assert_code(1)
        .assert_stdout_contains("status: not_ready")
        .assert_stderr_contains("article retype cannot run");
    assert_eq!(
        changed_files(&before, &snapshot(wt.path())),
        Vec::<String>::new()
    );

    // A malformed plan is an error before any page is read.
    write(
        &wt,
        "plans/bad.json",
        "{\"schema\": \"other\", \"items\": []}",
    );
    wt.run(&["article", "retype", "--plan", "plans/bad.json"])
        .assert_code(1)
        .assert_stderr_contains("wikitool.article-retype-plan.v1");
    assert_no_requests(&wiki);
}

/// A PATH is retyped exactly: a draft with a free name is read and written in place, and a
/// mirror page whose title has a draft is refused rather than retyped over the draft's edits.
#[test]
fn a_path_names_exactly_the_file_retyped() {
    let wiki = wiki();
    let wt = project(&wiki, true);
    let draft = ".wikitool/drafts/Miya Notes.wiki";
    let human = "{{SHORTDESC:Notes}}\n'''Miya''' has human edits.\n\n[[Category:Memes]]\n";
    write(&wt, draft, human);
    write(
        &wt,
        "wiki_content/Main/Miya_Notes.wiki",
        "{{SHORTDESC:Notes}}\n'''Miya''' as pulled.\n\n[[Category:Memes]]\n",
    );
    let mut with_miya = plan();
    with_miya["items"]
        .as_array_mut()
        .expect("items")
        .push(json!({"title": "Miya Notes", "classes": ["test:InternetMeme"]}));
    write(
        &wt,
        "plans/miya.json",
        &serde_json::to_string_pretty(&with_miya).expect("json"),
    );
    let before = snapshot(wt.path());
    wt.run(&[
        "article",
        "retype",
        draft,
        "--plan",
        "plans/miya.json",
        "--apply",
        "draft",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/pages/0/title", "Miya Notes")
    .assert_json("/pages/0/source_kind", "draft")
    .assert_json("/pages/0/source_path", draft)
    .assert_json("/pages/0/draft_path", draft)
    .assert_json("/pages/0/source_sha256", sha256(human))
    .assert_json("/pages/0/draft_written", true);
    assert_eq!(
        changed_files(&before, &snapshot(wt.path())),
        vec![draft.to_string()]
    );
    assert_eq!(
        wt.read(draft),
        "{{SHORTDESC:Notes}}\n{{Instance of|test:InternetMeme}}\n'''Miya''' has human edits.\n\n[[Category:Memes]]\n"
    );

    let before = snapshot(wt.path());
    wt.run(&[
        "article",
        "retype",
        "wiki_content/Main/Miya_Notes.wiki",
        "--plan",
        "plans/miya.json",
        "--apply",
        "draft",
        "--format",
        "json",
    ])
    .assert_code(1)
    .assert_json("/pages/0/status", "refused")
    .assert_json("/pages/0/refusals/0/code", "draft_exists");
    assert_eq!(
        changed_files(&before, &snapshot(wt.path())),
        Vec::<String>::new()
    );
    assert_no_requests(&wiki);
}

/// A page in a custom namespace, a title that spells a path, and a titles file that starts
/// with a byte order mark.
#[test]
fn titles_outside_the_articles_are_refused_and_a_titles_file_may_start_with_a_bom() {
    let wiki = wiki();
    let wt = project(&wiki, true);
    write(&wt, "wiki_content/Goldenlight/Wojak.wiki", WOJAK);
    let odd = json!({
        "schema": "wikitool.article-retype-plan.v1",
        "items": [
            { "title": "Goldenlight:Wojak", "classes": ["test:InternetMeme"] },
            { "title": "..\\..\\wiki_content\\Main\\Wojak", "classes": ["test:InternetMeme"] }
        ]
    });
    write(
        &wt,
        "plans/odd.json",
        &serde_json::to_string_pretty(&odd).expect("json"),
    );
    let before = snapshot(wt.path());
    wt.run(&[
        "article",
        "retype",
        "--plan",
        "plans/odd.json",
        "--apply",
        "draft",
        "--format",
        "json",
    ])
    .assert_code(1)
    .assert_json("/refused_count", 2)
    .assert_json("/pages/0/refusals/0/code", "not_an_article")
    .assert_json("/pages/1/refusals/0/code", "invalid_title");
    assert_eq!(
        changed_files(&before, &snapshot(wt.path())),
        Vec::<String>::new()
    );

    write(&wt, "plans/titles.txt", "\u{feff}Wojak\r\n");
    wt.run(&[
        "article",
        "retype",
        "--plan",
        PLAN,
        "--titles-file",
        "plans/titles.txt",
        "--format",
        "json",
    ])
    .assert_code(0)
    .assert_json("/pages/0/title", "Wojak")
    .assert_json("/pages/0/status", "changed");
    assert_no_requests(&wiki);
}

/// Every page is read before any draft is written: a page that cannot be read, or whose draft
/// cannot be written, is a refusal in the full report, and the other drafts are still reported.
#[test]
fn unreadable_and_unwritable_pages_are_refusals_in_the_full_report() {
    let wiki = wiki();
    let wt = project(&wiki, true);
    // Pepe's draft is Latin-1, not UTF-8.
    let path = wt.path().join(".wikitool/drafts/Pepe.wiki");
    fs::create_dir_all(path.parent().expect("parent")).expect("drafts");
    fs::write(&path, b"'''Pep\xe9''' is a synthetic page.\n").expect("latin-1 draft");
    let output = wt.run(&[
        "article", "retype", "--plan", PLAN, "--apply", "draft", "--format", "json",
    ]);
    output
        .assert_code(1)
        .assert_json("/page_count", 4)
        .assert_json("/drafts_written", 1)
        .assert_json("/pages/0/title", "Wojak")
        .assert_json("/pages/0/draft_written", true)
        .assert_json("/pages/1/title", "Pepe")
        .assert_json("/pages/1/status", "refused")
        .assert_json("/pages/1/refusals/0/code", "read_failed")
        .assert_json("/pages/1/source_path", ".wikitool/drafts/Pepe.wiki");
    assert_eq!(wt.read(".wikitool/drafts/Wojak.wiki"), WOJAK_RETYPED);

    // A directory where Wojak's draft would go cannot be written.
    let wiki = self::wiki();
    let wt = project(&wiki, true);
    fs::create_dir_all(wt.path().join(".wikitool/drafts/Wojak.wiki")).expect("blocking dir");
    write(&wt, "plans/titles.txt", "Wojak\n");
    wt.run(&[
        "article",
        "retype",
        "--plan",
        PLAN,
        "--titles-file",
        "plans/titles.txt",
        "--apply",
        "draft",
        "--format",
        "json",
    ])
    .assert_code(1)
    .assert_json("/drafts_written", 0)
    .assert_json("/refused_count", 1)
    .assert_json("/pages/0/status", "refused")
    .assert_json("/pages/0/refusals/0/code", "write_failed")
    .assert_json("/pages/0/edits", json!([]));
    assert_no_requests(&wiki);
}
