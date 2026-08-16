//! Sina A-share daily history (`stock_zh_a_daily`).
//!
//! Port of `akshare.stock.stock_zh_a_sina.stock_zh_a_daily`. The historical
//! Sina endpoint `CN_MarketDataService.getKLineData` requires a `token` query
//! parameter that is derived from a JS-seeded arithmetic (an MD5 of a constructed
//! string). akshare computed it via `py_mini_racer`; here we re-implement the
//! arithmetic in pure Rust (no JS engine) — see [`sign_token`] and the inline
//! [`md5`] module.
//!
//! NOTE / calibration: the live Sina token scheme has changed over time
//! (`getToken.js` now 404s and the endpoint returns `{"__ERROR":1,...}` without a
//! valid token). The request pipeline, params and CSV parser below are faithful;
//! the exact token seed-extraction and [`TOKEN_SALT`] are the calibration points
//! (see [`sign_token`]). The response is parsed as CSV per akshare's contract;
//! if Sina switches to JSON, [`parse_csv`] will surface an [`Error::Parse`].

use crate::core::client::{Client, SOURCE_SINA};
use crate::core::error::{Error, Result};

/// Endpoint name used for client rate-limiting / error context.
const ENDPOINT: &str = "stock_zh_a_daily";

/// Sina K-line daily history endpoint.
const BASE_URL: &str =
    "https://quotes.sina.cn/cn/api/json_v2.php/CN_MarketDataService.getKLineData";

/// Bootstrap JS that historically yields the integer seed for the token. Returns
/// 404 today; left as the documented source and a calibration point.
const TOKEN_JS_URL: &str = "https://quotes.sina.cn/cn/api/js/getToken.js";

/// Fixed salt Sina concatenates with the seed before MD5. VERIFY against the live
/// `getToken.js` — this is the primary calibration knob for [`sign_token`].
const TOKEN_SALT: &str = "inter";

/// One daily OHLC row (akshare output contract: `date, open, high, low, close,
/// volume` plus the optional `outstanding_share` / `turnover` columns).
#[derive(Debug, Clone, serde::Serialize)]
pub struct DailySinaRow {
    pub date: String,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
    pub volume: Option<f64>,
    pub outstanding_share: Option<f64>,
    pub turnover: Option<f64>,
}

/// Map akshare's `adjust` to the Sina `type` query param.
fn adjust_to_type(adjust: &str) -> Result<&'static str> {
    match adjust {
        "" => Ok(""),
        "qfq" => Ok("qfq"),
        "hfq" => Ok("hfq"),
        "qfq-factor" | "hfq-factor" => Err(Error::InvalidParam(format!(
            "adjust `{adjust}` needs the separate factor endpoint (not part of getKLineData)"
        ))),
        other => Err(Error::InvalidParam(format!("unknown adjust: {other}"))),
    }
}

/// Fetch and compute Sina's request `token`.
///
/// Algorithm (pure-Rust port of the historical `py_mini_racer` flow):
/// 1. GET [`TOKEN_JS_URL`] for a seed integer (`dan`).
/// 2. `token = md5(dan + TOKEN_SALT)`.
///
/// The upstream bootstrap is currently dead (404), so this returns
/// [`Error::UpstreamChanged`] when it cannot obtain/derive a seed — that is the
/// network-calibration signal. Adjust [`TOKEN_SALT`] and [`extract_dan`] against
/// the live `getToken.js` once Sina restores or replaces the scheme.
async fn sign_token(client: &Client) -> Result<String> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let ts = now_ms.to_string();
    let params = [("_", ts.as_str())];
    let js = client
        .get_text(SOURCE_SINA, ENDPOINT, TOKEN_JS_URL, &params, None)
        .await?;
    let dan = extract_dan(&js).ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "could not extract token seed from getToken.js (upstream changed)".into(),
    })?;
    Ok(md5::hash_hex(format!("{dan}{TOKEN_SALT}").as_bytes()))
}

/// Pull the seed integer out of `getToken.js`. Historically the script ends with
/// `var dan = (function(){ ... return <N>; })();` — we take the longest run of
/// digits (>= 10) as the seed. Calibration point if Sina restructures the script.
fn extract_dan(js: &str) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    let mut cur = String::new();
    let mut start = 0;
    for (i, c) in js.char_indices() {
        if c.is_ascii_digit() {
            if cur.is_empty() {
                start = i;
            }
            cur.push(c);
        } else {
            if cur.len() >= 10 {
                best = Some((start, std::mem::take(&mut cur)));
            } else {
                cur.clear();
            }
        }
    }
    if cur.len() >= 10 {
        best = Some((start, cur));
    }
    best.map(|(_, s)| s)
}

/// Sina A-share daily history (CSV response, parsed into [`DailySinaRow`]s).
///
/// `adjust` is `""` (none) / `"qfq"` / `"hfq"` (factor endpoints are rejected).
/// Malformed CSV rows are skipped. Sends a `Referer` header and the signed `token`.
pub async fn stock_zh_a_daily(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
    adjust: &str,
) -> Result<Vec<DailySinaRow>> {
    let type_param = adjust_to_type(adjust)?;
    let token = sign_token(client).await?;

    let params: [(&str, &str); 5] = [
        ("symbol", symbol),
        ("begin_date", start_date),
        ("end_date", end_date),
        ("type", type_param),
        ("token", token.as_str()),
    ];
    let headers = [("Referer", "https://finance.sina.com.cn/")];

    let text = client
        .get_text(SOURCE_SINA, ENDPOINT, BASE_URL, &params, Some(&headers))
        .await?;

    parse_csv(&text)
}

/// Parse a Sina K-line CSV into rows, skipping malformed lines.
///
/// Header is matched by column name so the optional `outstanding_share` /
/// `turnover` columns may be present or absent. A row without a `date` is skipped;
/// a numeric cell that is empty / `null` / `nan` / `--` becomes `None`.
pub(crate) fn parse_csv(text: &str) -> Result<Vec<DailySinaRow>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(text.as_bytes());

    let headers = rdr.headers().map_err(|e| Error::Csv(e.to_string()))?;
    let idx = |name: &str| headers.iter().position(|h| h.trim() == name);

    let i_date = idx("date");
    let i_open = idx("open");
    let i_high = idx("high");
    let i_low = idx("low");
    let i_close = idx("close");
    let i_volume = idx("volume");
    let i_out = idx("outstanding_share");
    let i_turn = idx("turnover");

    let i_date = i_date.ok_or_else(|| Error::Parse {
        endpoint: ENDPOINT,
        message: "CSV missing required `date` column".into(),
    })?;

    let mut out = Vec::new();
    for rec in rdr.records() {
        let rec = match rec {
            Ok(r) => r,
            Err(_) => continue, // malformed row -> skip
        };
        let cell =
            |i: Option<usize>| -> Option<&str> { i.and_then(|i| rec.get(i)).map(|s| s.trim()) };
        let date = match cell(Some(i_date)) {
            Some(d) if !d.is_empty() => d.to_string(),
            _ => continue, // no date -> skip
        };
        out.push(DailySinaRow {
            date,
            open: cell(i_open).and_then(parse_opt_f64),
            high: cell(i_high).and_then(parse_opt_f64),
            low: cell(i_low).and_then(parse_opt_f64),
            close: cell(i_close).and_then(parse_opt_f64),
            volume: cell(i_volume).and_then(parse_opt_f64),
            outstanding_share: cell(i_out).and_then(parse_opt_f64),
            turnover: cell(i_turn).and_then(parse_opt_f64),
        });
    }
    Ok(out)
}

fn parse_opt_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() || t.eq_ignore_ascii_case("null") || t.eq_ignore_ascii_case("nan") || t == "--"
    {
        None
    } else {
        t.parse::<f64>().ok()
    }
}

/// Minimal, dependency-free MD5 (RFC 1321) — used only for Sina token signing.
mod md5 {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];

    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];

    pub fn hash(input: &[u8]) -> [u8; 16] {
        let mut a0: u32 = 0x67452301;
        let mut b0: u32 = 0xefcdab89;
        let mut c0: u32 = 0x98badcfe;
        let mut d0: u32 = 0x10325476;

        let mut msg = input.to_vec();
        let orig_len_bits = (input.len() as u64).wrapping_mul(8);
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&orig_len_bits.to_le_bytes());

        for chunk in msg.chunks(64) {
            let mut m = [0u32; 16];
            for (i, word) in m.iter_mut().enumerate() {
                *word = u32::from_le_bytes([
                    chunk[4 * i],
                    chunk[4 * i + 1],
                    chunk[4 * i + 2],
                    chunk[4 * i + 3],
                ]);
            }
            let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
            for i in 0..64 {
                let (f, g) = match i {
                    0..=15 => ((b & c) | (!b & d), i),
                    16..=31 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                    32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                    _ => (c ^ (b | !d), (7 * i) % 16),
                };
                let f = f
                    .wrapping_add(a)
                    .wrapping_add(K[i])
                    .wrapping_add(m[g])
                    .rotate_left(S[i])
                    .wrapping_add(b);
                a = d;
                d = c;
                c = b;
                b = f;
            }
            a0 = a0.wrapping_add(a);
            b0 = b0.wrapping_add(b);
            c0 = c0.wrapping_add(c);
            d0 = d0.wrapping_add(d);
        }

        let mut out = [0u8; 16];
        out[0..4].copy_from_slice(&a0.to_le_bytes());
        out[4..8].copy_from_slice(&b0.to_le_bytes());
        out[8..12].copy_from_slice(&c0.to_le_bytes());
        out[12..16].copy_from_slice(&d0.to_le_bytes());
        out
    }

    pub fn hash_hex(input: &[u8]) -> String {
        hash(input).iter().map(|b| format!("{b:02x}")).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_sina_daily_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/stock_zh_a_daily_sina.csv");
        let txt = std::fs::read_to_string(path).unwrap();
        let rows = parse_csv(&txt).unwrap();

        // 4 valid rows; one row has an empty date (skipped) and one has the
        // wrong field count (skipped).
        assert_eq!(rows.len(), 4);

        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].open, Some(10.10));
        assert_eq!(rows[0].high, Some(10.80));
        assert_eq!(rows[0].low, Some(9.90));
        assert_eq!(rows[0].close, Some(10.50));
        assert_eq!(rows[0].volume, Some(123456.0));
        assert_eq!(rows[0].outstanding_share, Some(800_000_000.0));
        assert_eq!(rows[0].turnover, Some(0.0154));

        // Row with a non-numeric `open` keeps the row but yields None for that cell.
        assert_eq!(rows[1].date, "2024-01-03");
        assert_eq!(rows[1].open, None);
        assert_eq!(rows[1].close, Some(10.20));

        // Last valid row.
        assert_eq!(rows[3].date, "2024-01-05");
        assert_eq!(rows[3].close, Some(11.00));
    }

    #[test]
    fn md5_known_vector() {
        // RFC 1321 / akshare outcrypto.js reference vector.
        assert_eq!(md5::hash_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }
}
