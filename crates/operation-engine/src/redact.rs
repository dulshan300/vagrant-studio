//! Removes secrets from command output before it is logged or shown.
//! Best-effort pattern matching; it errs on the side of redacting.

use std::sync::LazyLock;

use regex::Regex;

pub const REDACTED: &str = "[REDACTED]";

static ASSIGNMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)\b([a-z0-9_.-]*(?:password|passwd|secret|token|api[_-]?key|access[_-]?key|private[_-]?key|client[_-]?secret)[a-z0-9_.-]*)(\s*[=:]\s*)("[^"]*"|'[^']*'|[^\s,;]+)"#,
    )
    .expect("valid regex")
});
static URL_CREDENTIALS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b([a-z][a-z0-9+.-]*://)([^/\s:@]+):([^/\s@]+)@").expect("valid regex")
});
static BEARER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(bearer\s+)[a-z0-9._~+/=-]+").expect("valid regex"));
static INLINE_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"-----BEGIN [A-Z ]*PRIVATE KEY-----.*?(-----END [A-Z ]*PRIVATE KEY-----|$)")
        .expect("valid regex")
});

/// Line-by-line redactor; keeps state to drop multi-line private key blocks.
#[derive(Debug, Default)]
pub struct Redactor {
    in_private_key: bool,
}

impl Redactor {
    /// Returns the redacted line, or `None` if the whole line must be dropped.
    pub fn line(&mut self, line: &str) -> Option<String> {
        if self.in_private_key {
            if line.contains("-----END") && line.contains("PRIVATE KEY-----") {
                self.in_private_key = false;
            }
            return None;
        }
        if line.contains("-----BEGIN")
            && line.contains("PRIVATE KEY-----")
            && !line.contains("-----END")
        {
            self.in_private_key = true;
            return Some(format!("{REDACTED} private key"));
        }
        let line = INLINE_KEY.replace_all(line, format!("{REDACTED} private key").as_str());
        let line = URL_CREDENTIALS.replace_all(&line, format!("$1$2:{REDACTED}@").as_str());
        let line = BEARER.replace_all(&line, format!("${{1}}{REDACTED}").as_str());
        let line = ASSIGNMENT.replace_all(&line, format!("$1$2{REDACTED}").as_str());
        Some(line.into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(line: &str) -> Option<String> {
        Redactor::default().line(line)
    }

    #[test]
    fn redacts_assignments() {
        assert_eq!(
            r("DB_PASSWORD=hunter2 ok").unwrap(),
            "DB_PASSWORD=[REDACTED] ok"
        );
        assert_eq!(r("api_key: \"abc def\"").unwrap(), "api_key: [REDACTED]");
        assert_eq!(
            r("GITHUB_TOKEN = ghp_123,next").unwrap(),
            "GITHUB_TOKEN = [REDACTED],next"
        );
        assert_eq!(r("client_secret='x'").unwrap(), "client_secret=[REDACTED]");
    }

    #[test]
    fn redacts_url_credentials_and_bearer_tokens() {
        assert_eq!(
            r("cloning https://user:s3cret@github.com/o/r.git").unwrap(),
            "cloning https://user:[REDACTED]@github.com/o/r.git"
        );
        assert_eq!(
            r("Authorization: Bearer eyJhbGciOi.abc-def").unwrap(),
            "Authorization: Bearer [REDACTED]"
        );
    }

    #[test]
    fn drops_multi_line_private_keys() {
        let mut red = Redactor::default();
        let out: Vec<_> = [
            "before",
            "-----BEGIN OPENSSH PRIVATE KEY-----",
            "b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW",
            "-----END OPENSSH PRIVATE KEY-----",
            "after",
        ]
        .into_iter()
        .filter_map(|l| red.line(l))
        .collect();
        assert_eq!(out, ["before", "[REDACTED] private key", "after"]);
    }

    #[test]
    fn redacts_inline_private_keys_in_machine_readable_lines() {
        let line = r"1,web,ui,info,-----BEGIN RSA PRIVATE KEY-----\nMIIabc\n-----END RSA PRIVATE KEY----- tail";
        assert_eq!(
            r(line).unwrap(),
            "1,web,ui,info,[REDACTED] private key tail"
        );
    }

    #[test]
    fn leaves_normal_output_alone() {
        for line in [
            "==> web: Booting VM...",
            "1793205206,web,state,not_created",
            "    web: SSH auth method: private key",
            "Bringing machine 'web' up with 'vmware_desktop' provider...",
        ] {
            assert_eq!(r(line).unwrap(), line);
        }
    }
}
