//! Prints which terminal and editors this host would open. Run with
//! `cargo test -p vagrant-studio-platform --test real_terminal -- --ignored --nocapture`.

use std::path::Path;

use vagrant_studio_platform::system_terminal_launch;

#[test]
#[ignore = "inspects the real host"]
fn prints_the_terminal_for_this_host() {
    let launch = system_terminal_launch(
        "vagrant ssh",
        Path::new("vagrant"),
        &["ssh".into(), "default".into()],
    );
    println!("{launch:#?}");
}

#[test]
#[ignore = "inspects the real host"]
fn prints_the_editors_on_this_host() {
    println!("{:#?}", vagrant_studio_platform::system_editors());
}
