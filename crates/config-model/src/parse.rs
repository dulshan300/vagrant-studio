//! Conservative recognizer: accepts exactly the statements the model can
//! represent and stops at the first one it can't, with its line number.

use crate::Unsupported;
use crate::lexer::{Tok, Token};
use crate::model::*;

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Str(String),
    Int(u64),
    Bool(bool),
    Sym(String),
}

struct Arg {
    key: String,
    value: Value,
    line: usize,
}

pub(crate) fn parse(tokens: &[Token]) -> Result<VagrantConfig, Unsupported> {
    let mut p = Parser { tokens, pos: 0 };
    p.skip_newlines();
    if p.peek().is_none() {
        return Err(Unsupported::new(1, "the file is empty"));
    }
    if p.peek() != Some(&Tok::Const("Vagrant".into())) {
        let found = p.peek().map(describe).unwrap_or_default();
        return Err(p.unsupported(format!(
            "`{found}` isn't supported by the visual editor; the file must start with `Vagrant.configure(\"2\") do |config|`"
        )));
    }
    p.pos += 1;
    p.expect_punct('.')?;
    p.expect_ident("configure")?;
    p.expect_punct('(')?;
    match p.next_tok() {
        Some(Tok::Str(v)) if v == "2" => {}
        Some(Tok::Int(2)) => {}
        _ => return Err(p.unsupported_prev("only `Vagrant.configure(\"2\")` is supported")),
    }
    p.expect_punct(')')?;
    let var = p.block_header()?;
    let mut config = VagrantConfig::default();
    p.body(&var, &mut config.defaults, Some(&mut config.machines))?;
    p.expect_ident("end")?;
    p.skip_newlines();
    if p.peek().is_some() {
        return Err(p.unsupported("only comments may follow the `Vagrant.configure` block"));
    }
    Ok(config)
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.pos).map(|t| &t.tok)
    }

    fn line(&self) -> usize {
        self.tokens
            .get(self.pos)
            .or_else(|| self.tokens.last())
            .map_or(1, |t| t.line)
    }

    fn next_tok(&mut self) -> Option<Tok> {
        let t = self.tokens.get(self.pos).map(|t| t.tok.clone());
        self.pos += 1;
        t
    }

    fn unsupported(&self, msg: impl Into<String>) -> Unsupported {
        Unsupported::new(self.line(), msg)
    }

    /// Error located at the token just consumed.
    fn unsupported_prev(&self, msg: impl Into<String>) -> Unsupported {
        let line = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map_or(1, |t| t.line);
        Unsupported::new(line, msg)
    }

    fn skip_newlines(&mut self) {
        while self.peek() == Some(&Tok::Newline) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, tok: &Tok, msg: &str) -> Result<(), Unsupported> {
        if self.peek() == Some(tok) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.unsupported(msg))
        }
    }

    fn expect_punct(&mut self, c: char) -> Result<(), Unsupported> {
        self.expect(&Tok::Punct(c), &format!("expected `{c}` here"))
    }

    fn expect_ident(&mut self, name: &str) -> Result<(), Unsupported> {
        self.expect(&Tok::Ident(name.into()), &format!("expected `{name}` here"))
    }

    fn ident(&mut self) -> Result<String, Unsupported> {
        match self.next_tok() {
            Some(Tok::Ident(name)) => Ok(name),
            _ => Err(self.unsupported_prev("expected a name here")),
        }
    }

    fn string(&mut self, what: &str) -> Result<String, Unsupported> {
        match self.next_tok() {
            Some(Tok::Str(s)) => Ok(s),
            _ => Err(self.unsupported_prev(format!("{what} must be a plain string"))),
        }
    }

    /// `do |name|` followed by the end of the line.
    fn block_header(&mut self) -> Result<String, Unsupported> {
        self.expect_ident("do")?;
        self.expect_punct('|')?;
        let var = self.ident()?;
        self.expect_punct('|')?;
        self.end_of_statement()?;
        Ok(var)
    }

    fn end_of_statement(&mut self) -> Result<(), Unsupported> {
        match self.peek() {
            None | Some(Tok::Newline) => Ok(()),
            _ => Err(self.unsupported("unexpected code after the statement")),
        }
    }

    /// Statements of `var` until the block's `end`. `machines` is only
    /// passed for the top-level block, where `vm.define` is allowed.
    fn body(
        &mut self,
        var: &str,
        settings: &mut MachineSettings,
        mut machines: Option<&mut Vec<MachineDefinition>>,
    ) -> Result<(), Unsupported> {
        loop {
            self.skip_newlines();
            match self.peek() {
                None => return Err(self.unsupported("missing `end`")),
                Some(Tok::Ident(w)) if w == "end" => return Ok(()),
                Some(Tok::Ident(w)) if w == var => {}
                Some(Tok::Ident(w)) if w == "config" => {
                    return Err(self.unsupported(
                        "setting `config` inside a machine block isn't supported; use the block's own variable",
                    ));
                }
                Some(other) => {
                    return Err(self.unsupported(format!(
                        "`{}` isn't supported by the visual editor",
                        describe(other)
                    )));
                }
            }
            self.pos += 1;
            self.expect_punct('.')?;
            if self.peek() != Some(&Tok::Ident("vm".into())) {
                let name = self.peek().map(describe).unwrap_or_default();
                return Err(self.unsupported(format!(
                    "`{var}.{name}` settings aren't supported by the visual editor"
                )));
            }
            self.pos += 1;
            self.expect_punct('.')?;
            let line = self.line();
            let setting = self.ident()?;
            match setting.as_str() {
                "box" => set_once(
                    &mut settings.box_name,
                    self.assigned("box")?,
                    line,
                    "vm.box",
                )?,
                "hostname" => set_once(
                    &mut settings.hostname,
                    self.assigned("hostname")?,
                    line,
                    "vm.hostname",
                )?,
                "network" => self.network(settings)?,
                "synced_folder" => self.synced_folder(settings)?,
                "provider" => self.provider(settings, line)?,
                "provision" => self.provision(settings)?,
                "define" => match machines.as_deref_mut() {
                    Some(list) => self.define(list, line)?,
                    None => {
                        return Err(Unsupported::new(
                            line,
                            "`vm.define` inside a machine block isn't supported",
                        ));
                    }
                },
                other => {
                    return Err(Unsupported::new(
                        line,
                        format!("`{var}.vm.{other}` isn't supported by the visual editor"),
                    ));
                }
            }
            self.end_of_statement()?;
        }
    }

    fn assigned(&mut self, what: &str) -> Result<String, Unsupported> {
        self.expect_punct('=')?;
        self.string(what)
    }

    /// `, key: value, …` (newlines are allowed after commas).
    fn args(&mut self) -> Result<Vec<Arg>, Unsupported> {
        let mut args: Vec<Arg> = Vec::new();
        while self.peek() == Some(&Tok::Punct(',')) {
            self.pos += 1;
            self.skip_newlines();
            let line = self.line();
            let Some(Tok::Label(key)) = self.next_tok() else {
                return Err(self.unsupported_prev("expected an option such as `name: value`"));
            };
            let value = match self.next_tok() {
                Some(Tok::Str(s)) => Value::Str(s),
                Some(Tok::Int(n)) => Value::Int(n),
                Some(Tok::Sym(s)) => Value::Sym(s),
                Some(Tok::Ident(w)) if w == "true" => Value::Bool(true),
                Some(Tok::Ident(w)) if w == "false" => Value::Bool(false),
                _ => {
                    return Err(self.unsupported_prev(format!(
                        "the value of `{key}:` must be a string, number, true or false"
                    )));
                }
            };
            if args.iter().any(|a| a.key == key) {
                return Err(Unsupported::new(line, format!("`{key}:` is given twice")));
            }
            args.push(Arg { key, value, line });
        }
        Ok(args)
    }

    fn network(&mut self, settings: &mut MachineSettings) -> Result<(), Unsupported> {
        let kind = self.string("the network type")?;
        let args = self.args()?;
        match kind.as_str() {
            "private_network" => {
                let mut ip = None;
                let mut dhcp = false;
                for a in args {
                    match (a.key.as_str(), a.value) {
                        ("ip", Value::Str(s)) => ip = Some(s),
                        ("type", Value::Str(s)) if s == "dhcp" => dhcp = true,
                        (key, _) => return Err(unknown_option(&a.line, key, "private_network")),
                    }
                }
                if ip.is_some() == dhcp {
                    return Err(self.unsupported_prev(
                        "a private network needs either `ip:` or `type: \"dhcp\"`",
                    ));
                }
                settings.private_networks.push(PrivateNetwork { ip });
            }
            "forwarded_port" => {
                let (mut guest, mut host, mut host_ip, mut protocol) = (None, None, None, None);
                for a in args {
                    match (a.key.as_str(), a.value) {
                        ("guest", Value::Int(n)) => guest = Some(port(n, a.line)?),
                        ("host", Value::Int(n)) => host = Some(port(n, a.line)?),
                        ("host_ip", Value::Str(s)) => host_ip = Some(s),
                        ("protocol", Value::Str(s)) if s == "tcp" => protocol = Some(Protocol::Tcp),
                        ("protocol", Value::Str(s)) if s == "udp" => protocol = Some(Protocol::Udp),
                        (key, _) => return Err(unknown_option(&a.line, key, "forwarded_port")),
                    }
                }
                let (Some(guest), Some(host)) = (guest, host) else {
                    return Err(
                        self.unsupported_prev("a forwarded port needs `guest:` and `host:`")
                    );
                };
                settings.forwarded_ports.push(ForwardedPort {
                    guest,
                    host,
                    host_ip,
                    protocol,
                });
            }
            other => {
                return Err(self.unsupported_prev(format!(
                    "`{other}` networks aren't supported by the visual editor yet"
                )));
            }
        }
        Ok(())
    }

    fn synced_folder(&mut self, settings: &mut MachineSettings) -> Result<(), Unsupported> {
        let host_path = self.string("the host folder")?;
        self.expect_punct(',')?;
        self.skip_newlines();
        let guest_path = self.string("the guest folder")?;
        let mut disabled = false;
        for a in self.args()? {
            match (a.key.as_str(), a.value) {
                ("disabled", Value::Bool(b)) => disabled = b,
                (key, _) => return Err(unknown_option(&a.line, key, "synced_folder")),
            }
        }
        settings.synced_folders.push(SyncedFolder {
            host_path,
            guest_path,
            disabled,
        });
        Ok(())
    }

    fn provider(&mut self, settings: &mut MachineSettings, line: usize) -> Result<(), Unsupported> {
        let name = match self.next_tok() {
            Some(Tok::Str(s) | Tok::Sym(s)) => s,
            _ => return Err(self.unsupported_prev("expected a provider name")),
        };
        let Some(kind) = ProviderKind::parse(&name) else {
            return Err(Unsupported::new(
                line,
                format!("settings for the `{name}` provider aren't supported by the visual editor"),
            ));
        };
        if settings.providers.iter().any(|p| p.provider == kind) {
            return Err(Unsupported::new(
                line,
                format!("a second `{name}` provider block isn't supported"),
            ));
        }
        let var = self.block_header()?;
        let mut block = ProviderSettings {
            provider: kind,
            cpus: None,
            memory_mb: None,
        };
        loop {
            self.skip_newlines();
            let line = self.line();
            match self.next_tok() {
                Some(Tok::Ident(w)) if w == "end" => break,
                Some(Tok::Ident(w)) if w == var => {}
                Some(other) => {
                    return Err(Unsupported::new(
                        line,
                        format!("`{}` isn't supported in a provider block", describe(&other)),
                    ));
                }
                None => return Err(self.unsupported("missing `end`")),
            }
            self.expect_punct('.')?;
            let key = self.ident()?;
            let (field, value) = match (kind, key.as_str()) {
                (ProviderKind::VmwareDesktop, "vmx") => {
                    self.expect_punct('[')?;
                    let vmx_key = self.string("the vmx key")?;
                    self.expect_punct(']')?;
                    self.expect_punct('=')?;
                    let field = match vmx_key.as_str() {
                        "numvcpus" => "cpus",
                        "memsize" => "memory",
                        other => {
                            return Err(Unsupported::new(
                                line,
                                format!("`vmx[\"{other}\"]` isn't supported by the visual editor"),
                            ));
                        }
                    };
                    let Some(Tok::Str(s)) = self.next_tok() else {
                        return Err(self.unsupported_prev("vmx values must be strings"));
                    };
                    (field, number(&s, line)?)
                }
                (_, "cpus" | "memory") => {
                    self.expect_punct('=')?;
                    let value = match self.next_tok() {
                        Some(Tok::Int(n)) => to_u32(n, line)?,
                        Some(Tok::Str(s)) if key == "memory" => number(&s, line)?,
                        _ => return Err(self.unsupported_prev(format!("`{key}` must be a number"))),
                    };
                    (if key == "cpus" { "cpus" } else { "memory" }, value)
                }
                (_, other) => {
                    return Err(Unsupported::new(
                        line,
                        format!(
                            "`{var}.{other}` isn't supported by the visual editor; it supports CPU and memory only"
                        ),
                    ));
                }
            };
            let slot = if field == "cpus" {
                &mut block.cpus
            } else {
                &mut block.memory_mb
            };
            set_once(slot, value, line, field)?;
            self.end_of_statement()?;
        }
        settings.providers.push(block);
        Ok(())
    }

    fn provision(&mut self, settings: &mut MachineSettings) -> Result<(), Unsupported> {
        let kind = self.string("the provisioner type")?;
        if kind != "shell" {
            return Err(self.unsupported_prev(format!(
                "`{kind}` provisioners aren't supported by the visual editor yet"
            )));
        }
        let (mut source, mut run, mut privileged) = (None, None, None);
        for a in self.args()? {
            match (a.key.as_str(), a.value) {
                ("inline", Value::Str(script)) if source.is_none() => {
                    source = Some(ShellSource::Inline { script })
                }
                ("path", Value::Str(path)) if source.is_none() => {
                    source = Some(ShellSource::Path { path })
                }
                ("run", Value::Str(s)) => {
                    run = Some(ProvisionRun::parse(&s).ok_or_else(|| {
                        Unsupported::new(a.line, format!("`run: \"{s}\"` isn't supported"))
                    })?)
                }
                ("privileged", Value::Bool(b)) => privileged = Some(b),
                (key, _) => return Err(unknown_option(&a.line, key, "shell provisioner")),
            }
        }
        let Some(source) = source else {
            return Err(self.unsupported_prev("a shell provisioner needs `inline:` or `path:`"));
        };
        settings.provisioners.push(ShellProvisioner {
            source,
            run,
            privileged,
        });
        Ok(())
    }

    fn define(
        &mut self,
        machines: &mut Vec<MachineDefinition>,
        line: usize,
    ) -> Result<(), Unsupported> {
        let name = match self.next_tok() {
            Some(Tok::Str(s) | Tok::Sym(s)) => s,
            _ => return Err(self.unsupported_prev("the machine name must be a plain string")),
        };
        if machines.iter().any(|m| m.name == name) {
            return Err(Unsupported::new(
                line,
                format!("machine \"{name}\" is defined twice"),
            ));
        }
        let mut machine = MachineDefinition {
            name,
            primary: false,
            autostart: true,
            settings: MachineSettings::default(),
        };
        for a in self.args()? {
            match (a.key.as_str(), a.value) {
                ("primary", Value::Bool(b)) => machine.primary = b,
                ("autostart", Value::Bool(b)) => machine.autostart = b,
                (key, _) => return Err(unknown_option(&a.line, key, "vm.define")),
            }
        }
        let var = self.block_header()?;
        self.body(&var, &mut machine.settings, None)?;
        self.expect_ident("end")?;
        machines.push(machine);
        Ok(())
    }
}

fn describe(tok: &Tok) -> String {
    match tok {
        Tok::Ident(s) | Tok::Const(s) => s.clone(),
        Tok::Label(s) => format!("{s}:"),
        Tok::Str(s) => format!("\"{s}\""),
        Tok::Sym(s) => format!(":{s}"),
        Tok::Int(n) => n.to_string(),
        Tok::Punct(c) => c.to_string(),
        Tok::Newline => "line break".into(),
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T, line: usize, what: &str) -> Result<(), Unsupported> {
    if slot.is_some() {
        return Err(Unsupported::new(
            line,
            format!("`{what}` is set more than once"),
        ));
    }
    *slot = Some(value);
    Ok(())
}

fn unknown_option(line: &usize, key: &str, on: &str) -> Unsupported {
    Unsupported::new(
        *line,
        format!("the `{key}:` option of {on} isn't supported by the visual editor"),
    )
}

fn port(n: u64, line: usize) -> Result<u16, Unsupported> {
    u16::try_from(n)
        .ok()
        .filter(|p| *p > 0)
        .ok_or_else(|| Unsupported::new(line, format!("{n} isn't a valid port")))
}

fn to_u32(n: u64, line: usize) -> Result<u32, Unsupported> {
    u32::try_from(n).map_err(|_| Unsupported::new(line, "number out of range"))
}

fn number(s: &str, line: usize) -> Result<u32, Unsupported> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Unsupported::new(
            line,
            format!("\"{s}\" isn't a whole number"),
        ));
    }
    s.parse()
        .map_err(|_| Unsupported::new(line, "number out of range"))
}
