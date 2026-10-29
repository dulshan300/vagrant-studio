//! One-time confirmation tokens for destructive operations.
//!
//! The UI asks for a token for an exact target, shows the phrase the user
//! must type, and sends both back with the request. The engine refuses a
//! destructive command without a matching, unexpired token, so a UI bug
//! can't destroy machines that weren't confirmed.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use vagrant_studio_domain::EnvironmentId;
use vagrant_studio_vagrant_adapter::VagrantCommand;

const VALIDITY: Duration = Duration::minutes(10);

/// Returned to the UI when it asks to confirm a destructive operation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Confirmation {
    pub token: String,
    /// What the user has to type.
    pub phrase: String,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
}

/// Sent back with the destructive request.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmationProof {
    pub token: String,
    pub typed: String,
}

struct Pending {
    /// `None` for host-level commands such as `box remove`.
    environment_id: Option<EnvironmentId>,
    command: VagrantCommand,
    phrase: String,
    expires_at: OffsetDateTime,
}

#[derive(Default)]
pub(crate) struct Confirmations {
    pending: Mutex<HashMap<String, Pending>>,
}

impl Confirmations {
    pub(crate) fn issue(
        &self,
        environment_id: Option<EnvironmentId>,
        command: VagrantCommand,
        phrase: String,
    ) -> Confirmation {
        let now = OffsetDateTime::now_utc();
        let token = uuid::Uuid::new_v4().simple().to_string();
        let expires_at = now + VALIDITY;
        let mut pending = self.pending.lock().expect("lock");
        pending.retain(|_, p| p.expires_at > now);
        pending.insert(
            token.clone(),
            Pending {
                environment_id,
                command,
                phrase: phrase.clone(),
                expires_at,
            },
        );
        Confirmation {
            token,
            phrase,
            expires_at,
        }
    }

    /// Checks and consumes a token. Any attempt uses it up.
    pub(crate) fn redeem(
        &self,
        proof: &ConfirmationProof,
        environment_id: Option<EnvironmentId>,
        command: &VagrantCommand,
    ) -> Result<(), &'static str> {
        let pending = self.pending.lock().expect("lock").remove(&proof.token);
        let Some(p) = pending else {
            return Err("This confirmation is no longer valid. Confirm the action again.");
        };
        if p.expires_at <= OffsetDateTime::now_utc() {
            return Err("This confirmation has expired. Confirm the action again.");
        }
        if p.environment_id != environment_id || &p.command != command {
            return Err(
                "The confirmation was given for a different target. Confirm the action again.",
            );
        }
        if proof.typed.trim() != p.phrase {
            return Err("The typed confirmation doesn't match.");
        }
        Ok(())
    }
}
