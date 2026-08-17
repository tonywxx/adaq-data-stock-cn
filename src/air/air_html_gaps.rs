//! Air-quality HTML/XML endpoints ported from `akshare/air/*`.
//!
//! All four upstreams are unreachable from the build sandbox (see `## Blocked`
//! below), so their parsers are written to follow akshare's logic but their
//! tests are `#[ignore]`d — no live fixture could be captured:
//!
//! * [`air_quality_hebei`] — `air/air_hebei.py:23` (XML feed at a bare IP, down)
//! * [`sunrise_city_list`] — `air/sunrise_tad.py:15` (timeanddate.com → 403)
//! * [`sunrise_daily`] — `air/sunrise_tad.py:40` (timeanddate.com → 403)
//! * [`sunrise_monthly`] — `air/sunrise_tad.py:73` (timeanddate.com → 403)

use crate::core::client::Client;
use crate::core::error::{Error, Result};

/// Extract every `<table>` as a list of rows-of-cells (mirrors `pd.read_html`).
fn extract_tables(html: &str, endpoint: &'static str) -> Result<Vec<Vec<Vec<String>>>> {
    crate::core::html::tables(html, endpoint)
}

// ---------------------------------------------------------------------------
// air_quality_hebei — XML feed (河北省空气质量预报), parsed without a XML lib.
// ---------------------------------------------------------------------------

/// One Hebei air-quality observation (a `<Pointer>` inside a `<City>`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct AirQualityHebei {
    /// City (akshare `城市`).
    pub city: String,
    /// Region (akshare `区域`).
    pub region: Option<String>,
    /// Monitoring station (akshare `监测点`).
    pub station: Option<String>,
    /// Observation timestamp (akshare `时间`).
    pub datetime: Option<String>,
    /// AQI (akshare `AQI`).
    pub aqi: Option<f64>,
    /// Air-quality level (akshare `空气质量等级`).
    pub level: Option<String>,
    /// Primary pollutant (akshare `首要污染物`).
    pub max_poll: Option<String>,
    /// Longitude (akshare `经度`).
    pub longitude: Option<f64>,
    /// Latitude (akshare `纬度`).
    pub latitude: Option<f64>,
    /// Pollutant name→value map (akshare `<poll>_Value`).
    pub poll_values: std::collections::HashMap<String, f64>,
}

/// 河北省空气质量预报信息发布系统-空气质量预报 (`air_quality_hebei`, akshare `air_hebei.py:23`).
pub async fn air_quality_hebei(client: &Client) -> Result<Vec<AirQualityHebei>> {
    let url = "http://218.11.10.130:8080/api/hour/130000.xml";
    let xml = client
        .get_text("hebei_air", "air_quality_hebei", url, &[], None)
        .await?;
    parse_air_quality_hebei(&xml, "air_quality_hebei")
}

/// Return every `<tag ...>...</tag>` (or self-closing `<tag .../>`) block in `s`.
fn tag_blocks<'a>(s: &'a str, tag: &str) -> Vec<&'a str> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut out = Vec::new();
    let mut start = 0;
    while let Some(o) = s[start..].find(&open) {
        let o = start + o;
        let rest = &s[o..];
        let close_pos = rest.find(&close);
        let self_pos = rest.find("/>");
        let end = match (close_pos, self_pos) {
            (Some(c), Some(sc)) if sc < c => o + sc + 2,
            (Some(c), _) => o + c + close.len(),
            (None, Some(sc)) => o + sc + 2,
            (None, None) => break,
        };
        out.push(&s[o..end]);
        start = end;
    }
    out
}

/// Extract `name="..."` from a tag-open fragment.
fn attr(s: &str, name: &str) -> Option<String> {
    let pat = format!("{name}=\"");
    let i = s.find(&pat)? + pat.len();
    let j = s[i..].find('"')?;
    Some(s[i..i + j].to_string())
}

/// Extract the text between `<tag>` and `</tag>`.
fn child_text(s: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let i = s.find(&open)? + open.len();
    let j = s[i..].find(&close)?;
    Some(s[i..i + j].trim().to_string())
}

/// Parse the Hebei XML feed with manual string scanning (no XML dependency).
pub(crate) fn parse_air_quality_hebei(xml: &str, endpoint: &'static str) -> Result<Vec<AirQualityHebei>> {
    let num = |s: &str| s.trim().parse::<f64>().ok();
    let mut out = Vec::new();
    for cb in tag_blocks(xml, "City") {
        let city = attr(cb, "Name").unwrap_or_default();
        for pb in tag_blocks(cb, "Pointer") {
            let mut polls = std::collections::HashMap::new();
            for ps in tag_blocks(pb, "Poll") {
                if let (Some(n), Some(v)) = (attr(ps, "Name"), attr(ps, "Value")) {
                    if let Some(f) = num(&v) {
                        polls.insert(n, f);
                    }
                }
            }
            out.push(AirQualityHebei {
                city: city.clone(),
                region: child_text(pb, "Region"),
                station: child_text(pb, "Name"),
                datetime: child_text(pb, "DataTime"),
                aqi: child_text(pb, "AQI").and_then(|s| num(&s)),
                level: child_text(pb, "Level"),
                max_poll: child_text(pb, "MaxPoll"),
                longitude: child_text(pb, "CLng").and_then(|s| num(&s)),
                latitude: child_text(pb, "CLat").and_then(|s| num(&s)),
                poll_values: polls,
            });
        }
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no City/Pointer in XML".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// sunrise_city_list
// ---------------------------------------------------------------------------

/// A city name (lowercased) available for sunrise/sunset queries.
pub type SunriseCity = String;

/// 查询日出与日落数据的城市列表 (`sunrise_city_list`, akshare `sunrise_tad.py:15`).
pub async fn sunrise_city_list(client: &Client) -> Result<Vec<SunriseCity>> {
    let url = "https://www.timeanddate.com/astronomy/china";
    let html = client
        .get_text("timeanddate", "sunrise_city_list", url, &[], None)
        .await?;
    parse_sunrise_city_list(&html, "sunrise_city_list")
}

pub(crate) fn parse_sunrise_city_list(html: &str, endpoint: &'static str) -> Result<Vec<SunriseCity>> {
    let tables = extract_tables(html, endpoint)?;
    if tables.len() < 3 {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: format!("expected >=3 tables, found {}", tables.len()),
        });
    }
    let mut cities = Vec::new();
    let one = &tables[1];
    for cells in one {
        for &idx in &[0usize, 3, 6] {
            if let Some(c) = cells.get(idx) {
                let c = c.trim().to_lowercase();
                if !c.is_empty() {
                    cities.push(c);
                }
            }
        }
    }
    let two = &tables[2];
    for cells in two {
        for idx in 0..=4 {
            if let Some(c) = cells.get(idx) {
                let c = c.trim().to_lowercase();
                if !c.is_empty() {
                    cities.push(c);
                }
            }
        }
    }
    if cities.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no cities".into() });
    }
    Ok(cities)
}

// ---------------------------------------------------------------------------
// sunrise_daily / sunrise_monthly
// ---------------------------------------------------------------------------

/// One daily sunrise/sunset observation (timeanddate.com `sun/china` table).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SunriseRow {
    /// Day-of-month (akshare index column).
    pub day: String,
    /// Sunrise time.
    pub sunrise: Option<String>,
    /// Sunset time.
    pub sunset: Option<String>,
    /// Day length.
    pub day_length: Option<String>,
    /// Solar noon.
    pub solar_noon: Option<String>,
}

/// 每日日出日落数据 (`sunrise_daily`, akshare `sunrise_tad.py:40`).
pub async fn sunrise_daily(client: &Client, date: &str, city: &str) -> Result<Vec<SunriseRow>> {
    let url = format!(
        "https://www.timeanddate.com/sun/china/{city}?month={}&year={}",
        &date[4..6],
        &date[0..4]
    );
    let html = client
        .get_text("timeanddate", "sunrise_daily", &url, &[], None)
        .await?;
    parse_sunrise_table(&html, "sunrise_daily", Some(&date[6..]))
}

/// 每月日出日落数据 (`sunrise_monthly`, akshare `sunrise_tad.py:73`).
pub async fn sunrise_monthly(client: &Client, date: &str, city: &str) -> Result<Vec<SunriseRow>> {
    let url = format!(
        "https://www.timeanddate.com/sun/china/{city}?month={}&year={}",
        &date[4..6],
        &date[0..4]
    );
    let html = client
        .get_text("timeanddate", "sunrise_monthly", &url, &[], None)
        .await?;
    parse_sunrise_table(&html, "sunrise_monthly", None)
}

/// Parse the `sun/china` table (akshare `pd.read_html(..., header=2)[1]`).
/// When `day` is `Some`, only the row whose day matches is returned.
pub(crate) fn parse_sunrise_table(
    html: &str,
    endpoint: &'static str,
    day: Option<&str>,
) -> Result<Vec<SunriseRow>> {
    let tables = extract_tables(html, endpoint)?;
    if tables.len() < 2 {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: format!("expected >=2 tables, found {}", tables.len()),
        });
    }
    let rows = &tables[1];
    let mut out = Vec::new();
    for cells in rows.iter().skip(1) {
        if cells.is_empty() {
            continue;
        }
        let d = cells[0].trim();
        if let Some(want) = day {
            if d.trim_start_matches('0') != want.trim_start_matches('0') {
                continue;
            }
        }
        out.push(SunriseRow {
            day: d.to_string(),
            sunrise: cells.get(1).cloned(),
            sunset: cells.get(2).cloned(),
            day_length: cells.get(3).cloned(),
            solar_noon: cells.get(4).cloned(),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no rows".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // All air upstreams are unreachable from the build sandbox:
    //   * air_quality_hebei → http://218.11.10.130 bare IP, connection refused
    //   * sunrise_* → timeanddate.com returns HTTP 403 (Cloudflare) from the
    //     build network. No live fixture could be captured, so tests are ignored.

    #[test]
    #[ignore = "upstream http://218.11.10.130:8080 unreachable from build sandbox"]
    fn parses_air_quality_hebei() {
        let _ = parse_air_quality_hebei("", "air_quality_hebei");
    }

    #[test]
    #[ignore = "upstream timeanddate.com unreachable (HTTP 403) from build sandbox"]
    fn parses_sunrise_city_list() {
        let _ = parse_sunrise_city_list("", "sunrise_city_list");
    }

    #[test]
    #[ignore = "upstream timeanddate.com unreachable (HTTP 403) from build sandbox"]
    fn parses_sunrise_daily() {
        let _ = parse_sunrise_table("", "sunrise_daily", Some("28"));
    }

    #[test]
    #[ignore = "upstream timeanddate.com unreachable (HTTP 403) from build sandbox"]
    fn parses_sunrise_monthly() {
        let _ = parse_sunrise_table("", "sunrise_monthly", None);
    }
}
