/// Turns a byte stream into text without splitting characters across reads.
/// Invalid sequences become U+FFFD, like `String::from_utf8_lossy`.
#[derive(Debug, Default)]
pub(crate) struct Utf8Stream {
    pending: Vec<u8>,
}

impl Utf8Stream {
    /// Text decoded so far; an incomplete character at the end waits for
    /// the next call.
    pub(crate) fn push(&mut self, bytes: &[u8]) -> String {
        self.pending.extend_from_slice(bytes);
        let mut out = String::new();
        let mut rest: &[u8] = &self.pending;
        loop {
            match std::str::from_utf8(rest) {
                Ok(text) => {
                    out.push_str(text);
                    rest = &[];
                    break;
                }
                Err(e) => {
                    let (valid, after) = rest.split_at(e.valid_up_to());
                    // `valid_up_to` marks a valid prefix by definition.
                    out.push_str(std::str::from_utf8(valid).unwrap_or_default());
                    match e.error_len() {
                        Some(len) => {
                            out.push(char::REPLACEMENT_CHARACTER);
                            rest = &after[len..];
                        }
                        // Incomplete character at the end: keep it.
                        None => {
                            rest = after;
                            break;
                        }
                    }
                }
            }
        }
        self.pending = rest.to_vec();
        out
    }

    /// Whatever is left when the stream ends.
    pub(crate) fn finish(&mut self) -> String {
        let rest = std::mem::take(&mut self.pending);
        String::from_utf8_lossy(&rest).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::Utf8Stream;

    #[test]
    fn keeps_characters_split_across_reads() {
        let bytes = "größe ✓".as_bytes();
        let mut s = Utf8Stream::default();
        let mut out = String::new();
        for b in bytes {
            out.push_str(&s.push(std::slice::from_ref(b)));
        }
        out.push_str(&s.finish());
        assert_eq!(out, "größe ✓");
    }

    #[test]
    fn replaces_invalid_bytes() {
        let mut s = Utf8Stream::default();
        assert_eq!(s.push(b"a\xffb"), "a\u{fffd}b");
        assert_eq!(s.push(b"\xe2\x9c"), "");
        assert_eq!(s.finish(), "\u{fffd}");
    }
}
