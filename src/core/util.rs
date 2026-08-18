//! Small shared helpers used by more than one HTTP backend.
//!
//! Keeping these in one place means the `reqwest` and `impersonate` backends
//! percent-encode identically (see architecture review C2) — a `urlencode`
//! that diverged between the two would silently corrupt form/query bodies.

/// Percent-encode a string for a query string or `application/x-www-form-urlencoded`
/// body (RFC 3986). Unreserved alphanumerics and `- _ . ~` pass through; spaces
/// become `+`; everything else becomes `%XX`.
pub fn urlencode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push(hex_digit(b >> 4));
                out.push(hex_digit(b & 0x0f));
            }
        }
    }
    out
}

fn hex_digit(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        _ => (b'A' + (n - 10)) as char,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_spaces_and_specials() {
        assert_eq!(urlencode("a b"), "a+b");
        assert_eq!(urlencode("a&b=c"), "a%26b%3Dc");
        assert_eq!(urlencode("中文"), "%E4%B8%AD%E6%96%87");
    }
}
