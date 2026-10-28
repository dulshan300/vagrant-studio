use serde::Serialize;

/// A user-facing validation failure for one input field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{field}: {message}")]
pub struct ValidationError {
    pub field: &'static str,
    pub message: String,
}

impl ValidationError {
    pub fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

const MAX_NAME: usize = 100;
const MAX_DESCRIPTION: usize = 2000;
const MAX_ENV_NAME: usize = 50;
const MAX_URL: usize = 500;
const MAX_TAG: usize = 32;
const MAX_TAGS: usize = 20;
const MAX_SLUG: usize = 64;

pub fn validate_name(name: &str) -> Result<String, ValidationError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ValidationError::new("name", "Name is required."));
    }
    if name.chars().count() > MAX_NAME {
        return Err(ValidationError::new(
            "name",
            format!("Name must be at most {MAX_NAME} characters."),
        ));
    }
    if name.chars().any(char::is_control) {
        return Err(ValidationError::new(
            "name",
            "Name cannot contain control characters.",
        ));
    }
    Ok(name.to_owned())
}

pub fn validate_description(description: &str) -> Result<String, ValidationError> {
    let description = description.trim();
    if description.chars().count() > MAX_DESCRIPTION {
        return Err(ValidationError::new(
            "description",
            format!("Description must be at most {MAX_DESCRIPTION} characters."),
        ));
    }
    Ok(description.to_owned())
}

/// Environment names are short identifiers shown in tabs and logs.
pub fn validate_environment_name(name: &str) -> Result<String, ValidationError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_ENV_NAME {
        return Err(ValidationError::new(
            "environmentName",
            format!("Environment name must be 1–{MAX_ENV_NAME} characters."),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.'))
    {
        return Err(ValidationError::new(
            "environmentName",
            "Environment name may only contain letters, digits, spaces, '-', '_' and '.'.",
        ));
    }
    Ok(name.to_owned())
}

/// Accepts http(s), ssh, git and scp-style (`git@host:owner/repo.git`) URLs.
/// Embedded credentials are rejected so secrets never reach stored or
/// portable metadata.
pub fn validate_repository_url(url: Option<&str>) -> Result<Option<String>, ValidationError> {
    let Some(url) = url.map(str::trim).filter(|u| !u.is_empty()) else {
        return Ok(None);
    };
    let err = |m: &str| Err(ValidationError::new("repositoryUrl", m));
    if url.len() > MAX_URL {
        return err("Repository URL is too long.");
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return err("Repository URL cannot contain spaces.");
    }

    if let Some((scheme, rest)) = url.split_once("://") {
        if !matches!(
            scheme.to_ascii_lowercase().as_str(),
            "https" | "http" | "ssh" | "git"
        ) {
            return err("Use an https, http, ssh or git URL.");
        }
        let authority = rest.split('/').next().unwrap_or_default();
        if authority.is_empty() {
            return err("Repository URL needs a host.");
        }
        if let Some((userinfo, _)) = authority.rsplit_once('@') {
            if userinfo.contains(':') {
                return err("Remove the password or token from the URL.");
            }
            if matches!(scheme, "https" | "http") {
                return err("Remove the user name or token from the URL.");
            }
        }
        return Ok(Some(url.to_owned()));
    }

    // scp-like syntax: [user@]host:path
    match url.split_once(':') {
        Some((host, path)) if !host.is_empty() && !path.is_empty() && !host.contains('/') => {
            Ok(Some(url.to_owned()))
        }
        _ => err("Enter a repository URL such as https://host/owner/repo.git."),
    }
}

/// Vagrant provider names such as `virtualbox`, `vmware_desktop`, `libvirt`.
pub fn validate_provider_name(provider: Option<&str>) -> Result<Option<String>, ValidationError> {
    let Some(p) = provider.map(str::trim).filter(|p| !p.is_empty()) else {
        return Ok(None);
    };
    if p.len() > 40
        || !p.starts_with(|c: char| c.is_ascii_lowercase())
        || !p
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return Err(ValidationError::new(
            "provider",
            format!("\"{p}\" is not a valid Vagrant provider name."),
        ));
    }
    Ok(Some(p.to_owned()))
}

/// Tags are case-insensitive; they are stored lowercased.
pub fn validate_tag(tag: &str) -> Result<String, ValidationError> {
    let tag = tag.trim().to_lowercase();
    if tag.is_empty() || tag.chars().count() > MAX_TAG {
        return Err(ValidationError::new(
            "tags",
            format!("Tags must be 1–{MAX_TAG} characters."),
        ));
    }
    if !tag
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(ValidationError::new(
            "tags",
            format!("Tag \"{tag}\" may only contain letters, digits, '-', '_' and '.'."),
        ));
    }
    Ok(tag)
}

/// Validates, lowercases, de-duplicates and sorts tags.
pub fn normalize_tags<S: AsRef<str>>(tags: &[S]) -> Result<Vec<String>, ValidationError> {
    let mut out = tags
        .iter()
        .map(|t| validate_tag(t.as_ref()))
        .collect::<Result<Vec<_>, _>>()?;
    out.sort();
    out.dedup();
    if out.len() > MAX_TAGS {
        return Err(ValidationError::new(
            "tags",
            format!("A project can have at most {MAX_TAGS} tags."),
        ));
    }
    Ok(out)
}

/// ASCII slug used for folder names and URLs: `My Web App!` → `my-web-app`.
pub fn slugify(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug: String = slug.trim_matches('-').chars().take(MAX_SLUG).collect();
    let slug = slug.trim_end_matches('-');
    if slug.is_empty() {
        "project".to_owned()
    } else {
        slug.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_and_bounded() {
        assert_eq!(validate_name("  Web Platform ").unwrap(), "Web Platform");
        assert!(validate_name("   ").is_err());
        assert!(validate_name(&"x".repeat(101)).is_err());
        assert!(validate_name("bad\u{7}name").is_err());
        assert_eq!(validate_name("Café ünïcode").unwrap(), "Café ünïcode");
    }

    #[test]
    fn environment_names() {
        assert_eq!(validate_environment_name(" dev ").unwrap(), "dev");
        assert!(validate_environment_name("a/b").is_err());
        assert!(validate_environment_name("").is_err());
    }

    #[test]
    fn repository_urls() {
        let ok = |u: &str| validate_repository_url(Some(u)).unwrap().unwrap();
        assert_eq!(
            ok("https://github.com/o/r.git"),
            "https://github.com/o/r.git"
        );
        assert_eq!(ok("git@github.com:o/r.git"), "git@github.com:o/r.git");
        assert_eq!(ok("ssh://git@host:22/o/r.git"), "ssh://git@host:22/o/r.git");
        assert_eq!(validate_repository_url(Some("  ")).unwrap(), None);
        assert_eq!(validate_repository_url(None).unwrap(), None);

        for bad in [
            "ftp://host/r",
            "https://user:token@github.com/o/r.git",
            "https://token@github.com/o/r.git",
            "https:///nohost",
            "not a url",
            "plainword",
        ] {
            assert!(validate_repository_url(Some(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn tags_are_normalized() {
        assert_eq!(
            normalize_tags(&["Web", "db", "web", " backend "]).unwrap(),
            vec!["backend", "db", "web"]
        );
        assert!(normalize_tags(&["has space"]).is_err());
        assert!(normalize_tags(&[""]).is_err());
        let many: Vec<String> = (0..21).map(|i| format!("t{i}")).collect();
        assert!(normalize_tags(&many).is_err());
    }

    #[test]
    fn provider_names() {
        assert_eq!(
            validate_provider_name(Some(" vmware_desktop "))
                .unwrap()
                .as_deref(),
            Some("vmware_desktop")
        );
        assert_eq!(validate_provider_name(Some("")).unwrap(), None);
        for bad in ["VirtualBox", "--provider", "a b", "1x", "x;y"] {
            assert!(validate_provider_name(Some(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn slugs() {
        assert_eq!(slugify("My Web App!"), "my-web-app");
        assert_eq!(slugify("  --a__b--  "), "a-b");
        assert_eq!(slugify("Café"), "caf");
        assert_eq!(slugify("日本語"), "project");
        assert_eq!(slugify(&"a".repeat(100)).len(), 64);
    }
}
