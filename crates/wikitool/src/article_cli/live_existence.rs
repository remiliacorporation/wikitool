//! Whether the scouted topic already exists on the configured wiki. The local
//! catalog only knows what has been pulled, so scout asks the live API when it
//! can and otherwise says plainly that it did not check.

use serde::Serialize;
use wikitool_core::config::WikiConfig;
use wikitool_core::mw::client_config_from_wikitool_config;
use wikitool_core::support::parse_redirect;
use wikitool_core::sync::{MediaWikiClient, WikiReadApi};

/// Upper bound for this one advisory read; a slow or unreachable wiki should
/// cost scout seconds, not the default retry budget.
const LIVE_CHECK_TIMEOUT_MS: u64 = 10_000;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(super) enum LiveExistence {
    Exists {
        title: String,
        revision_id: i64,
        api_url: String,
    },
    Redirect {
        title: String,
        target: String,
        api_url: String,
    },
    Missing {
        title: String,
        api_url: String,
    },
    Unchecked {
        reason: String,
    },
}

impl LiveExistence {
    /// `exists`, `redirect (to TARGET)`, `missing` or `unchecked (REASON)`.
    pub(super) fn label(&self) -> String {
        match self {
            Self::Exists { title, .. } => format!("exists ({title})"),
            Self::Redirect { target, .. } => format!("redirect (to {target})"),
            Self::Missing { .. } => "missing".to_string(),
            Self::Unchecked { reason } => format!("unchecked ({reason})"),
        }
    }
}

pub(super) fn check_live_existence(
    config: &WikiConfig,
    topic: Option<&str>,
    disabled: bool,
) -> LiveExistence {
    if disabled {
        return LiveExistence::Unchecked {
            reason: "disabled with --no-live-check; local state only".to_string(),
        };
    }
    let Some(topic) = topic.map(str::trim).filter(|topic| !topic.is_empty()) else {
        return LiveExistence::Unchecked {
            reason: "no topic title to check".to_string(),
        };
    };
    let Some(api_url) = config.api_url_owned().filter(|url| !url.trim().is_empty()) else {
        return LiveExistence::Unchecked {
            reason: "wiki API is not configured; local state only".to_string(),
        };
    };
    let mut client_config = client_config_from_wikitool_config(config);
    client_config.timeout_ms = client_config.timeout_ms.min(LIVE_CHECK_TIMEOUT_MS);
    client_config.max_retries = 0;
    let observed = MediaWikiClient::new(client_config)
        .and_then(|mut client| client.get_page_contents(&[topic.to_string()]));
    match observed {
        Err(error) => LiveExistence::Unchecked {
            reason: format!("live check against {api_url} failed: {error:#}; local state only"),
        },
        Ok(pages) => match pages.into_iter().next() {
            None => LiveExistence::Missing {
                title: topic.to_string(),
                api_url,
            },
            Some(page) => match parse_redirect(&page.content) {
                (true, Some(target)) => LiveExistence::Redirect {
                    title: page.title,
                    target,
                    api_url,
                },
                _ => LiveExistence::Exists {
                    title: page.title,
                    revision_id: page.revision_id,
                    api_url,
                },
            },
        },
    }
}
