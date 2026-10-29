//! Model → Vagrantfile text, with the managed-file header and checksum.

use crate::model::*;
use crate::validate::{FieldError, validate};
use crate::{MARKER_PREFIX, analyze, checksum};

const HEADER: &str = "\
# -*- mode: ruby -*-
# vi: set ft=ruby :
#
# Managed by the Vagrant Studio visual editor. You can also edit it by hand;
# the editor re-reads the file and switches to raw editing if it can't show it.
";

/// Heredoc terminators, tried in order until one doesn't occur in the script.
const TERMINATORS: &[&str] = &["SHELL", "SCRIPT", "EOS", "END_OF_SCRIPT"];
const RUBY_KEYWORDS: &[&str] = &[
    "alias", "and", "begin", "break", "case", "class", "def", "defined", "do", "else", "elsif",
    "end", "ensure", "false", "for", "if", "in", "module", "next", "nil", "not", "or", "redo",
    "rescue", "retry", "return", "self", "super", "then", "true", "undef", "unless", "until",
    "when", "while", "yield",
];

/// Generates a Vagrantfile, refusing models that fail validation. The output
/// is re-read with the recognizer and must give back the same model, so
/// nothing is ever written that the editor couldn't open again unchanged.
pub fn generate(
    config: &VagrantConfig,
    line_ending: LineEnding,
) -> Result<String, Vec<FieldError>> {
    let errors = validate(config);
    if !errors.is_empty() {
        return Err(errors);
    }
    let body = body(config);
    let text = format!("{HEADER}{MARKER_PREFIX}{}\n{body}", checksum(&body));
    let check = analyze(&text);
    if check.model.as_ref() != Some(config) || check.modified_outside {
        return Err(vec![FieldError::new(
            "",
            format!(
                "Internal error: the generated file doesn't read back the same ({}). Please report this.",
                check
                    .unsupported
                    .map(|u| format!("line {}: {}", u.line, u.message))
                    .unwrap_or_else(|| "model differs".into())
            ),
        )]);
    }
    Ok(match line_ending {
        LineEnding::Lf => text,
        LineEnding::Crlf => text.replace('\n', "\r\n"),
    })
}

#[derive(Default)]
struct Writer {
    out: String,
    /// Insert a blank line before the next line written in this block.
    gap: bool,
    /// Something was written since the block started.
    any: bool,
}

impl Writer {
    fn line(&mut self, depth: usize, text: &str) {
        if self.gap && self.any {
            self.out.push('\n');
        }
        self.gap = false;
        self.any = true;
        if !text.is_empty() {
            self.out.push_str(&"  ".repeat(depth));
            self.out.push_str(text);
        }
        self.out.push('\n');
    }

    /// Starts a new group of lines (separated by a blank line).
    fn group(&mut self) {
        self.gap = true;
    }

    fn open_block(&mut self) {
        self.any = false;
        self.gap = false;
    }

    /// Lines written as-is (heredoc bodies).
    fn raw(&mut self, text: &str) {
        self.out.push_str(text);
        self.out.push('\n');
    }
}

fn body(config: &VagrantConfig) -> String {
    let mut w = Writer::default();
    w.raw("");
    w.line(0, "Vagrant.configure(\"2\") do |config|");
    w.open_block();
    settings(&mut w, 1, "config", &config.defaults);
    for m in &config.machines {
        w.group();
        let var = block_var(&m.name);
        let mut head = format!("config.vm.define {}", ruby_string(&m.name));
        if m.primary {
            head.push_str(", primary: true");
        }
        if !m.autostart {
            head.push_str(", autostart: false");
        }
        w.line(1, &format!("{head} do |{var}|"));
        w.open_block();
        settings(&mut w, 2, &var, &m.settings);
        w.gap = false;
        w.line(1, "end");
    }
    w.gap = false;
    w.line(0, "end");
    w.out
}

fn settings(w: &mut Writer, d: usize, v: &str, s: &MachineSettings) {
    if let Some(b) = &s.box_name {
        w.line(d, &format!("{v}.vm.box = {}", ruby_string(b)));
    }
    if let Some(h) = &s.hostname {
        w.line(d, &format!("{v}.vm.hostname = {}", ruby_string(h)));
    }
    for n in &s.private_networks {
        let opt = match &n.ip {
            Some(ip) => format!("ip: {}", ruby_string(ip)),
            None => "type: \"dhcp\"".to_owned(),
        };
        w.line(d, &format!("{v}.vm.network \"private_network\", {opt}"));
    }
    for n in &s.public_networks {
        let mut line = format!("{v}.vm.network \"public_network\"");
        if let Some(ip) = &n.ip {
            line.push_str(&format!(", ip: {}", ruby_string(ip)));
        }
        if let Some(bridge) = &n.bridge {
            line.push_str(&format!(", bridge: {}", ruby_string(bridge)));
        }
        w.line(d, &line);
    }
    for p in &s.forwarded_ports {
        let mut line = format!(
            "{v}.vm.network \"forwarded_port\", guest: {}, host: {}",
            p.guest, p.host
        );
        if let Some(ip) = &p.host_ip {
            line.push_str(&format!(", host_ip: {}", ruby_string(ip)));
        }
        if let Some(proto) = p.protocol {
            line.push_str(&format!(", protocol: \"{}\"", proto.as_str()));
        }
        w.line(d, &line);
    }
    for f in &s.synced_folders {
        let mut line = format!(
            "{v}.vm.synced_folder {}, {}",
            ruby_string(&f.host_path),
            ruby_string(&f.guest_path)
        );
        if f.disabled {
            line.push_str(", disabled: true");
        }
        w.line(d, &line);
    }
    for p in &s.providers {
        w.group();
        let pv = match p.provider {
            ProviderKind::Virtualbox => "vb",
            ProviderKind::VmwareDesktop => "v",
        };
        w.line(
            d,
            &format!("{v}.vm.provider \"{}\" do |{pv}|", p.provider.as_str()),
        );
        match p.provider {
            ProviderKind::Virtualbox => {
                if let Some(c) = p.cpus {
                    w.line(d + 1, &format!("vb.cpus = {c}"));
                }
                if let Some(m) = p.memory_mb {
                    w.line(d + 1, &format!("vb.memory = {m}"));
                }
            }
            ProviderKind::VmwareDesktop => {
                if let Some(c) = p.cpus {
                    w.line(d + 1, &format!("v.vmx[\"numvcpus\"] = \"{c}\""));
                }
                if let Some(m) = p.memory_mb {
                    w.line(d + 1, &format!("v.vmx[\"memsize\"] = \"{m}\""));
                }
            }
        }
        w.line(d, "end");
    }
    for p in &s.provisioners {
        w.group();
        let head = match &p.name {
            Some(name) => format!("{v}.vm.provision {}, type: \"shell\"", ruby_string(name)),
            None => format!("{v}.vm.provision \"shell\""),
        };
        let mut opts = String::new();
        if let Some(run) = p.run {
            opts.push_str(&format!(", run: \"{}\"", run.as_str()));
        }
        if let Some(privileged) = p.privileged {
            opts.push_str(&format!(", privileged: {privileged}"));
        }
        match &p.source {
            ShellSource::Path { path } => {
                w.line(d, &format!("{head}, path: {}{opts}", ruby_string(path)))
            }
            ShellSource::Inline { script } => match heredoc(script, &"  ".repeat(d)) {
                Some((start, lines)) => {
                    w.line(d, &format!("{head}, inline: {start}{opts}"));
                    for l in lines {
                        w.raw(&l);
                    }
                }
                None => w.line(d, &format!("{head}, inline: {}{opts}", ruby_string(script))),
            },
        }
    }
}

/// A heredoc whose Ruby value is exactly `script`: the opening token and the
/// body lines including the terminator. `None` when a quoted string fits better.
fn heredoc(script: &str, indent: &str) -> Option<(String, Vec<String>)> {
    let content = script.strip_suffix('\n')?;
    let lines: Vec<&str> = content.split('\n').collect();
    let term = TERMINATORS
        .iter()
        .find(|t| !lines.iter().any(|l| l.trim() == **t))?;
    // Without escapes or interpolation the unquoted form means the same as
    // the quoted one and reads like hand-written Vagrantfiles.
    let literal = !script.contains('\\')
        && !script.contains("#{")
        && !script.contains("#@")
        && !script.contains("#$");
    let quoted = |t: &str| {
        if literal {
            t.to_owned()
        } else {
            format!("'{t}'")
        }
    };
    let leading = |l: &str| l.len() - l.trim_start().len();
    // `<<~` strips the smallest indentation, so it is used only when some
    // line is flush left and no line needs whitespace-only or tab handling.
    let squiggly = lines.iter().any(|l| !l.is_empty() && leading(l) == 0)
        && !lines
            .iter()
            .any(|l| (!l.is_empty() && l.trim().is_empty()) || l.starts_with('\t'));
    let mut body: Vec<String> = if squiggly {
        lines
            .iter()
            .map(|l| {
                if l.is_empty() {
                    String::new()
                } else {
                    format!("{indent}  {l}")
                }
            })
            .collect()
    } else {
        lines.iter().map(|l| (*l).to_owned()).collect()
    };
    body.push(format!("{indent}{term}"));
    let start = format!("<<{}{}", if squiggly { "~" } else { "-" }, quoted(term));
    Some((start, body))
}

/// Double-quoted Ruby string literal.
pub(crate) fn ruby_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '#' if matches!(chars.peek(), Some('{' | '@' | '$')) => out.push_str("\\#"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A Ruby block variable for a machine: its name when that is a valid,
/// non-reserved local variable name, otherwise a sanitized form.
fn block_var(name: &str) -> String {
    let mut var: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if !var.starts_with(|c: char| c.is_ascii_lowercase() || c == '_') {
        var.insert_str(0, "m_");
    }
    if RUBY_KEYWORDS.contains(&var.as_str()) || matches!(var.as_str(), "config" | "vb" | "v") {
        var.push_str("_vm");
    }
    var
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_variables_are_valid_ruby_locals() {
        assert_eq!(block_var("web"), "web");
        assert_eq!(block_var("Web-1.local"), "web_1_local");
        assert_eq!(block_var("1st"), "m_1st");
        assert_eq!(block_var("end"), "end_vm");
        assert_eq!(block_var("config"), "config_vm");
        assert_eq!(block_var("vb"), "vb_vm");
    }

    #[test]
    fn ruby_strings_escape_interpolation() {
        assert_eq!(ruby_string("a\"b\\#{c}#d"), r#""a\"b\\\#{c}#d""#);
        assert_eq!(ruby_string("x\ny"), r#""x\ny""#);
    }

    #[test]
    fn heredoc_forms() {
        let (start, body) = heredoc("apt-get update\n  echo hi\n\n", "  ").unwrap();
        assert_eq!(start, "<<~SHELL");
        assert_eq!(
            body,
            vec!["    apt-get update", "      echo hi", "", "  SHELL"]
        );
        // Indented content keeps its indentation with <<-.
        let (start, body) = heredoc("  a\n", "  ").unwrap();
        assert_eq!(start, "<<-SHELL");
        assert_eq!(body, vec!["  a", "  SHELL"]);
        // Backslashes need the quoted form.
        let (start, body) = heredoc("a \\\n  b\nSHELL\n", "").unwrap();
        assert_eq!(start, "<<~'SCRIPT'");
        assert_eq!(body, vec!["  a \\", "    b", "  SHELL", "SCRIPT"]);
        let (start, _) = heredoc("  a \\\n", "").unwrap();
        assert_eq!(start, "<<-'SHELL'");
        assert!(heredoc("no newline", "").is_none());
    }
}
