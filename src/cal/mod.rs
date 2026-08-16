//! Realized-volatility helpers ported from `akshare/cal/rv.py`.
//!
//! The only genuinely portable piece is the **Yang-Zhang realized volatility**
//! estimator (`volatility_yz_rv`), which is a pure OHLC → daily-RV computation
//! with no I/O. The two `rv_from_*` helpers in akshare are thin "fetch-minute-
//! bars-then-format" wrappers; we port the one whose underlying minute-bar
//! fetch already exists in this crate and DEFER the other.
//!
//! | Rust fn | akshare source | status |
//! |---|---|---|
//! | `volatility_yz_rv` | `cal/rv.py:92` | DONE (pure) |
//! | `rv_from_stock_zh_a_hist_min_em` | `cal/rv.py:13` | DONE (wraps `stock::misc::stock_zh_a_hist_min_em`) |
//! | `rv_from_futures_zh_minute_sina` | `cal/rv.py:61` | DONE (wraps `futures::sina::futures_zh_minute_sina`) |
//!
//! ## Yang-Zhang formula (as implemented by akshare)
//!
//! ```text
//! RV = sqrt( (1-k) * Vrs + Vo + k * Vc )
//!   Vrs = mean over the day of ( ui*(ui-ci) + di*(di-ci) )
//!         ui = ln(Hi/Oi), ci = ln(Ci/Oi), di = ln(Li/Oi)
//!   Vo  = sample variance over the day of oi,  oi = ln(Oi / C_{i-1})
//!   Vc  = sample variance over the day of ci
//!   k   = 0.34 / (1.34 + (n+1)/(n-1)),  n = intraday_bars / trading_days
//! ```
//! Days with fewer than two intraday bars contribute no variance and are
//! dropped (matching akshare's `.dropna()`).

use crate::core::error::Result;
use crate::futures::sina::futures_zh_minute_sina;
use crate::stock::misc::stock_zh_a_hist_min_em;

/// One OHLC bar, the minimal input to [`volatility_yz_rv`].
///
/// `date` is any timestamp string; only its leading `YYYY-MM-DD` (or `YYYY/MM/DD`)
/// portion is used for the daily grouping that the estimator performs.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OhlcRow {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

/// One day's Yang-Zhang realized-volatility estimate.
#[derive(Debug, Clone, serde::Serialize)]
pub struct YangZhangRvRow {
    /// Trading day (date portion of the input timestamps).
    pub date: String,
    /// Daily Yang-Zhang realized volatility (decimal, not percent).
    pub rv: f64,
}

/// Extract the calendar-day key (`YYYY-MM-DD` / `YYYY/MM/DD`) from a timestamp
/// string. Falls back to the whole string when no separator is found.
fn day_key(date: &str) -> String {
    let trimmed = date.trim();
    // Take the leading date token before any space or 'T'.
    let head = trimmed.split([' ', 'T']).next().unwrap_or(trimmed);
    head.to_string()
}

/// Yang-Zhang realized volatility from a series of intraday OHLC bars.
///
/// `rows` must be in chronological order (oldest first), as returned by the
/// `rv_from_*` helpers or any minute-bar fetch. The function groups bars by
/// calendar day, drops the first bar of the whole series (it has no preceding
/// close for `oi`), and drops any day with fewer than two intraday bars.
pub fn volatility_yz_rv(rows: &[OhlcRow]) -> Vec<YangZhangRvRow> {
    if rows.len() < 2 {
        return Vec::new();
    }

    // Per-bar intraday statistics for i >= 1 (needs previous close).
    struct Bar {
        day: String,
        ui: f64,
        ci: f64,
        di: f64,
        oi: f64,
    }
    let mut bars: Vec<Bar> = Vec::with_capacity(rows.len() - 1);
    for w in rows.windows(2) {
        let prev = &w[0];
        let cur = &w[1];
        let ui = (cur.high / cur.open).ln();
        let ci = (cur.close / cur.open).ln();
        let di = (cur.low / cur.open).ln();
        let oi = (cur.open / prev.close).ln();
        bars.push(Bar {
            day: day_key(&cur.date),
            ui,
            ci,
            di,
            oi,
        });
    }

    // Group by day.
    let mut days: Vec<String> = Vec::new();
    let mut day_stats: Vec<Vec<(f64, f64, f64)>> = Vec::new(); // (rs, oi, ci)
    for b in &bars {
        match days.iter().position(|d| d == &b.day) {
            Some(i) => day_stats[i].push((
                b.ui * (b.ui - b.ci) + b.di * (b.di - b.ci),
                b.oi,
                b.ci,
            )),
            None => {
                days.push(b.day.clone());
                day_stats.push(vec![(
                    b.ui * (b.ui - b.ci) + b.di * (b.di - b.ci),
                    b.oi,
                    b.ci,
                )]);
            }
        }
    }

    let total_bars = bars.len() as f64;
    let day_count = days.len() as f64;
    if day_count < 1.0 {
        return Vec::new();
    }
    // akshare: n = len(data) / len(rs_var).
    let n = total_bars / day_count;
    let k = 0.34 / (1.34 + (n + 1.0) / (n - 1.0));

    let mut out = Vec::with_capacity(days.len());
    for (day, grp) in days.iter().zip(day_stats.iter()) {
        // Need at least two intraday bars for a sample variance.
        if grp.len() < 2 {
            continue;
        }
        let m = grp.len() as f64;
        let rs_mean = grp.iter().map(|(rs, _, _)| rs).sum::<f64>() / m;
        let oi_mean = grp.iter().map(|(_, oi, _)| oi).sum::<f64>() / m;
        let ci_mean = grp.iter().map(|(_, _, ci)| ci).sum::<f64>() / m;
        // Sample variance (ddof=1), matching pandas `.var()`.
        let vo = grp.iter().map(|(_, oi, _)| (oi - oi_mean).powi(2)).sum::<f64>() / (m - 1.0);
        let vc = grp.iter().map(|(_, _, ci)| (ci - ci_mean).powi(2)).sum::<f64>() / (m - 1.0);
        let rv = ((1.0 - k) * rs_mean + vo + k * vc).sqrt();
        out.push(YangZhangRvRow {
            date: day.clone(),
            rv,
        });
    }
    out
}

/// 东方财富-分钟行情 → Yang-Zhang 已实现波动率.
///
/// Fetches Eastmoney minute K-lines via [`stock_zh_a_hist_min_em`] (period
/// `1`/`5`/`15`/`30`/`60`, adjust `""`/`qfq`/`hfq`), drops zero-open bars, and
/// runs [`volatility_yz_rv`].
pub async fn rv_from_stock_zh_a_hist_min_em(
    client: &crate::core::client::Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
    period: &str,
    adjust: &str,
) -> Result<Vec<YangZhangRvRow>> {
    let mins = stock_zh_a_hist_min_em(client, symbol, start_date, end_date, period, adjust).await?;
    let ohlc: Vec<OhlcRow> = mins
        .into_iter()
        .filter(|r| r.open.unwrap_or(0.0) != 0.0)
        .map(|r| OhlcRow {
            date: r.time,
            open: r.open.unwrap_or(0.0),
            high: r.high.unwrap_or(0.0),
            low: r.low.unwrap_or(0.0),
            close: r.close.unwrap_or(0.0),
        })
        .collect();
    Ok(volatility_yz_rv(&ohlc))
}

/// 新浪期货-分钟行情 → Yang-Zhang 已实现波动率.
///
/// Fetches Sina minute K-lines via [`futures_zh_minute_sina`] (period
/// `1`/`5`/`15`/`30`/`60`), drops zero-open bars, and runs
/// [`volatility_yz_rv`].
pub async fn rv_from_futures_zh_minute_sina(
    client: &crate::core::client::Client,
    symbol: &str,
    period: &str,
) -> Result<Vec<YangZhangRvRow>> {
    let mins = futures_zh_minute_sina(client, symbol, period).await?;
    let ohlc: Vec<OhlcRow> = mins
        .into_iter()
        .filter(|r| r.open.unwrap_or(0.0) != 0.0)
        .map(|r| OhlcRow {
            date: r.datetime,
            open: r.open.unwrap_or(0.0),
            high: r.high.unwrap_or(0.0),
            low: r.low.unwrap_or(0.0),
            close: r.close.unwrap_or(0.0),
        })
        .collect();
    Ok(volatility_yz_rv(&ohlc))
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn ohlc(date: &str, o: f64, h: f64, l: f64, c: f64) -> OhlcRow {
        OhlcRow {
            date: date.to_string(),
            open: o,
            high: h,
            low: l,
            close: c,
        }
    }

    #[test]
    fn yz_rv_constant_prices_is_zero() {
        // A day with no intraday movement has ui=ci=di=0 and oi=0, so every
        // variance term is 0 and the daily estimate must be exactly 0.
        let rows = vec![
            ohlc("2024-01-02 09:31:00", 10.0, 10.0, 10.0, 10.0),
            ohlc("2024-01-02 09:32:00", 10.0, 10.0, 10.0, 10.0),
            ohlc("2024-01-02 09:33:00", 10.0, 10.0, 10.0, 10.0),
            ohlc("2024-01-03 09:31:00", 10.0, 10.0, 10.0, 10.0),
            ohlc("2024-01-03 09:32:00", 10.0, 10.0, 10.0, 10.0),
            ohlc("2024-01-03 09:33:00", 10.0, 10.0, 10.0, 10.0),
        ];
        let out = volatility_yz_rv(&rows);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].date, "2024-01-02");
        assert_eq!(out[1].date, "2024-01-03");
        for r in &out {
            assert!(r.rv.abs() < 1e-12, "expected 0, got {}", r.rv);
        }
    }

    #[test]
    fn yz_rv_moving_prices_is_positive() {
        let rows = vec![
            ohlc("2024-01-02 09:31:00", 10.0, 10.0, 10.0, 10.0),
            ohlc("2024-01-02 09:32:00", 10.0, 11.0, 10.0, 11.0),
            ohlc("2024-01-02 09:33:00", 11.0, 12.0, 11.0, 12.0),
            ohlc("2024-01-03 09:31:00", 12.0, 12.0, 12.0, 12.0),
            ohlc("2024-01-03 09:32:00", 12.0, 13.0, 12.0, 13.0),
            ohlc("2024-01-03 09:33:00", 13.0, 14.0, 13.0, 14.0),
        ];
        let out = volatility_yz_rv(&rows);
        assert_eq!(out.len(), 2);
        for r in &out {
            assert!(r.rv > 0.0 && r.rv.is_finite(), "rv={}", r.rv);
        }
    }

    #[test]
    fn yz_rv_single_bar_day_dropped() {
        // Day 1 has only one intraday bar after the leading drop → no variance.
        let rows = vec![
            ohlc("2024-01-02 09:31:00", 10.0, 10.0, 10.0, 10.0),
            ohlc("2024-01-02 09:32:00", 10.0, 11.0, 10.0, 11.0),
            ohlc("2024-01-03 09:31:00", 11.0, 12.0, 11.0, 12.0),
            ohlc("2024-01-03 09:32:00", 12.0, 13.0, 12.0, 13.0),
            ohlc("2024-01-03 09:33:00", 13.0, 14.0, 13.0, 14.0),
        ];
        let out = volatility_yz_rv(&rows);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].date, "2024-01-03");
    }

    #[test]
    fn yz_rv_too_few_rows_is_empty() {
        let rows = vec![ohlc("2024-01-02 09:31:00", 10.0, 10.0, 10.0, 10.0)];
        assert!(volatility_yz_rv(&rows).is_empty());
    }

    #[test]
    fn day_key_handles_separators() {
        assert_eq!(day_key("2024-01-02 09:31:00"), "2024-01-02");
        assert_eq!(day_key("2024/01/02"), "2024/01/02");
        assert_eq!(day_key("2024-01-03"), "2024-01-03");
    }
}
