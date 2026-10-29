//! Opt-in: runs every provider check against this machine's real Vagrant and
//! tools and prints the report. Needs Vagrant installed.
//! `cargo test -p vagrant-studio-provider-registry --test real_host -- --ignored --nocapture`

use vagrant_studio_platform::SystemPlatform;
use vagrant_studio_provider_registry::{Registry, SystemProbe};
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
