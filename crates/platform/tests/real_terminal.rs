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

#[test]
#[ignore = "opens a real editor window"]
fn opens_the_first_editor() {
    let editor = vagrant_studio_platform::system_editors().remove(0);
    let folder = std::env::var("VS_TEST_FOLDER").unwrap();
    println!("{} {}", editor.path.display(), folder);
    vagrant_studio_platform::spawn_editor(&editor.path, Path::new(&folder)).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(5));
}
