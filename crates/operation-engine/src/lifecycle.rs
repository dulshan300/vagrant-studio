//! Targeting rules for machine lifecycle commands.
//!
//! Machines are always named explicitly: without names, `vagrant up` skips
//! machines marked `autostart: false`, so "all machines" means every machine
//! Vagrant last reported. Names are passed in Vagrantfile order (the order
//! `vagrant status` lists them); Vagrant itself reverses it for `destroy`.

use vagrant_studio_domain::OperationType;
use vagrant_studio_vagrant_adapter::{LifecycleAction, MachineName, Provisioning, StateClass};

use crate::{EngineError, EnvironmentStatus, MachineStatusDto};

pub fn operation_type(action: LifecycleAction) -> OperationType {
    match action {
        LifecycleAction::Up => OperationType::Up,
        LifecycleAction::Halt => OperationType::Halt,
        LifecycleAction::Reload => OperationType::Reload,
        LifecycleAction::Suspend => OperationType::Suspend,
        LifecycleAction::Resume => OperationType::Resume,
        LifecycleAction::Provision => OperationType::Provision,
        LifecycleAction::Destroy => OperationType::Destroy,
    }
}

/// Checks the requested machines against the last status from Vagrant and
/// returns them in Vagrantfile order.
pub fn order_targets(
    status: Option<&EnvironmentStatus>,
    requested: &[String],
) -> Result<Vec<MachineName>, EngineError> {
    let Some(status) = status else {
        return Err(EngineError::Invalid(
            "Machine state hasn't been checked yet. Refresh status first.".into(),
        ));
    };
    if status.error.is_some() {
        return Err(EngineError::Invalid(
            "The last status check failed. Fix the problem and refresh status first.".into(),
        ));
    }
    if requested.is_empty() {
        return Err(EngineError::Invalid("Choose at least one machine.".into()));
    }
    if let Some(unknown) = requested
        .iter()
        .find(|r| !status.machines.iter().any(|m| &m.name == *r))
    {
        return Err(EngineError::Invalid(format!(
            "Vagrant didn't report a machine named “{unknown}”. Refresh status and try again."
        )));
    }
    status
        .machines
        .iter()
        .filter(|m| requested.contains(&m.name))
        .map(|m| MachineName::try_from(m.name.clone()).map_err(EngineError::Invalid))
        .collect()
}

/// Refuses provisioner choices the action can't use and gives requests that
/// mean the same one form, so duplicate requests are recognized.
pub fn check_provisioning(
    action: LifecycleAction,
    provisioning: Provisioning,
) -> Result<Provisioning, EngineError> {
    match provisioning {
        Provisioning::Default => Ok(Provisioning::Default),
        _ if !action.runs_provisioners() => Err(EngineError::Invalid(format!(
            "`vagrant {}` doesn't run provisioners.",
            action.verb()
        ))),
        Provisioning::All if action == LifecycleAction::Provision => Ok(Provisioning::Default),
        Provisioning::Only { names } if names.is_empty() => Err(EngineError::Invalid(
            "Choose at least one provisioner.".into(),
        )),
        Provisioning::Only { names }
            if names
                .iter()
                .enumerate()
                .any(|(i, n)| names[..i].contains(n)) =>
        {
            Err(EngineError::Invalid(
                "A provisioner is chosen more than once.".into(),
            ))
        }
        other => Ok(other),
    }
}

/// What the user types to confirm: the machine name for one machine, the
/// environment name for several.
pub fn confirmation_phrase(environment_name: &str, targets: &[MachineName]) -> String {
    match targets {
        [one] => one.as_str().to_owned(),
        _ => environment_name.to_owned(),
    }
}

/// Describes targeted machines whose observed state contradicts what the
/// command should have produced. Unknown states are not judged.
pub(crate) fn state_mismatch(
    action: LifecycleAction,
    targets: &[MachineName],
    machines: &[MachineStatusDto],
) -> Option<String> {
    let expected = action.expected_state();
    let off: Vec<String> = machines
        .iter()
        .filter(|m| targets.is_empty() || targets.iter().any(|t| t.as_str() == m.name))
        .filter_map(|m| {
            let state = m.state.as_deref()?;
            let class = StateClass::of(state);
            (class != StateClass::Unknown && class != expected).then(|| {
                format!(
                    "{} is {}",
                    m.name,
                    m.state_short.as_deref().unwrap_or(state)
                )
            })
        })
        .collect();
    (!off.is_empty()).then(|| {
        format!(
            "Vagrant reports {} afterwards (expected {}).",
            off.join(", "),
            describe(expected)
        )
    })
}

fn describe(class: StateClass) -> &'static str {
    match class {
        StateClass::Running => "running",
        StateClass::Stopped => "stopped",
        StateClass::Suspended => "suspended",
        StateClass::NotCreated => "not created",
        StateClass::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vagrant_studio_vagrant_adapter::ProvisionerName;

    fn machine(name: &str, state: &str) -> MachineStatusDto {
        MachineStatusDto {
            name: name.into(),
            provider: Some("virtualbox".into()),
            state: Some(state.into()),
            state_short: Some(state.replace('_', " ")),
            state_long: None,
        }
    }

    fn status(machines: Vec<MachineStatusDto>) -> EnvironmentStatus {
        EnvironmentStatus {
            environment_id: vagrant_studio_domain::EnvironmentId::new(),
            operation_id: None,
            observed_at: time::OffsetDateTime::now_utc(),
            machines,
            error: None,
        }
    }

    fn names(targets: &[MachineName]) -> Vec<&str> {
        targets.iter().map(MachineName::as_str).collect()
    }

    #[test]
    fn targets_follow_vagrantfile_order() {
        let s = status(vec![
            machine("db", "poweroff"),
            machine("web", "poweroff"),
            machine("worker", "not_created"),
        ]);
        let t = order_targets(Some(&s), &["worker".into(), "db".into()]).unwrap();
        assert_eq!(names(&t), ["db", "worker"]);
    }

    #[test]
    fn refuses_unknown_missing_or_failed_status() {
        let s = status(vec![machine("web", "running")]);
        for (status, requested) in [
            (None, vec!["web".to_owned()]),
            (Some(&s), vec![]),
            (Some(&s), vec!["db".to_owned()]),
            (Some(&s), vec!["-f".to_owned()]),
        ] {
            assert!(
                matches!(
                    order_targets(status, &requested),
                    Err(EngineError::Invalid(_))
                ),
                "{requested:?}"
            );
        }
        let mut failed = s.clone();
        failed.error = Some(crate::StatusError {
            class: None,
            message: "boom".into(),
        });
        assert!(order_targets(Some(&failed), &["web".into()]).is_err());
    }

    #[test]
    fn provisioning_is_checked_and_normalized() {
        use LifecycleAction::*;
        let only = |names: &[&str]| Provisioning::Only {
            names: names
                .iter()
                .map(|n| ProvisionerName::try_from((*n).to_owned()).unwrap())
                .collect(),
        };
        assert_eq!(
            check_provisioning(Provision, Provisioning::All).unwrap(),
            Provisioning::Default
        );
        assert_eq!(
            check_provisioning(Reload, Provisioning::All).unwrap(),
            Provisioning::All
        );
        assert_eq!(check_provisioning(Up, only(&["a"])).unwrap(), only(&["a"]));
        assert_eq!(
            check_provisioning(Halt, Provisioning::Default).unwrap(),
            Provisioning::Default
        );
        for (action, p) in [
            (Halt, Provisioning::All),
            (Destroy, only(&["a"])),
            (Provision, only(&[])),
            (Provision, only(&["a", "b", "a"])),
        ] {
            assert!(
                matches!(check_provisioning(action, p), Err(EngineError::Invalid(_))),
                "{action:?}"
            );
        }
    }

    #[test]
    fn phrase_is_machine_or_environment_name() {
        let one = vec![MachineName::try_from("web".to_owned()).unwrap()];
        assert_eq!(confirmation_phrase("Default env", &one), "web");
        let two = vec![
            MachineName::try_from("web".to_owned()).unwrap(),
            MachineName::try_from("db".to_owned()).unwrap(),
        ];
        assert_eq!(confirmation_phrase("Default env", &two), "Default env");
    }

    #[test]
    fn reports_only_contradicting_known_states_of_targets() {
        let machines = vec![
            machine("web", "running"),
            machine("db", "poweroff"),
            machine("cache", "host_state_unknown"),
            machine("other", "not_created"),
        ];
        let targets: Vec<MachineName> = ["web", "db", "cache"]
            .iter()
            .map(|n| MachineName::try_from((*n).to_owned()).unwrap())
            .collect();
        assert_eq!(
            state_mismatch(LifecycleAction::Up, &targets, &machines).as_deref(),
            Some("Vagrant reports db is poweroff afterwards (expected running).")
        );
        assert_eq!(
            state_mismatch(LifecycleAction::Up, &targets[..1], &machines),
            None
        );
        assert!(
            state_mismatch(LifecycleAction::Destroy, &[], &machines)
                .unwrap()
                .contains("web is running, db is poweroff")
        );
    }
}
