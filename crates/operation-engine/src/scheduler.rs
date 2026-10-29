//! Locks that decide when a queued operation may start (architecture.md §4).
//!
//! Each environment has a read/write lock: mutating commands take it for
//! writing, so at most one runs per environment, and read-only commands
//! (`status`, `validate`) share it. Commands that use the host's box store
//! also take the global box lock; box changes (`box add/update/remove`)
//! take it for writing, so they wait for running `up`s and block new ones. Tokio's locks are fair, so waiting operations start in request
//! order. The environment lock is always taken before the box lock.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};
use vagrant_studio_domain::EnvironmentId;
use vagrant_studio_vagrant_adapter::VagrantCommand;

#[derive(Default)]
pub(crate) struct Locks {
    environments: Mutex<HashMap<EnvironmentId, Arc<RwLock<()>>>>,
    boxes: Arc<RwLock<()>>,
}

/// Held for the whole operation, including the follow-up status refresh.
#[allow(dead_code)] // the guards are held, not read
pub(crate) enum Held {
    Write(OwnedRwLockWriteGuard<()>),
    Read(OwnedRwLockReadGuard<()>),
}

impl Locks {
    /// The environment's write lock, if nothing holds or waits for it.
    pub(crate) fn try_write(
        &self,
        environment_id: EnvironmentId,
    ) -> Option<OwnedRwLockWriteGuard<()>> {
        let lock = self
            .environments
            .lock()
            .expect("lock")
            .entry(environment_id)
            .or_default()
            .clone();
        lock.try_write_owned().ok()
    }

    pub(crate) async fn acquire(
        &self,
        environment_id: Option<EnvironmentId>,
        command: &VagrantCommand,
    ) -> Vec<Held> {
        let mut held = Vec::new();
        if let Some(id) = environment_id {
            let lock = self
                .environments
                .lock()
                .expect("lock")
                .entry(id)
                .or_default()
                .clone();
            held.push(if command.is_mutating() {
                Held::Write(lock.write_owned().await)
            } else {
                Held::Read(lock.read_owned().await)
            });
        }
        if command.changes_boxes() {
            held.push(Held::Write(self.boxes.clone().write_owned().await));
        } else if command.uses_boxes() {
            held.push(Held::Read(self.boxes.clone().read_owned().await));
        }
        held
    }
}
