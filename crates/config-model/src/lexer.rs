//! Tokens for the small part of Ruby that the recognizer accepts. Anything
//! outside it (operators, interpolation, `%w[]`, `=begin`, …) is reported as
//! unsupported rather than guessed at.

use crate::Unsupported;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tok {
    /// Identifiers and keywords (`config`, `do`, `end`, `true`).
    Ident(String),
    /// Capitalized names (`Vagrant`).
    Const(String),
    /// A keyword-argument key: `ip:`.
    Label(String),
    /// A string or heredoc, already decoded to the value Ruby would produce.
    Str(String),
    Sym(String),
    Int(u64),
    Punct(char),
    Newline,
}

#[derive(Debug, Clone)]
pub(crate) struct Token {
    pub tok: Tok,
    pub line: usize,
}

pub(crate) struct Lexed {
    pub tokens: Vec<Token>,
    /// (line, text) of every comment, including the leading `#`.
    pub comments: Vec<(usize, String)>,
}

struct PendingHeredoc {
    token: usize,
    line: usize,
    terminator: String,
    /// `<<-` or `<<~`: the terminator may be indented.
    indented_end: bool,
    squiggly: bool,
    /// `<<~'EOS'`: no escapes or interpolation.
    raw: bool,
}

/// Lexes LF-only source.
pub(crate) fn lex(src: &str) -> Result<Lexed, Unsupported> {
    let lines: Vec<&str> = src.split('\n').collect();
    let mut out = Lexed {
        tokens: Vec::new(),
        comments: Vec::new(),
    };
    let mut i = 0;
    while i < lines.len() {
        let line_no = i + 1;
        let line = lines[i];
        if line.starts_with("=begin") || line == "__END__" {
            return Err(Unsupported::new(line_no, "this syntax isn't supported"));
        }
        let pending = lex_line(line, line_no, &mut out)?;
        i += 1;
        for h in pending {
            let mut body = Vec::new();
            loop {
                let Some(&l) = lines.get(i) else {
                    return Err(Unsupported::new(h.line, "unterminated heredoc"));
                };
                i += 1;
                let end = if h.indented_end { l.trim_start() } else { l };
                if end == h.terminator {
                    break;
                }
                body.push(l);
            }
            let value = heredoc_value(&body, &h)?;
            out.tokens[h.token].tok = Tok::Str(value);
        }
        out.tokens.push(Token {
            tok: Tok::Newline,
            line: line_no,
        });
    }
    Ok(out)
}

fn lex_line(
    line: &str,
    line_no: usize,
    out: &mut Lexed,
) -> Result<Vec<PendingHeredoc>, Unsupported> {
    let err = |msg: &str| Unsupported::new(line_no, msg);
    let chars: Vec<char> = line.chars().collect();
    let mut pending = Vec::new();
    let mut p = 0;
    let push = |out: &mut Lexed, tok: Tok| out.tokens.push(Token { tok, line: line_no });
    while p < chars.len() {
        let c = chars[p];
        match c {
            ' ' | '\t' => p += 1,
            '#' => {
                out.comments
                    .push((line_no, chars[p..].iter().collect::<String>()));
                break;
            }
            '"' | '\'' => {
                let start = p + 1;
                let mut q = start;
                while q < chars.len() && chars[q] != c {
                    if chars[q] == '\\' {
                        q += 1;
                    }
                    q += 1;
                }
                if q >= chars.len() {
                    return Err(err("strings must start and end on the same line"));
                }
                let text: String = chars[start..q].iter().collect();
                let value = if c == '"' {
                    decode_double_quoted(&text).map_err(|m| err(&m))?
                } else {
                    decode_single_quoted(&text)
                };
                push(out, Tok::Str(value));
                p = q + 1;
            }
            '0'..='9' => {
                let start = p;
                while p < chars.len() && chars[p].is_ascii_digit() {
                    p += 1;
                }
                if p < chars.len() && (chars[p].is_alphanumeric() || matches!(chars[p], '_' | '.'))
                {
                    return Err(err("only whole numbers are supported"));
                }
                let digits: String = chars[start..p].iter().collect();
                let n = digits.parse().map_err(|_| err("number out of range"))?;
                push(out, Tok::Int(n));
            }
            ':' => {
                if chars
                    .get(p + 1)
                    .is_some_and(|c| c.is_ascii_alphabetic() || *c == '_')
                {
                    let (name, end) = word(&chars, p + 1);
                    push(out, Tok::Sym(name));
                    p = end;
                } else {
                    return Err(err("this syntax isn't supported"));
                }
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let (name, mut end) = word(&chars, p);
                if matches!(chars.get(end), Some('?' | '!')) {
                    return Err(err(&format!("`{name}{}` isn't supported", chars[end])));
                }
                let tok = if chars.get(end) == Some(&':') && chars.get(end + 1) != Some(&':') {
                    end += 1;
                    Tok::Label(name)
                } else if c.is_ascii_uppercase() {
                    Tok::Const(name)
                } else {
                    Tok::Ident(name)
                };
                push(out, tok);
                p = end;
            }
            '<' if chars.get(p + 1) == Some(&'<') => {
                let (h, end) = heredoc_start(&chars, p + 2, line_no)?;
                pending.push(PendingHeredoc {
                    token: out.tokens.len(),
                    ..h
                });
                push(out, Tok::Str(String::new()));
                p = end;
            }
            '=' if matches!(chars.get(p + 1), Some('=' | '>' | '~')) => {
                return Err(err("operators aren't supported"));
            }
            '.' | ',' | '(' | ')' | '=' | '|' | '[' | ']' => {
                if c == '.' && chars.get(p + 1) == Some(&'.') {
                    return Err(err("ranges aren't supported"));
                }
                push(out, Tok::Punct(c));
                p += 1;
            }
            _ => return Err(err(&format!("`{c}` isn't supported here"))),
        }
    }
    Ok(pending)
}

fn word(chars: &[char], start: usize) -> (String, usize) {
    let mut end = start;
    while end < chars.len() && (chars[end].is_ascii_alphanumeric() || chars[end] == '_') {
        end += 1;
    }
    (chars[start..end].iter().collect(), end)
}

/// Parses `-ID`, `~'ID'`, … after `<<`.
fn heredoc_start(
    chars: &[char],
    mut p: usize,
    line: usize,
) -> Result<(PendingHeredoc, usize), Unsupported> {
    let (mut indented_end, mut squiggly) = (false, false);
    match chars.get(p) {
        Some('-') => {
            indented_end = true;
            p += 1;
        }
        Some('~') => {
            indented_end = true;
            squiggly = true;
            p += 1;
        }
        _ => {}
    }
    let quote = chars.get(p).copied().filter(|c| matches!(c, '\'' | '"'));
    if quote.is_some() {
        p += 1;
    }
    if !chars
        .get(p)
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == '_')
    {
        return Err(Unsupported::new(line, "this syntax isn't supported"));
    }
    let (terminator, mut end) = word(chars, p);
    if let Some(q) = quote {
        if chars.get(end) != Some(&q) {
            return Err(Unsupported::new(line, "unterminated heredoc name"));
        }
        end += 1;
    }
    Ok((
        PendingHeredoc {
            token: 0,
            line,
            terminator,
            indented_end,
            squiggly,
            raw: quote == Some('\''),
        },
        end,
    ))
}

fn heredoc_value(body: &[&str], h: &PendingHeredoc) -> Result<String, Unsupported> {
    let err = |msg: &str| Unsupported::new(h.line, msg);
    let mut lines: Vec<String> = body.iter().map(|l| (*l).to_owned()).collect();
    if h.squiggly {
        if !h.raw && body.iter().any(|l| l.contains('\\')) {
            return Err(err("backslashes in a `<<~` heredoc aren't supported"));
        }
        if body.iter().any(|l| {
            l.chars()
                .take_while(|c| c.is_whitespace())
                .any(|c| c == '\t')
        }) {
            return Err(err("tab indentation in a `<<~` heredoc isn't supported"));
        }
        let indent = body
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.len() - l.trim_start_matches(' ').len())
            .min()
            .unwrap_or(0);
        for l in &mut lines {
            let leading = l.len() - l.trim_start_matches(' ').len();
            l.drain(..leading.min(indent));
        }
    }
    let mut text = String::new();
    for l in &lines {
        text.push_str(l);
        text.push('\n');
    }
    if h.raw {
        Ok(text)
    } else {
        decode_double_quoted(&text).map_err(|m| err(&m))
    }
}

/// Ruby double-quoted string semantics for the escapes Vagrantfiles use;
/// interpolation and numeric escapes are refused.
pub(crate) fn decode_double_quoted(s: &str) -> Result<String, String> {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('s') => out.push(' '),
                Some('r') => out.push('\r'),
                Some('e') => out.push('\u{1b}'),
                Some('a') => out.push('\u{7}'),
                Some('b') => out.push('\u{8}'),
                Some('f') => out.push('\u{c}'),
                Some('v') => out.push('\u{b}'),
                Some('\n') => {}
                Some(c @ ('0'..='9' | 'x' | 'u' | 'c' | 'C' | 'M')) => {
                    return Err(format!("the escape `\\{c}` isn't supported"));
                }
                Some(c) => out.push(c),
                None => return Err("a string can't end with a backslash".into()),
            },
            '#' if matches!(chars.peek(), Some('{' | '@' | '$')) => {
                return Err("string interpolation (`#{…}`) isn't supported".into());
            }
            c => out.push(c),
        }
    }
    Ok(out)
}

fn decode_single_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && matches!(chars.peek(), Some('\\' | '\'')) {
            out.push(chars.next().expect("peeked"));
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Tok> {
        lex(src)
            .unwrap()
            .tokens
            .into_iter()
            .map(|t| t.tok)
            .collect()
    }

    #[test]
    fn lexes_statements_labels_and_symbols() {
        assert_eq!(
            toks(r#"a.vm.network "x", ip: "1", guest: 80, k: :sym # note"#),
            vec![
                Tok::Ident("a".into()),
                Tok::Punct('.'),
                Tok::Ident("vm".into()),
                Tok::Punct('.'),
                Tok::Ident("network".into()),
                Tok::Str("x".into()),
                Tok::Punct(','),
                Tok::Label("ip".into()),
                Tok::Str("1".into()),
                Tok::Punct(','),
                Tok::Label("guest".into()),
                Tok::Int(80),
                Tok::Punct(','),
                Tok::Label("k".into()),
                Tok::Sym("sym".into()),
                Tok::Newline,
            ]
        );
        assert_eq!(
            lex("x # note\n").unwrap().comments,
            vec![(1, "# note".into())]
        );
    }

    #[test]
    fn decodes_strings_like_ruby() {
        assert_eq!(toks(r#""a\"b\\c\n\$x""#)[0], Tok::Str("a\"b\\c\n$x".into()));
        assert_eq!(toks(r"'a\'b\\c\n'")[0], Tok::Str("a'b\\c\\n".into()));
        assert!(lex(r##""#{x}""##).is_err());
        assert!(lex(r#""\x41""#).is_err());
        assert!(lex("\"open\n").is_err());
    }

    #[test]
    fn heredoc_variants() {
        let dash = "x <<-SHELL\n    echo a \\\n      b\n  SHELL\n";
        assert_eq!(toks(dash)[1], Tok::Str("    echo a       b\n".into()));
        let squiggly = "x <<~SHELL\n    echo a\n      b\n\n  SHELL\n";
        assert_eq!(toks(squiggly)[1], Tok::Str("echo a\n  b\n\n".into()));
        let raw = "x <<-'EOS'\n  #{not} \\n\n  EOS\n";
        assert_eq!(toks(raw)[1], Tok::Str("  #{not} \\n\n".into()));
        assert!(lex("x <<-EOS\n  #{y}\nEOS\n").is_err());
        assert!(lex("x <<-EOS\n a\n").is_err());
        // A plain `<<EOS` must end at column 0.
        assert!(lex("x <<EOS\n a\n EOS\n").is_err());
    }

    #[test]
    fn line_numbers_follow_heredoc_bodies() {
        let lexed = lex("a <<-S\n1\n2\nS\nb\n").unwrap();
        let b = lexed
            .tokens
            .iter()
            .find(|t| t.tok == Tok::Ident("b".into()))
            .unwrap();
        assert_eq!(b.line, 5);
    }

    #[test]
    fn refuses_operators_and_unknown_syntax() {
        for src in [
            "a == b",
            "a => b",
            "%w[a b]",
            "a ? b : c",
            "x = 1.5",
            "a..b",
            "x!",
        ] {
            assert!(lex(src).is_err(), "{src}");
        }
        assert!(lex("=begin\n=end\n").is_err());
    }
}
