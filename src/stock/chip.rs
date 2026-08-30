//! 筹码分布 (CYQ / chipping distribution) — ports `chip_distribution` from the
//! `simonlin1212/a-stock-data` skill.
//!
//! Pure local computation, **no network**. Given a series of daily OHLC bars with
//! a turnover ratio, it reconstructs the distribution of held shares across a
//! price grid using a triangular weight per day and a turnover-decay recurrence:
//!
//! ```text
//! chips = chips * (1 - turnover) + day_weight * turnover
//! ```
//!
//! The first valid day seeds the grid with the full float (the skill's "must
//! seed from day-1 holdings, not from zero" correction). `turn` is a percentage
//! (e.g. `0.31` means 0.31%); `decay` scales turnover (THS uses ~1.5–2.0 to
//! make old chips fade faster).
//!
//! Input bars MUST be sorted ascending by date and use **forward-adjusted**
//! (前复权) prices, or the cost basis will be wrong across ex-rights days.

use crate::core::error::{Error, Result};

/// One daily bar fed into [`chip_distribution`].
#[derive(Debug, Clone)]
pub struct ChipBar {
    /// `YYYY-MM-DD` — used only to enforce ascending time order.
    pub date: String,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    /// Turnover ratio as a **percent** (0.31 = 0.31%).
    pub turn: f64,
}

/// Result of [`chip_distribution`].
#[derive(Debug, Clone)]
pub struct ChipDistribution {
    /// Latest close (current price).
    pub price: f64,
    /// Share of holdings below the current price ∈ [0, 1] (获利比例).
    pub profit_ratio: f64,
    /// Volume-weighted average holding cost.
    pub avg_cost: f64,
    /// 5%–95% cost quantile prices (`cost_90`).
    pub cost_90: (f64, f64),
    /// 15%–85% cost quantile prices (`cost_70`).
    pub cost_70: (f64, f64),
    /// `(high - low) / (high + low)` for the 90% band; `None` if undefined.
    pub concentration_90: Option<f64>,
    /// `(high - low) / (high + low)` for the 70% band; `None` if undefined.
    pub concentration_70: Option<f64>,
    /// Most dense price level (筹码峰).
    pub peak_price: f64,
    /// `(price, density)` histogram; only bins above `1e-6` are kept.
    pub histogram: Vec<(f64, f64)>,
}

/// Port of `chip_distribution(bars, grid_size, decay)`.
///
/// `grid_size` is the number of price bins (default 300); `decay` is the turnover
/// decay multiplier (1.0 = literal turnover).
pub fn chip_distribution(
    bars: &[ChipBar],
    grid_size: usize,
    decay: f64,
) -> Result<ChipDistribution> {
    if bars.is_empty() {
        return Err(Error::InvalidParam(
            "chip_distribution: empty bars".into(),
        ));
    }
    if grid_size < 2 {
        return Err(Error::InvalidParam(
            "chip_distribution: grid_size must be >= 2".into(),
        ));
    }

    // Enforce ascending date order (turnover decay is directional).
    let mut sorted: Vec<&ChipBar> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));

    // Price range across all valid bars.
    let mut lo = f64::MAX;
    let mut hi = f64::MIN;
    for b in &sorted {
        if b.low > 0.0 {
            lo = lo.min(b.low);
        }
        if b.high > 0.0 {
            hi = hi.max(b.high);
        }
    }
    if lo == f64::MAX || hi == f64::MIN || hi <= lo {
        return Err(Error::InvalidParam(
            "chip_distribution: no valid price range (check high/low)".into(),
        ));
    }
    // 2% padding on each side (or a sane floor) so extreme days still land in-grid.
    let pad = if (hi - lo) * 0.02 > 0.0 {
        (hi - lo) * 0.02
    } else {
        (lo * 0.02).max(0.01)
    };
    let grid = linspace(lo - pad, hi + pad, grid_size);

    let mut chips: Option<Vec<f64>> = None;
    for b in &sorted {
        if b.high <= 0.0 || b.low <= 0.0 || b.close <= 0.0 {
            continue;
        }
        let t = (b.turn / 100.0 * decay).clamp(0.0, 1.0);
        let avg = (b.high + b.low + b.close) / 3.0;
        let w = triangular_weights(&grid, b.low, b.high, avg);
        let sum: f64 = w.iter().sum();
        if sum <= 0.0 {
            continue;
        }
        match chips {
            None => chips = Some(w),
            Some(ref mut c) => {
                for (i, ci) in c.iter_mut().enumerate() {
                    *ci = *ci * (1.0 - t) + w[i] * t;
                }
            }
        }
    }
    let mut chips = chips.ok_or_else(|| {
        Error::InvalidParam("chip_distribution: all price intervals invalid".into())
    })?;
    let total: f64 = chips.iter().sum();
    if total <= 0.0 {
        return Err(Error::InvalidParam(
            "chip_distribution: zero total chips".into(),
        ));
    }
    for c in chips.iter_mut() {
        *c /= total;
    }

    let price = sorted.last().unwrap().close;
    // Cumulative distribution for quantile lookup.
    let mut cum = vec![0.0f64; grid.len()];
    let mut acc = 0.0;
    for i in 0..grid.len() {
        acc += chips[i];
        cum[i] = acc;
    }
    let price_at = |q: f64| interp(&cum, &grid, q);

    let p05 = price_at(0.05);
    let p15 = price_at(0.15);
    let p85 = price_at(0.85);
    let p95 = price_at(0.95);

    // Peak (most dense bin).
    let mut peak_i = 0;
    let mut peak_v = 0.0;
    for (i, &cv) in chips.iter().enumerate() {
        if cv > peak_v {
            peak_v = cv;
            peak_i = i;
        }
    }

    let profit: f64 = grid
        .iter()
        .zip(chips.iter())
        .map(|(g, c)| if *g <= price { *c } else { 0.0 })
        .sum();
    let avg_cost: f64 = grid.iter().zip(chips.iter()).map(|(g, c)| g * c).sum();

    let concentration_90 = if (p95 + p05).abs() > 0.0 {
        Some((p95 - p05) / (p95 + p05))
    } else {
        None
    };
    let concentration_70 = if (p85 + p15).abs() > 0.0 {
        Some((p85 - p15) / (p85 + p15))
    } else {
        None
    };

    let histogram = grid
        .iter()
        .zip(chips.iter())
        .filter(|(_, c)| **c > 1e-6)
        .map(|(g, c)| (*g, *c))
        .collect();

    Ok(ChipDistribution {
        price,
        profit_ratio: profit,
        avg_cost,
        cost_90: (p05, p95),
        cost_70: (p15, p85),
        concentration_90,
        concentration_70,
        peak_price: grid[peak_i],
        histogram,
    })
}

/// Triangular weight of one day's chips across the price grid (peak at the
/// average price, area-normalized). Mirrors the skill's `_triangular_weights`.
fn triangular_weights(grid: &[f64], low: f64, high: f64, avg: f64) -> Vec<f64> {
    let n = grid.len();
    let mut w = vec![0.0f64; n];
    if !low.is_finite() || !high.is_finite() || !avg.is_finite() || high < low {
        return w;
    }
    if (high - low).abs() < 1e-9 {
        // One-price day (limit up/down): pile everything on the nearest bin.
        let mut best = 0;
        let mut best_d = f64::MAX;
        for (i, &g) in grid.iter().enumerate() {
            let d = (g - low).abs();
            if d < best_d {
                best_d = d;
                best = i;
            }
        }
        w[best] = 1.0;
        return w;
    }
    let avg = avg.clamp(low, high);
    for i in 0..n {
        let g = grid[i];
        if g >= low && g <= avg {
            w[i] = if (avg - low).abs() > 1e-9 {
                (g - low) / (avg - low)
            } else {
                1.0
            };
        } else if g > avg && g <= high {
            w[i] = if (high - avg).abs() > 1e-9 {
                (high - g) / (high - avg)
            } else {
                1.0
            };
        }
    }
    let total: f64 = w.iter().sum();
    if total > 0.0 {
        for x in w.iter_mut() {
            *x /= total;
        }
        return w;
    }
    // Fallback: if the interval is narrower than the grid step, snap to avg.
    let mut best = 0;
    let mut best_d = f64::MAX;
    for (i, &g) in grid.iter().enumerate() {
        let d = (g - avg).abs();
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    w[best] = 1.0;
    w
}

/// Linearly spaced grid from `start` to `end` (inclusive) of length `n`.
fn linspace(start: f64, end: f64, n: usize) -> Vec<f64> {
    if n == 1 {
        return vec![start];
    }
    let step = (end - start) / (n as f64 - 1.0);
    (0..n).map(|i| start + step * i as f64).collect()
}

/// `numpy.interp(x, xp, fp)` clone: `xp` strictly increasing, `fp` the values.
fn interp(xp: &[f64], fp: &[f64], x: f64) -> f64 {
    if xp.is_empty() || fp.is_empty() {
        return 0.0;
    }
    if x <= xp[0] {
        return fp[0];
    }
    let last = xp.len() - 1;
    if x >= xp[last] {
        return fp[last];
    }
    for i in 1..xp.len() {
        if x <= xp[i] {
            let t = (x - xp[i - 1]) / (xp[i] - xp[i - 1]);
            return fp[i - 1] + t * (fp[i] - fp[i - 1]);
        }
    }
    fp[last]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tiny synthetic series: a stock drifting 10→20 with light turnover.
    fn sample() -> Vec<ChipBar> {
        (0..20)
            .map(|i| {
                let p = 10.0 + i as f64;
                ChipBar {
                    date: format!("2024-01-{:02}", i + 1),
                    high: p + 0.5,
                    low: p - 0.5,
                    close: p,
                    turn: 2.0,
                }
            })
            .collect()
    }

    #[test]
    fn computes_distribution_and_invariants() {
        let r = chip_distribution(&sample(), 300, 1.0).unwrap();
        // Current price = last close = 29.
        assert!((r.price - 29.0).abs() < 1e-9);
        // Profit ratio must be in [0,1].
        assert!(r.profit_ratio >= 0.0 && r.profit_ratio <= 1.0);
        // avg_cost falls between grid extremes (sanity, not equality).
        assert!(r.avg_cost > 0.0 && r.avg_cost.is_finite());
        // cost_90 must bracket cost_70.
        assert!(r.cost_90.0 <= r.cost_90.1);
        assert!(r.cost_70.0 >= r.cost_90.0 && r.cost_70.1 <= r.cost_90.1);
        // Concentration 90 >= concentration 70 (wider band is less concentrated).
        if let (Some(c90), Some(c70)) = (r.concentration_90, r.concentration_70) {
            assert!(c90 >= c70 - 1e-9);
        }
        assert!(r.peak_price.is_finite());
        assert!(!r.histogram.is_empty());
    }

    #[test]
    fn rejects_empty_and_bad_grid() {
        assert!(chip_distribution(&[], 300, 1.0).is_err());
        assert!(chip_distribution(&sample(), 1, 1.0).is_err());
    }

    #[test]
    fn linspace_basic() {
        let g = linspace(0.0, 10.0, 11);
        assert_eq!(g.len(), 11);
        assert_eq!(g[0], 0.0);
        assert_eq!(g[10], 10.0);
    }
}
