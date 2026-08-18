//! Canonical JSON value extractors — the single lever point for pulling typed
//! fields out of upstream `serde_json::Value` maps and arrays.
//!
//! Domain modules used to re-declare tiny `fstr`/`fnum`/`parse_f64`/`inum`
//! helpers ~220 times (see architecture review C1). Those copies disagreed on
//! how to treat `"-"`, empty strings, and thousands separators, and the same
//! bug-prone logic lived in 100+ homes. This module is where that logic now
//! lives: fixing a parsing edge case here fixes it everywhere it is used.
//!
//! Unified conventions (the inconsistencies this replaces):
//! - String fields are trimmed; `"-"` and `""` collapse to `None` (upstream
//!   "no data" sentinels).
//! - Float/integer fields tolerate thousands separators (`,`) and the `"-"`
//!   sentinel in string form.

use serde_json::Value;

/// Extract a string field, trimmed.
///
/// Returns `None` when the key is missing, the value is `null`, it is not a
/// string, or the trimmed string is `""` / `"-"`.
pub fn opt_str(v: &Value, k: &str) -> Option<String> {
    match v.get(k) {
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() || t == "-" {
                None
            } else {
                Some(t.to_string())
            }
        }
        Some(Value::Null) | None => None,
        _ => None,
    }
}

/// Like [`opt_str`] but yields `default` instead of `None` when absent.
///
/// Use this for the `fn fstr(item, k) -> String` convention that returned
/// `""` on a missing field.
pub fn opt_str_or(v: &Value, k: &str, default: &str) -> String {
    opt_str(v, k).unwrap_or_else(|| default.to_string())
}

/// Extract a float field.
///
/// Handles `Number` values directly and `String` values (trimmed, with
/// thousands separators stripped and `"-"`/`""` → `None`). Anything else
/// (including `bool`/`null`/missing) → `None`.
pub fn opt_f64(v: &Value, k: &str) -> Option<f64> {
    match v.get(k) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => parse_f64_str(s),
        _ => None,
    }
}

/// Extract an integer field → `i64`.
///
/// Handles `Number` values directly and `String` values (trimmed, comma-stripped,
/// `"-"`/`""` → `None`).
pub fn opt_i64(v: &Value, k: &str) -> Option<i64> {
    match v.get(k) {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() || t == "-" {
                None
            } else {
                t.replace(',', "").parse::<i64>().ok()
            }
        }
        _ => None,
    }
}

/// Parse a float from a string, tolerating thousands separators, surrounding
/// whitespace, and the `"-"`/`""` sentinels (→ `None`). Replaces the many
/// `fn parse_f64(s: &str) -> Option<f64>` copies.
pub fn parse_f64_str(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() || t == "-" {
        return None;
    }
    t.replace(',', "").parse::<f64>().ok()
}

/// Index an array and return the element as a trimmed `String`; missing or
/// `null` → `""`. Matches the `fn str_at(arr, i) -> String` convention.
pub fn str_at(arr: &[Value], i: usize) -> String {
    match arr.get(i) {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Null) => String::new(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// Index an array and return the element as a `f64` (string-aware, comma-safe).
/// Matches the `fn num_at(arr, i) -> Option<f64>` / `fn f64_at(arr, i)` conventions.
pub fn f64_at(arr: &[Value], i: usize) -> Option<f64> {
    match arr.get(i) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => parse_f64_str(s),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn opt_str_variants() {
        let v = json!({"a": "x", "b": "  ", "c": "-", "d": null, "e": 5});
        assert_eq!(opt_str(&v, "a"), Some("x".into()));
        assert_eq!(opt_str(&v, "b"), None); // whitespace-only collapses
        assert_eq!(opt_str(&v, "c"), None); // "-" sentinel
        assert_eq!(opt_str(&v, "d"), None);
        assert_eq!(opt_str(&v, "missing"), None);
        assert_eq!(opt_str(&v, "e"), None); // non-string → None
    }

    #[test]
    fn opt_str_or_default() {
        let v = json!({"a": "x"});
        assert_eq!(opt_str_or(&v, "a", ""), "x");
        assert_eq!(opt_str_or(&v, "missing", ""), "");
        assert_eq!(opt_str_or(&v, "missing", "0"), "0");
    }

    #[test]
    fn opt_f64_number_and_string() {
        let v = json!({"n": 1.5, "s": "2,000.25", "dash": "-", "empty": "", "bad": "abc"});
        assert_eq!(opt_f64(&v, "n"), Some(1.5));
        assert_eq!(opt_f64(&v, "s"), Some(2000.25)); // comma stripped
        assert_eq!(opt_f64(&v, "dash"), None);
        assert_eq!(opt_f64(&v, "empty"), None);
        assert_eq!(opt_f64(&v, "bad"), None);
        assert_eq!(opt_f64(&v, "missing"), None);
    }

    #[test]
    fn opt_i64_comma_and_dash() {
        let v = json!({"n": 42, "s": "1,234", "dash": "-"});
        assert_eq!(opt_i64(&v, "n"), Some(42));
        assert_eq!(opt_i64(&v, "s"), Some(1234));
        assert_eq!(opt_i64(&v, "dash"), None);
    }

    #[test]
    fn parse_f64_str_sentinels() {
        assert_eq!(parse_f64_str("1,234.5"), Some(1234.5));
        assert_eq!(parse_f64_str(" - "), None);
        assert_eq!(parse_f64_str(""), None);
    }

    #[test]
    fn array_indexers() {
        let arr = vec![json!(" a "), json!(null), json!("1,000"), json!(7)];
        assert_eq!(str_at(&arr, 0), "a"); // trimmed
        assert_eq!(str_at(&arr, 1), ""); // null
        assert_eq!(str_at(&arr, 2), "1,000");
        assert_eq!(str_at(&arr, 99), ""); // out of bounds
        assert_eq!(f64_at(&arr, 2), Some(1000.0));
        assert_eq!(f64_at(&arr, 3), Some(7.0));
        assert_eq!(f64_at(&arr, 99), None);
    }
}
