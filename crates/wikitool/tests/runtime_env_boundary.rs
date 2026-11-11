//! How Wikitool selects its project root and loads `.env` at the process boundary.
//!
//! Two tests build their own `Command` instead of using the Wikitest harness, because the
//! harness always passes `--project-root` and runs in the project root, which is exactly what
//! they vary: one runs from a directory above the selected project, whose `.env` must not
//! load, and the other passes no `--project-root`, so the root comes from the working
//! directory. Without `--project-root`, Wikitool may also find a root above its own
//! executable, so the harness deliberately offers no way to omit it.

use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::tempdir;
use wikitest::harness::Project;

const WIKITOOL_ENV_KEYS: &[&str] = &[
    "WIKITOOL_PROJECT_ROOT",
    "WIKITOOL_DATA_DIR",
    "WIKITOOL_CONFIG",
    "WIKITOOL_WIKI_URL",
    "WIKITOOL_WIKI_API_URL",
    "WIKITOOL_ARTICLE_PATH",
    "WIKITOOL_USER_AGENT",
    "WIKITOOL_BOT_USER",
    "WIKITOOL_BOT_PASS",
];

#[test]
fn explicit_project_root_does_not_load_an_ancestor_dotenv() {
    let temp = tempdir().expect("tempdir");
    let project = temp.path().join("selected-project");
    fs::create_dir_all(project.join(".wikitool")).expect("project state dir");
    fs::write(
        project.join(".wikitool/config.toml"),
        "[wiki]\nurl = \"https://selected.example\"\napi_url = \"https://selected.example/api.php\"\n",
    )
    .expect("project config");
    fs::write(
        temp.path().join(".env"),
        "WIKITOOL_WIKI_URL=https://wrong.example\nWIKITOOL_WIKI_API_URL=https://wrong.example/api.php\n",
    )
    .expect("hostile ancestor dotenv");

    let mut command = Command::new(env!("CARGO_BIN_EXE_wikitool"));
    command
        .current_dir(temp.path())
        .arg("--project-root")
        .arg(&project)
        .args(["config", "show", "--format", "json"])
        .env("WIKITOOL_SILENT", "1");
    for key in WIKITOOL_ENV_KEYS {
        command.env_remove(key);
    }

    let output = command.output().expect("run wikitool");
    assert!(
        output.status.success(),
        "wikitool failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("config JSON");
    assert_eq!(report["wiki"]["url"]["value"], "https://selected.example");
    assert_eq!(
        report["wiki"]["api_url"]["value"],
        "https://selected.example/api.php"
    );
    assert_eq!(report["wiki"]["api_url"]["source"], "config");
}

#[test]
fn process_environment_overrides_the_selected_project_dotenv() {
    // The harness selects this project with --project-root and removes inherited WIKITOOL_*
    // variables, so the only process value is the one passed here.
    let wt = Project::new(env!("CARGO_BIN_EXE_wikitool"));
    let project = wt.path();
    fs::create_dir_all(project.join(".wikitool")).expect("project state dir");
    fs::write(
        project.join(".wikitool/config.toml"),
        "[wiki]\napi_url = \"https://config.example/api.php\"\n",
    )
    .expect("project config");
    fs::write(
        project.join(".env"),
        "WIKITOOL_WIKI_API_URL=https://dotenv.example/api.php\n",
    )
    .expect("project dotenv");

    let output = wt.run_with_env(
        &["config", "show", "--format", "json"],
        &[
            ("WIKITOOL_SILENT", "1"),
            ("WIKITOOL_WIKI_API_URL", "https://process.example/api.php"),
        ],
    );
    let report = output.assert_code(0).json();
    assert_eq!(
        report["wiki"]["api_url"]["value"],
        "https://process.example/api.php"
    );
    assert_eq!(report["wiki"]["api_url"]["source"], "env");
}

#[test]
fn selected_project_dotenv_cannot_retarget_the_project_root() {
    let temp = tempdir().expect("tempdir");
    let project = temp.path().join("selected-project");
    let redirected = temp.path().join("redirected-project");
    fs::create_dir_all(project.join(".wikitool")).expect("selected state dir");
    fs::create_dir_all(redirected.join(".wikitool")).expect("redirected state dir");
    fs::write(
        project.join(".wikitool/config.toml"),
        "[wiki]\napi_url = \"https://selected.example/api.php\"\n",
    )
    .expect("selected config");
    fs::write(
        redirected.join(".wikitool/config.toml"),
        "[wiki]\napi_url = \"https://redirected.example/api.php\"\n",
    )
    .expect("redirected config");
    fs::write(
        project.join(".env"),
        format!("WIKITOOL_PROJECT_ROOT={}\n", redirected.display()),
    )
    .expect("project dotenv");

    let mut command = Command::new(env!("CARGO_BIN_EXE_wikitool"));
    command
        .current_dir(&project)
        .args(["config", "show", "--format", "json"])
        .env("WIKITOOL_SILENT", "1");
    for key in WIKITOOL_ENV_KEYS {
        command.env_remove(key);
    }

    let output = command.output().expect("run wikitool");
    assert!(
        output.status.success(),
        "wikitool failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("config JSON");
    assert_eq!(
        report["wiki"]["api_url"]["value"],
        "https://selected.example/api.php"
    );
    assert_eq!(
        report["project_root"],
        project.to_string_lossy().replace('\\', "/")
    );
}
