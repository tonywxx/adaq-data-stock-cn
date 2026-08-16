//! Air-quality HTML-table ports that do NOT require JS execution.
//!
//! Implements the two `pd.read_html` scrapers from `akshare/air/air_zhenqi.py`
//! that hit the public 真气网 (zq12369.com) rank page:
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `air_city_table` | `air/air_zhenqi.py:64` | all-city AQI ranking (`[1]`) |
//! | `air_quality_rank` | `air/air_zhenqi.py:219` | 168-city AQI ranking (`实时`/`[0]`) |
//!
//! ## DEFERRED (no code below — see report)
//!
//! - `air_quality_hebei` (`air/air_hebei.py:23`) — `GET
//!   http://218.11.10.130:8080/api/hour/130000.xml` is an **XML** feed served
//!   from a bare IP that is currently unreachable (connection timeout). Needs an
//!   XML parser, not the `scraper` HTML crate.
//! - `air_quality_hist` / `air_quality_watch_point` (`air/air_zhenqi.py:142`,
//!   `:99`) — JS-signed payloads (`py_mini_racer` crypto/outcrypto.js).
//! - `sunrise_city_list` / `sunrise_daily` / `sunrise_monthly`
//!   (`air/sunrise_tad.py`) — timeanddate.com sits behind a Cloudflare
//!   JS challenge (`"Just a moment…"`), so the HTML is never delivered to a
//!   plain `reqwest` client.

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use scraper::{Html, Selector};

/// Source bucket for the 真气网 rank page.
const SOURCE_ZQ: &str = "zq12369";

/// One row of a 真气网 AQI ranking table.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AirQualityRankRow {
    /// Rank (akshare `序号`/`降序`). `air_city_table` renumbers by position;
    /// `air_quality_rank` keeps the site's `降序` value.
    pub rank: i64,
    /// Province (akshare `省份`).
    pub province: String,
    /// City (akshare `城市`).
    pub city: String,
    /// AQI (akshare `AQI`).
    pub aqi: Option<f64>,
    /// Air-quality verbal label (akshare `空气质量`).
    pub air_quality: String,
    /// PM2.5 concentration µg/m³ (akshare `PM2.5浓度`).
    pub pm25: Option<f64>,
    /// Primary pollutant (akshare `首要污染物`).
    pub primary_pollutant: String,
}

/// All-city AQI ranking (`air_city_table`, akshare `air/air_zhenqi.py:64`).
///
/// Fetches `environment.php?date=2020-05-01&tab=rank&order=DESC&type=DAY` and
/// parses the second AQI `<table>` (`pd.read_html[1]`), dropping the first data
/// row and renumbering `序号` by position.
pub async fn air_city_table(client: &Client) -> Result<Vec<AirQualityRankRow>> {
    let url = "https://www.zq12369.com/environment.php";
    let params: &[(&str, &str)] = &[
        ("date", "2020-05-01"),
        ("tab", "rank"),
        ("order", "DESC"),
        ("type", "DAY"),
    ];
    let html = client
        .get_text(SOURCE_ZQ, "air_city_table", url, params, None)
        .await?;
    parse_air_city_table(&html)
}

/// Parse `air_city_table` from captured HTML.
pub(crate) fn parse_air_city_table(html: &str) -> Result<Vec<AirQualityRankRow>> {
    // akshare reads `pd.read_html(...)[1]` — the second AQI table once the empty
    // table is skipped — and renumbers `序号` by row position.
    parse_air_aqi_table(html, 1, false)
}

/// 168-city AQI ranking (`air_quality_rank`, akshare `air/air_zhenqi.py:219`).
///
/// Defaults to the `实时` (live) branch: `environment.php?tab=rank&order=DESC&type=MONTH`,
/// parsing the first AQI `<table>` (`pd.read_html[0]`).
pub async fn air_quality_rank(client: &Client) -> Result<Vec<AirQualityRankRow>> {
    let url = "https://www.zq12369.com/environment.php";
    let params: &[(&str, &str)] = &[("tab", "rank"), ("order", "DESC"), ("type", "MONTH")];
    let html = client
        .get_text(SOURCE_ZQ, "air_quality_rank", url, params, None)
        .await?;
    parse_air_quality_rank(&html)
}

/// Parse `air_quality_rank` from captured HTML.
pub(crate) fn parse_air_quality_rank(html: &str) -> Result<Vec<AirQualityRankRow>> {
    // akshare reads `pd.read_html(...)[0]` for the `实时` branch and keeps the
    // site's `降序` column as the rank.
    parse_air_aqi_table(html, 0, true)
}

/// Shared parser for the 真气网 AQI ranking tables.
///
/// `table_index` counts only tables that carry a header row (>=2 `<th>` cells),
/// mirroring how `pd.read_html` skips the empty `<table>` on the page.
/// `use_descending_as_rank` selects the rank source: `air_city_table` renumbers
/// by remaining-row position; `air_quality_rank` keeps the `降序` cell value.
/// Both akshare variants drop the first data row (`iloc[1:, :]`).
fn parse_air_aqi_table(
    html: &str,
    table_index: usize,
    use_descending_as_rank: bool,
) -> Result<Vec<AirQualityRankRow>> {
    let fragment = Html::parse_document(html);
    let table_sel = Selector::parse("table").unwrap();
    let tr_sel = Selector::parse("tr").unwrap();
    let th_sel = Selector::parse("th").unwrap();
    let td_sel = Selector::parse("td").unwrap();

    // Candidate tables: those with a header row of >=2 <th> cells.
    let mut candidates = Vec::new();
    for table in fragment.select(&table_sel) {
        let has_header = table
            .select(&tr_sel)
            .any(|tr| tr.select(&th_sel).count() >= 2);
        if has_header {
            candidates.push(table);
        }
    }

    let table = candidates.get(table_index).ok_or_else(|| Error::Parse {
        endpoint: SOURCE_ZQ,
        message: format!("AQI ranking table index {table_index} not found"),
    })?;

    // Collect data rows (rows whose cells are all <td>, i.e. not the header).
    let mut data_rows: Vec<Vec<String>> = Vec::new();
    for tr in table.select(&tr_sel) {
        if tr.select(&th_sel).count() > 0 {
            continue; // header row
        }
        let cells: Vec<String> = tr
            .select(&td_sel)
            .map(|e| e.text().collect::<String>())
            .map(|s| s.trim().to_string())
            .collect();
        if !cells.is_empty() {
            data_rows.push(cells);
        }
    }

    // akshare drops the first data row (iloc[1:, :]).
    let skip = data_rows.len().min(1);
    let data_rows = &data_rows[skip..];

    let mut out = Vec::with_capacity(data_rows.len());
    for (i, cells) in data_rows.iter().enumerate() {
        let rank = if use_descending_as_rank {
            cells
                .first()
                .and_then(|c| c.trim().parse::<i64>().ok())
                .unwrap_or(0)
        } else {
            (i + 1) as i64
        };
        let province = cells.get(1).cloned().unwrap_or_default();
        let city = cells.get(2).cloned().unwrap_or_default();
        let aqi = cells.get(3).and_then(|c| parse_leading_f64(c));
        let air_quality = cells.get(4).cloned().unwrap_or_default();
        let pm25 = cells.get(5).and_then(|c| parse_leading_f64(c));
        let primary_pollutant = cells.get(6).cloned().unwrap_or_default();
        out.push(AirQualityRankRow {
            rank,
            province,
            city,
            aqi,
            air_quality,
            pm25,
            primary_pollutant,
        });
    }
    Ok(out)
}

/// Parse a leading numeric value from a cell that may carry units, e.g.
/// `"108 ug/m³"` -> `108.0`, `"54"` -> `54.0`, `"中度污染"` -> `None`.
fn parse_leading_f64(s: &str) -> Option<f64> {
    let s = s.trim();
    let end = s
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .unwrap_or(s.len());
    s[..end].parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> String {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(p).unwrap()
    }

    #[test]
    fn parses_air_city_table() {
        let rows = parse_air_city_table(&fixture("air_city_table.html")).unwrap();
        assert!(!rows.is_empty(), "expected >0 rows");
        assert_eq!(rows.len(), 167);
        // First data row after dropping site row 0; 序号 renumbered by position.
        assert_eq!(rows[0].rank, 1);
        assert_eq!(rows[0].province, "河北");
        assert_eq!(rows[0].city, "廊坊");
        assert_eq!(rows[0].aqi, Some(199.0));
        assert_eq!(rows[0].air_quality, "中度污染");
        assert_eq!(rows[0].pm25, Some(54.0));
        assert_eq!(rows[0].primary_pollutant, "O3");
    }

    #[test]
    fn parses_air_quality_rank() {
        let rows = parse_air_quality_rank(&fixture("air_quality_rank.html")).unwrap();
        assert!(!rows.is_empty(), "expected >0 rows");
        assert_eq!(rows.len(), 167);
        // 实时 branch keeps the site's 降序 column as the rank.
        assert_eq!(rows[0].rank, 2);
        assert_eq!(rows[0].province, "四川");
        assert_eq!(rows[0].city, "泸州");
        assert_eq!(rows[0].aqi, Some(86.0));
        assert_eq!(rows[0].air_quality, "良");
        assert_eq!(rows[0].pm25, Some(53.0));
        assert_eq!(rows[0].primary_pollutant, "PM2.5");
    }
}
