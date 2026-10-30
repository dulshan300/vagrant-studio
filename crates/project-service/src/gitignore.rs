//! The `.gitignore` written when Vagrant Studio creates a Git repository.

use std::path::Path;

use vagrant_studio_domain::ProjectId;

use crate::{ProjectService, Result, write_new_file};

/// Ignores what Vagrant creates per computer. `.vagrant/` also holds each
/// machine's private SSH key, so it must never be committed.
pub const GITIGNORE: &str = "\
# Created by Vagrant Studio. Edit freely; it is never rewritten.

# Vagrant's machine state: machine IDs and private SSH keys for this computer.
.vagrant/

# Logs, such as the serial console log some boxes write next to the Vagrantfile.
*.log

# Boxes made with `vagrant package`.
*.box
";

impl ProjectService {
    /// Writes [`GITIGNORE`] into the project folder unless a `.gitignore`
    /// already exists, which is left as it is. Returns whether it wrote one.
    pub async fn add_gitignore(&self, project_id: ProjectId) -> Result<bool> {
        let root = self.require(project_id).await?.root_path;
        let path = Path::new(&root).join(".gitignore");
        if path.exists() {
            return Ok(false);
        }
        match write_new_file(&path, GITIGNORE) {
            Ok(()) => Ok(true),
            // Created by someone else in the meantime: still left alone.
            Err(_) if path.exists() => Ok(false),
            Err(e) => Err(e),
        }
    }
}
