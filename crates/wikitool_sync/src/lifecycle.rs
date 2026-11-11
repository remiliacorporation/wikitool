//! Typed lifecycle of durable remote-mutation receipts.
//!
//! SQLite stores every value below as its `as_str` spelling, and the schema's
//! CHECK lists name the same spellings. Rows are decoded here when a receipt is
//! read, so an unknown stored string is an error that names its mutation; values
//! become strings again only where they are bound as SQL parameters (`ToSql`).
//! The phase guards in `WHERE` clauses are rendered from [`PhaseGuard`], so the
//! Rust lifecycle and the durable SQL enforcement share one definition.

use std::fmt;

use anyhow::Result;
use rusqlite::ToSql;
use rusqlite::types::{ToSqlOutput, ValueRef};

/// A stored string outside its lifecycle vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnknownStoredValue {
    vocabulary: &'static str,
    value: String,
}

impl UnknownStoredValue {
    /// Name the receipt that holds the unknown value.
    pub(crate) fn in_mutation(self, operation: &str, mutation_id: i64) -> anyhow::Error {
        anyhow::anyhow!(
            "{operation} mutation {mutation_id} has unknown {} {:?}",
            self.vocabulary,
            self.value
        )
    }
}

macro_rules! stored_vocabulary {
    (
        $(#[$meta:meta])*
        $name:ident, $vocabulary:literal {
            $($(#[$variant_meta:meta])* $variant:ident => $text:literal,)+
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(crate) enum $name {
            $($(#[$variant_meta])* $variant,)+
        }

        impl $name {
            #[cfg(test)]
            pub(crate) const ALL: &'static [Self] = &[$(Self::$variant,)+];

            pub(crate) fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text,)+
                }
            }

            pub(crate) fn parse(value: &str) -> Result<Self, UnknownStoredValue> {
                match value {
                    $($text => Ok(Self::$variant),)+
                    _ => Err(UnknownStoredValue {
                        vocabulary: $vocabulary,
                        value: value.to_string(),
                    }),
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(ToSqlOutput::Borrowed(ValueRef::Text(self.as_str().as_bytes())))
            }
        }
    };
}

stored_vocabulary! {
    /// Where a receipt stands between its durable intent and a terminal outcome.
    MutationPhase, "phase" {
        /// The intent is durable; the single request may not have been sent.
        IntentPersisted => "intent_persisted",
        /// A typed MediaWiki response is bound; sync state has not advanced.
        ResponseBound => "response_bound",
        /// The request may have applied and nothing observed proves either way.
        OutcomeAmbiguous => "outcome_ambiguous",
        /// Evidence conflicts or local effects remain; reconcile or close it.
        ReconciliationRequired => "reconciliation_required",
        /// Terminal without advancing sync state; the terminal outcome says why.
        Resolved => "resolved",
        /// Terminal: the verified remote change advanced sync state.
        StateAdvanced => "state_advanced",
    }
}

stored_vocabulary! {
    /// Why an edit receipt became terminal.
    EditTerminalOutcome, "terminal outcome" {
        StateAdvanced => "state_advanced",
        NotApplied => "not_applied",
        AppliedThenChanged => "applied_then_changed",
        OperatorClosedUnresolved => "operator_closed_unresolved",
    }
}

stored_vocabulary! {
    /// Why a delete receipt became terminal.
    DeleteTerminalOutcome, "terminal outcome" {
        StateAdvanced => "state_advanced",
        NotApplied => "not_applied",
        AppliedThenRecreated => "applied_then_recreated",
        RemotePresentAfterMissing => "remote_present_after_missing",
        OperatorClosedUnresolved => "operator_closed_unresolved",
    }
}

stored_vocabulary! {
    /// Why an upload receipt became terminal.
    UploadTerminalOutcome, "terminal outcome" {
        Uploaded => "uploaded",
        NotApplied => "not_applied",
        AppliedThenChanged => "applied_then_changed",
    }
}

stored_vocabulary! {
    /// Progress of a delete receipt's bound local source and backup.
    DeleteLocalEffectStatus, "local-effect status" {
        NotApplicable => "not_applicable",
        Pending => "pending",
        BackupReady => "backup_ready",
        SourceStaging => "source_staging",
        SourceStaged => "source_staged",
        Complete => "complete",
    }
}

stored_vocabulary! {
    /// The typed MediaWiki answer bound to a delete receipt.
    DeleteResponseKind, "response kind" {
        Deleted => "deleted",
        AlreadyMissing => "already_missing",
    }
}

impl MutationPhase {
    /// Phases after which a receipt never changes again.
    pub(crate) const TERMINAL: [Self; 2] = [Self::StateAdvanced, Self::Resolved];

    /// Upload receipts never bind a response or advance page-sync state.
    pub(crate) const UPLOAD: [Self; 4] = [
        Self::IntentPersisted,
        Self::OutcomeAmbiguous,
        Self::ReconciliationRequired,
        Self::Resolved,
    ];

    pub(crate) fn is_terminal(self) -> bool {
        Self::TERMINAL.contains(&self)
    }

    fn admitted_for_upload(self) -> Result<Self, UnknownStoredValue> {
        if Self::UPLOAD.contains(&self) {
            Ok(self)
        } else {
            Err(UnknownStoredValue {
                vocabulary: "upload phase",
                value: self.as_str().to_string(),
            })
        }
    }
}

impl DeleteLocalEffectStatus {
    /// Statuses in which the receipt owns a moved local source, which neither
    /// operator closure nor terminal recovery may discard.
    const STAGED_SOURCE: [Self; 2] = [Self::SourceStaging, Self::SourceStaged];

    pub(crate) fn owns_staged_source(self) -> bool {
        Self::STAGED_SOURCE.contains(&self)
    }

    /// The `WHERE` predicate that refuses a receipt owning a staged source.
    pub(crate) fn no_staged_source_sql() -> String {
        format!(
            "local_effect_status NOT IN ({})",
            sql_text_list(Self::STAGED_SOURCE.map(Self::as_str))
        )
    }
}

/// The two phases a receipt enters while its remote outcome is unsettled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnresolvedPhase {
    OutcomeAmbiguous,
    ReconciliationRequired,
}

impl UnresolvedPhase {
    pub(crate) fn phase(self) -> MutationPhase {
        match self {
            Self::OutcomeAmbiguous => MutationPhase::OutcomeAmbiguous,
            Self::ReconciliationRequired => MutationPhase::ReconciliationRequired,
        }
    }
}

impl fmt::Display for UnresolvedPhase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.phase().as_str())
    }
}

/// The phases a guarded receipt write may start from. Each guard is enforced
/// durably by the `WHERE` predicate [`PhaseGuard::sql`] renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PhaseGuard {
    /// Any phase that is not terminal.
    Unresolved,
    /// Only before the single request is sent.
    IntentPersisted,
    /// A typed response may be bound before advancement, including after an
    /// ambiguous or conflicting observation.
    ResponseBindable,
    /// Edit advancement needs a bound response.
    ResponseBound,
    /// Delete advancement: a bound response, or a retry of staged local cleanup.
    DeleteAdvanceable,
    /// Completion of staged delete cleanup.
    ReconciliationRequired,
}

impl PhaseGuard {
    pub(crate) fn sql(self) -> String {
        let admitted: &[MutationPhase] = match self {
            Self::Unresolved => {
                return format!(
                    "phase NOT IN ({})",
                    sql_text_list(MutationPhase::TERMINAL.map(MutationPhase::as_str))
                );
            }
            Self::IntentPersisted => &[MutationPhase::IntentPersisted],
            Self::ResponseBindable => &[
                MutationPhase::IntentPersisted,
                MutationPhase::OutcomeAmbiguous,
                MutationPhase::ReconciliationRequired,
            ],
            Self::ResponseBound => &[MutationPhase::ResponseBound],
            Self::DeleteAdvanceable => &[
                MutationPhase::ResponseBound,
                MutationPhase::ReconciliationRequired,
            ],
            Self::ReconciliationRequired => &[MutationPhase::ReconciliationRequired],
        };
        match admitted {
            [only] => format!("phase = '{only}'"),
            several => format!(
                "phase IN ({})",
                sql_text_list(several.iter().map(|phase| phase.as_str()))
            ),
        }
    }
}

/// A guarded phase change. Edit receipts use `BindResponse`, `MarkUnresolved`,
/// `Resolve` and `AdvanceEdit`; delete receipts use `BindResponse`,
/// `MarkUnresolved`, `Resolve`, `AdvanceDelete` and `FinishDeleteCleanup`;
/// upload receipts use only `MarkUnresolved` and `Resolve`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PhaseTransition {
    BindResponse,
    MarkUnresolved(UnresolvedPhase),
    /// Terminal without advancing sync state.
    Resolve,
    AdvanceEdit,
    /// A delete whose local source is still staged stays unresolved until the
    /// staged file is unlinked.
    AdvanceDelete {
        local_source_staged: bool,
    },
    FinishDeleteCleanup,
}

impl PhaseTransition {
    pub(crate) fn guard(self) -> PhaseGuard {
        match self {
            Self::BindResponse => PhaseGuard::ResponseBindable,
            Self::MarkUnresolved(_) | Self::Resolve => PhaseGuard::Unresolved,
            Self::AdvanceEdit => PhaseGuard::ResponseBound,
            Self::AdvanceDelete { .. } => PhaseGuard::DeleteAdvanceable,
            Self::FinishDeleteCleanup => PhaseGuard::ReconciliationRequired,
        }
    }

    pub(crate) fn target(self) -> MutationPhase {
        match self {
            Self::BindResponse => MutationPhase::ResponseBound,
            Self::MarkUnresolved(phase) => phase.phase(),
            Self::Resolve => MutationPhase::Resolved,
            Self::AdvanceDelete {
                local_source_staged: true,
            } => MutationPhase::ReconciliationRequired,
            Self::AdvanceEdit
            | Self::AdvanceDelete {
                local_source_staged: false,
            }
            | Self::FinishDeleteCleanup => MutationPhase::StateAdvanced,
        }
    }
}

fn sql_text_list<'a>(values: impl IntoIterator<Item = &'a str>) -> String {
    values
        .into_iter()
        .map(|value| format!("'{value}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Decoded lifecycle columns of one `sync_edit_mutations` row.
pub(crate) fn decode_edit_lifecycle(
    mutation_id: i64,
    phase: &str,
    terminal_outcome: Option<&str>,
) -> Result<(MutationPhase, Option<EditTerminalOutcome>)> {
    let decoded = MutationPhase::parse(phase).and_then(|phase| {
        let outcome = terminal_outcome
            .map(EditTerminalOutcome::parse)
            .transpose()?;
        Ok((phase, outcome))
    });
    decoded.map_err(|unknown| unknown.in_mutation("edit", mutation_id))
}

/// Decoded lifecycle columns of one `sync_delete_mutations` row.
#[derive(Debug)]
pub(crate) struct DeleteLifecycle {
    pub phase: MutationPhase,
    pub response_kind: Option<DeleteResponseKind>,
    pub terminal_outcome: Option<DeleteTerminalOutcome>,
    pub local_effect_status: DeleteLocalEffectStatus,
}

pub(crate) fn decode_delete_lifecycle(
    mutation_id: i64,
    phase: &str,
    response_kind: Option<&str>,
    terminal_outcome: Option<&str>,
    local_effect_status: &str,
) -> Result<DeleteLifecycle> {
    let decoded = (|| -> Result<DeleteLifecycle, UnknownStoredValue> {
        Ok(DeleteLifecycle {
            phase: MutationPhase::parse(phase)?,
            response_kind: response_kind.map(DeleteResponseKind::parse).transpose()?,
            terminal_outcome: terminal_outcome
                .map(DeleteTerminalOutcome::parse)
                .transpose()?,
            local_effect_status: DeleteLocalEffectStatus::parse(local_effect_status)?,
        })
    })();
    decoded.map_err(|unknown| unknown.in_mutation("delete", mutation_id))
}

/// Decoded lifecycle columns of one `sync_upload_mutations` row.
pub(crate) fn decode_upload_lifecycle(
    mutation_id: i64,
    phase: &str,
    terminal_outcome: Option<&str>,
) -> Result<(MutationPhase, Option<UploadTerminalOutcome>)> {
    let decoded = MutationPhase::parse(phase)
        .and_then(MutationPhase::admitted_for_upload)
        .and_then(|phase| {
            let outcome = terminal_outcome
                .map(UploadTerminalOutcome::parse)
                .transpose()?;
            Ok((phase, outcome))
        });
    decoded.map_err(|unknown| unknown.in_mutation("upload", mutation_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_round_trips<T: Copy + PartialEq + fmt::Debug>(
        all: &[T],
        as_str: fn(T) -> &'static str,
        parse: fn(&str) -> Result<T, UnknownStoredValue>,
    ) {
        for value in all {
            assert_eq!(parse(as_str(*value)), Ok(*value));
        }
        let error = parse("not_a_stored_value").expect_err("unknown spelling is rejected");
        assert_eq!(error.value, "not_a_stored_value");
    }

    #[test]
    fn every_stored_vocabulary_round_trips_and_rejects_unknown_strings() {
        assert_round_trips(
            MutationPhase::ALL,
            MutationPhase::as_str,
            MutationPhase::parse,
        );
        assert_round_trips(
            EditTerminalOutcome::ALL,
            EditTerminalOutcome::as_str,
            EditTerminalOutcome::parse,
        );
        assert_round_trips(
            DeleteTerminalOutcome::ALL,
            DeleteTerminalOutcome::as_str,
            DeleteTerminalOutcome::parse,
        );
        assert_round_trips(
            UploadTerminalOutcome::ALL,
            UploadTerminalOutcome::as_str,
            UploadTerminalOutcome::parse,
        );
        assert_round_trips(
            DeleteLocalEffectStatus::ALL,
            DeleteLocalEffectStatus::as_str,
            DeleteLocalEffectStatus::parse,
        );
        assert_round_trips(
            DeleteResponseKind::ALL,
            DeleteResponseKind::as_str,
            DeleteResponseKind::parse,
        );
    }

    #[test]
    fn unknown_stored_values_name_their_mutation() {
        let error = decode_delete_lifecycle(7, "resolved", None, Some("vanished"), "complete")
            .expect_err("unknown outcome is rejected");
        assert_eq!(
            error.to_string(),
            "delete mutation 7 has unknown terminal outcome \"vanished\""
        );
        let error = decode_upload_lifecycle(3, "state_advanced", Some("uploaded"))
            .expect_err("an upload receipt cannot advance page-sync state");
        assert_eq!(
            error.to_string(),
            "upload mutation 3 has unknown upload phase \"state_advanced\""
        );
    }

    /// These are the literal guards the SQL used before it was rendered from
    /// the lifecycle; the unique unresolved-title indexes in the schema DDL
    /// still spell the first one literally.
    #[test]
    fn rendered_guards_equal_the_durable_sql_predicates() {
        assert_eq!(
            PhaseGuard::Unresolved.sql(),
            "phase NOT IN ('state_advanced', 'resolved')"
        );
        assert_eq!(
            PhaseGuard::IntentPersisted.sql(),
            "phase = 'intent_persisted'"
        );
        assert_eq!(
            PhaseGuard::ResponseBindable.sql(),
            "phase IN ('intent_persisted', 'outcome_ambiguous', 'reconciliation_required')"
        );
        assert_eq!(PhaseGuard::ResponseBound.sql(), "phase = 'response_bound'");
        assert_eq!(
            PhaseGuard::DeleteAdvanceable.sql(),
            "phase IN ('response_bound', 'reconciliation_required')"
        );
        assert_eq!(
            PhaseGuard::ReconciliationRequired.sql(),
            "phase = 'reconciliation_required'"
        );
        assert_eq!(
            DeleteLocalEffectStatus::no_staged_source_sql(),
            "local_effect_status NOT IN ('source_staging', 'source_staged')"
        );
    }
}
