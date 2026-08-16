//! Trading-day calendar — Sina `klc_td_sh.txt`.
//!
//! Port of akshare's `tool_trade_date_hist_sina` (in `tool/trade_date_hist.py`).
//!
//! NOTE: the Sina response is JS bit-packed (akshare decodes it with the
//! `hk_js_decode` routine). We port that decoder to pure Rust below
//! (`decode_sina_calendar`) so no JS engine is required. The decoded form is a
//! JSON array of `{"trade_date": <YYYYMMDD>}`, which [`parse`] consumes.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_SINA};
use crate::core::error::{Error, Result};

const URL: &str = "https://finance.sina.com.cn/realstock/company/klc_td_sh.txt";

/// A single trading day (ISO `YYYY-MM-DD`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TradeDateRow {
    pub date: String,
}

/// Fetch and decode the Sina trading calendar.
pub async fn tool_trade_date(client: &Client) -> Result<Vec<TradeDateRow>> {
    let text = client
        .get_text(SOURCE_SINA, "tool_trade_date", URL, &[], None)
        .await?;
    decode_sina_calendar(&text)
}

/// Parse the decoded JSON array `[{"trade_date": <int|str>}, ...]`.
/// Malformed entries are skipped.
#[allow(dead_code)] // offline test entry point; the live path uses `decode_sina_calendar`
pub(crate) fn parse(resp: &Value) -> Result<Vec<TradeDateRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "expected a JSON array of trade dates".into(),
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let date = match item.get("trade_date") {
            Some(Value::Number(n)) => match n.as_i64() {
                Some(iv) => {
                    let s = iv.to_string();
                    if s.len() == 8 {
                        format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8])
                    } else {
                        continue;
                    }
                }
                None => continue,
            },
            Some(Value::String(s)) => s.clone(),
            _ => continue,
        };
        out.push(TradeDateRow { date });
    }
    Ok(out)
}

/// Pure-Rust port of Sina's `hk_js_decode` `d()` for the trade-date list
/// (dispatch id 139 → `R`). Mirrors the bit-packing reader used by akshare.
fn decode_sina_calendar(text: &str) -> Result<Vec<TradeDateRow>> {
    let enc = text
        .split('=')
        .nth(1)
        .and_then(|s| s.split(';').next())
        .map(|s| s.replace('"', ""))
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "unexpected Sina calendar response shape".into(),
        })?;

    let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let cidx: std::collections::HashMap<char, i64> = alphabet
        .chars()
        .enumerate()
        .map(|(i, c)| (c, i as i64))
        .collect();
    let i_vec: Vec<i64> = enc.chars().filter_map(|c| cidx.get(&c).copied()).collect();

    let mut dec = Decoder {
        i: i_vec,
        n: 0,
        e: 0,
        o: 0,
        r: std::collections::HashMap::new(),
        d: (0..64).map(|k| 1i64 << k).collect(),
        u: 7657,
        s: 0,
    };
    dec.n = dec.i.len() as i64;

    let uselect = dec.w(&[12, 6], &[], &[]);
    dec.s = 63 ^ uselect[1];
    match uselect[0] {
        139 => dec.r_fn(),
        other => Err(Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: format!("unsupported Sina calendar dispatch id {other}"),
        }),
    }
}

struct Decoder {
    i: Vec<i64>,
    n: i64,
    e: i64,
    o: i64,
    r: std::collections::HashMap<&'static str, i64>,
    d: Vec<i64>,
    u: i64,
    s: i64,
}

impl Decoder {
    fn y(&mut self) -> bool {
        if self.e >= self.n {
            return false;
        }
        let bit = (self.i[self.e as usize] >> self.o) & 1;
        self.o += 1;
        if self.o >= 6 {
            self.o -= 6;
            self.e += 1;
        }
        bit == 1
    }

    fn w(&mut self, t: &[i64], r2: &[i64], a: &[bool]) -> Vec<i64> {
        let mut out = Vec::new();
        for (s, &cval) in t.iter().enumerate() {
            let mut uu = 0i64;
            if cval != 0 {
                if self.e >= self.n {
                    return out;
                }
                if cval <= 0 {
                    uu = 0;
                } else if cval <= 30 {
                    let mut remaining = cval;
                    while remaining > 0 {
                        let h = std::cmp::min(6 - self.o, remaining);
                        let chunk = (self.i[self.e as usize] >> self.o) & ((1i64 << h) - 1);
                        uu |= chunk << (cval - remaining);
                        self.o += h;
                        if self.o >= 6 {
                            self.o -= 6;
                            self.e += 1;
                        }
                        remaining -= h;
                    }
                    if r2.get(s).copied().unwrap_or(0) != 0 && uu >= self.d[(cval - 1) as usize] {
                        uu -= self.d[(cval - 1) as usize];
                    }
                } else {
                    let rec = self.w(
                        &[30, cval - 30],
                        &[0, *r2.get(s).unwrap_or(&0)],
                        &[false, *a.get(s).unwrap_or(&false)],
                    );
                    uu = rec[0] + rec[1] * self.d[30];
                }
                out.push(uu);
            } else {
                out.push(0);
            }
        }
        out
    }

    fn n_fn(&mut self) -> i64 {
        let tbit = if self.y() { 1 } else { 0 };
        let mut e2 = 1;
        loop {
            if !self.y() {
                return e2 * (if tbit == 1 { 1 } else { -1 });
            }
            e2 += 1;
        }
    }

    fn s_fn(&mut self, tt: i64) -> (i64, i64, i64) {
        let nmask = *self.r.get("wd").unwrap_or(&62);
        for _ in 0..tt {
            loop {
                let rd = self.r.entry("d").or_insert(0);
                *rd += 1;
                let b = ((*rd % 7) + 10) % 7;
                if (nmask >> b) & 1 == 1 {
                    break;
                }
            }
        }
        let total = self.u + self.r.get("d").copied().unwrap_or(0);
        ymd_from_epoch_days(total)
    }

    fn r_fn(&mut self) -> Result<Vec<TradeDateRow>> {
        if self.s > 1 {
            return Ok(Vec::new());
        }
        self.r.insert("l", 0);
        let mut ncnt = -1i64;
        let rd = self.w(&[18], &[1], &[])[0] - 1;
        self.r.insert("d", rd);
        let icount = self.w(&[18], &[], &[])[0];
        let mut out: Vec<TradeDateRow> = Vec::new();
        let mut started = false;
        while self.r.get("d").copied().unwrap_or(0) < icount {
            let (y, m, d) = self.s_fn(1);
            let date = format!("{y:04}-{m:02}-{d:02}");
            if ncnt <= 0 {
                if self.y() {
                    let lval = *self.r.get("l").unwrap_or(&0);
                    let nfv = self.n_fn();
                    self.r.insert("l", lval + nfv);
                }
                let rl = *self.r.get("l").unwrap_or(&0);
                ncnt = self.w(&[3 * rl], &[0], &[])[0] + 1;
                if !started {
                    started = true;
                    out.push(TradeDateRow { date });
                    ncnt -= 1;
                }
            } else {
                out.push(TradeDateRow { date });
            }
            ncnt -= 1;
        }
        Ok(out)
    }
}

/// Inverse of `days_from_civil`: convert days since 1970-01-01 to `(year, month, day)`.
fn ymd_from_epoch_days(z0: i64) -> (i64, i64, i64) {
    let z = z0 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_decoded_calendar_fixture() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tool_trade_date.json");
        let v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let rows = parse(&v).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "1990-12-19");
        assert_eq!(rows[1].date, "1990-12-20");
        assert_eq!(rows[2].date, "1990-12-21");
    }

    #[test]
    fn epoch_day_math_matches_known_date() {
        // 7657 days after 1970-01-01 is the first A-share trading day.
        let (y, m, d) = ymd_from_epoch_days(7657);
        assert_eq!((y, m, d), (1990, 12, 19));
    }
}
