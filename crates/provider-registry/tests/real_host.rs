//! Opt-in: runs every provider check against this machine's real Vagrant and
//! tools and prints the report. Needs Vagrant installed.
//! `cargo test -p vagrant-studio-provider-registry --test real_host -- --ignored --nocapture`

use vagrant_studio_platform::{PlatformAdapter, SystemPlatform};
use vagrant_studio_provider_registry::{Context, Registry, SystemProbe, bridged_interfaces};
use vagrant_studio_vagrant_adapter::ProcessRunner;

#[tokio::test]
#[ignore = "needs a real Vagrant installation"]
async fn reports_this_host() {
    let report = Registry::default()
        .check_host(&ProcessRunner, &SystemPlatform, &SystemProbe, None)
        .await;
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    assert!(report.vagrant.version.is_some(), "{:?}", report.vagrant);
    assert_eq!(report.providers.len(), 3);
}

#[tokio::test]
#[ignore = "needs a real host; prints VirtualBox's bridgeable adapters"]
async fn lists_bridged_interfaces() {
    let ctx = Context {
        runner: &ProcessRunner,
        platform: &SystemPlatform,
        probe: &SystemProbe,
        os: SystemPlatform.host_info().os,
        vagrant_version: None,
        plugins: &[],
    };
    println!("{:?}", bridged_interfaces(&ctx).await);
}
