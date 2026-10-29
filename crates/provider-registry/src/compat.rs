//! Which VirtualBox versions a Vagrant release can drive.
//!
//! Vagrant picks a VirtualBox driver by `major.minor` and refuses versions it
//! has no driver for. The table comes from each release's
//! `plugins/providers/virtualbox/driver/meta.rb` (`driver_map`); add a row
//! when checking a new Vagrant release, and never guess one.

type Release = (u32, u32, u32);
type MajorMinor = (u32, u32);

/// Vagrant release → oldest and newest VirtualBox `major.minor` it has a driver for.
const VIRTUALBOX_DRIVERS: &[(Release, MajorMinor, MajorMinor)] = &[((2, 4, 9), (4, 0), (7, 2))];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualBoxSupport {
    Supported,
    /// Vagrant has no driver for this VirtualBox; machines won't start.
    Unsupported {
        newest: MajorMinor,
    },
    /// Vagrant Studio has no record of this Vagrant release (or a version
    /// couldn't be parsed).
    Unknown,
}

pub fn virtualbox_support(vagrant: &str, virtualbox: &str) -> VirtualBoxSupport {
    let (Some(v), Some(vb)) = (parse::<3>(vagrant), parse::<2>(virtualbox)) else {
        return VirtualBoxSupport::Unknown;
    };
    let (v, vb) = ((v[0], v[1], v[2]), (vb[0], vb[1]));
    let row = VIRTUALBOX_DRIVERS
        .iter()
        .find(|(release, ..)| *release == v);
    if let Some(&(_, oldest, newest)) = row {
        return if (oldest..=newest).contains(&vb) {
            VirtualBoxSupport::Supported
        } else {
            VirtualBoxSupport::Unsupported { newest }
        };
    }
    // A newer Vagrant keeps the drivers of the newest release we know about,
    // but may or may not have added newer ones.
    let latest = VIRTUALBOX_DRIVERS
        .iter()
        .max_by_key(|(release, ..)| *release);
    match latest {
        Some(&(release, oldest, newest)) if v > release && (oldest..=newest).contains(&vb) => {
            VirtualBoxSupport::Supported
        }
        _ => VirtualBoxSupport::Unknown,
    }
}

/// Leading `N` numeric components of a version such as `2.4.9`, `7.1.18r173720`
/// or `7.2.0_BETA1`.
fn parse<const N: usize>(version: &str) -> Option<[u32; N]> {
    let mut parts = version.trim().split('.');
    let mut out = [0; N];
    for slot in &mut out {
        let part = parts.next()?;
        let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
        *slot = digits.parse().ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vagrant_release_uses_its_driver_range() {
        use VirtualBoxSupport::*;
        assert_eq!(virtualbox_support("2.4.9", "7.1.18"), Supported);
        assert_eq!(virtualbox_support("2.4.9", "7.2.0_BETA1"), Supported);
        assert_eq!(virtualbox_support("2.4.9", "4.0.2"), Supported);
        assert_eq!(
            virtualbox_support("2.4.9", "7.3.0"),
            Unsupported { newest: (7, 2) }
        );
        assert_eq!(
            virtualbox_support("2.4.9", "3.2.28"),
            Unsupported { newest: (7, 2) }
        );
    }

    #[test]
    fn other_vagrant_releases_are_not_guessed() {
        use VirtualBoxSupport::*;
        assert_eq!(virtualbox_support("2.4.10", "7.1.18"), Supported);
        assert_eq!(virtualbox_support("2.5.0", "7.3.0"), Unknown);
        assert_eq!(virtualbox_support("2.4.1", "7.1.18"), Unknown);
        assert_eq!(virtualbox_support("2.4.9", "garbage"), Unknown);
        assert_eq!(virtualbox_support("", "7.1"), Unknown);
    }
}
