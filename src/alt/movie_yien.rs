//! 艺恩 (Yien / endata) movie box-office — supplementary functions
//! (`akshare/movie/movie_yien.py`).
//!
//! The 艺恩 public API is **POST** with a `Referer` header at
//! `ys.endata.cn/enlib-api/...`, returning `data.table1` (rows for the current
//! page) plus `data.table2[0].TotalPage` (page count). We call it through
//! `Client::post_form_json` and follow the akshare pagination loop. The older
//! `decrypt`/`jm.js` JS-signing path is NOT used here — the `*_list.do`
//! endpoints return plain JSON.
//!
//! ## Function → source line
//!
//! | Rust fn | akshare line | status |
//! | --- | --- | --- |
//! | `get_current_week` | 50 | implemented (pure date helper) |
//! | `decrypt` | 65 | **DEFERRED** (see below) |
//! | `movie_boxoffice_realtime` | 207 | already ported in `src/alt/movie.rs` (skip) |
//! | `movie_boxoffice_daily` | 263 | already ported in `src/alt/movie.rs` (skip) |
//! | `movie_boxoffice_weekly` | 340 | **DEFERRED** (see below) |
//! | `movie_boxoffice_monthly` | 353 | already ported in `src/alt/movie.rs` (skip) |
//! | `movie_boxoffice_yearly` | 437 | already ported in `src/alt/movie.rs` (skip) |
//! | `movie_boxoffice_yearly_first_week` | 502 | implemented |
//! | `movie_boxoffice_cinema_daily` | 581 | implemented |
//! | `movie_boxoffice_cinema_weekly` | 642 | **DEFERRED** (see below) |
//!
//! ## DEFERRED
//!
//! * `decrypt` (line 65) — requires the `py_mini_racer` JS engine to run the
//!   bundled `jm.js` decryptor (`webInstace.shell`). Not a clean JSON endpoint.
//! * `movie_boxoffice_weekly` (line 340) — the upstream 艺恩 public weekly
//!   endpoint currently raises a permission error (needs auth); akshare itself
//!   raises `APIError` for it, so nothing deterministic can be fetched.
//! * `movie_boxoffice_cinema_weekly` (line 642) — same upstream permission error
//!   as the weekly box-office endpoint.

use chrono::{Datelike, Days, NaiveDate};
use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_ENDATA: &str = "endata";
const YEAR_URL: &str = "https://ys.endata.cn/enlib-api/api/movie/getMovie_BoxOffice_Year_List.do";
const CINEMA_URL: &str = "https://ys.endata.cn/enlib-api/api/cinema/getcinemaboxoffice_day_list.do";
const MOVIE_REFERRER: &str = "https://ys.endata.cn/BoxOffice/Movie";
const CINEMA_REFERRER: &str = "https://ys.endata.cn/BoxOffice/Org";
const PAGE_SIZE: &str = "500";
const CINEMA_PAGE_SIZE: &str = "100";

const ENDATA_HEADERS: &[(&str, &str)] = &[
    ("Accept", "application/json, text/plain, */*"),
    ("Content-Type", "application/x-www-form-urlencoded"),
    ("Origin", "https://ys.endata.cn"),
    ("Referer", MOVIE_REFERRER),
    (
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
         Chrome/138.0.0.0 Safari/537.36",
    ),
];

const CINEMA_HEADERS: &[(&str, &str)] = &[
    ("Accept", "application/json, text/plain, */*"),
    ("Content-Type", "application/x-www-form-urlencoded"),
    ("Origin", "https://ys.endata.cn"),
    ("Referer", CINEMA_REFERRER),
    (
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
         Chrome/138.0.0.0 Safari/537.36",
    ),
];

/// Extract a string field, if present.
fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Extract a numeric field, accepting either a JSON number or a numeric string.
fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// Extract an integer field (some upstreams encode ints as strings).
fn fint(item: &Value, k: &str) -> Option<i64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    })
}

/// `YYYYMMDD` -> `YYYY-MM-DD` (akshare posts dates in ISO form).
fn fmt_date(date: &str) -> String {
    if date.len() == 8 {
        format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..8])
    } else {
        date.to_string()
    }
}

/// Total page count advertised by the endata response.
fn total_pages(resp: &Value) -> u32 {
    resp.get("data")
        .and_then(|d| d.get("table2"))
        .and_then(|t| t.as_array())
        .and_then(|a| a.first())
        .and_then(|o| o.get("TotalPage"))
        .and_then(|n| n.as_u64())
        .unwrap_or(1) as u32
}

/// Number of days in the first week after release (akshare `_calc_first_week_days`):
/// `7 - weekday(release_date)`, where Monday = 0.
fn first_week_days(release_date: &str) -> Option<i64> {
    let d = NaiveDate::parse_from_str(release_date, "%Y-%m-%d").ok()?;
    Some(7 - d.weekday().num_days_from_monday() as i64)
}

// ---------------------------------------------------------------------------
// get_current_week
// ---------------------------------------------------------------------------

/// Monday of the week containing `date` (akshare `get_current_week`, line 50).
///
/// Pure helper: `date` is an 8-digit `YYYYMMDD` string; returns the Monday of
/// that week in `YYYY-MM-DD` form. Returns `None` if `date` is not parseable.
pub fn get_current_week(date: &str) -> Option<String> {
    let d = NaiveDate::parse_from_str(date, "%Y%m%d").ok()?;
    let monday = d - Days::new(d.weekday().num_days_from_monday() as u64);
    Some(monday.format("%Y-%m-%d").to_string())
}

// ---------------------------------------------------------------------------
// movie_boxoffice_yearly_first_week
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct MovieBoxofficeYearlyFirstWeek {
    /// Rank within the year.
    pub rank: Option<i64>,
    /// Movie name.
    pub movie_name: String,
    /// Genre (main).
    pub genre: Option<String>,
    /// First-week box office (万元; upstream `WeekBoxOffice` / 10000).
    pub week_box_office: Option<f64>,
    /// Share of total box office (%).
    pub week_box_percent: Option<f64>,
    /// Average audience per show.
    pub avg_show_audience_count: Option<f64>,
    /// Country / region.
    pub country: Option<String>,
    /// Release date.
    pub release_date: Option<String>,
    /// Days in the first week after release.
    pub first_week_days: Option<i64>,
    pub source: &'static str,
}

/// Yearly first-week box office (`movie_boxoffice_yearly_first_week`, 艺恩
/// `getMovie_BoxOffice_Year_List.do`, akshare line 502).
///
/// `date` is an 8-digit string, e.g. `"20201018"` (the year is taken from it).
pub async fn movie_boxoffice_yearly_first_week(
    client: &Client,
    date: &str,
) -> Result<Vec<MovieBoxofficeYearlyFirstWeek>> {
    let year = &date[..4];
    let start = format!("{year}-01-01");
    let end = format!("{year}-12-31");
    let range = format!("{start},{end}");
    let mut out = Vec::new();
    let mut page: u32 = 1;
    loop {
        let page_s = page.to_string();
        let params: [(&str, &str); 14] = [
            ("r", "0.123456789"),
            ("datetype", "Year"),
            ("date", &range),
            ("sdate", &start),
            ("edate", &end),
            ("dateid", year),
            ("sdateid", year),
            ("edateid", year),
            ("bserviceprice", "1"),
            ("columnslist", "100,101,108,118,119,106,109,107"),
            ("pageindex", &page_s),
            ("pagesize", PAGE_SIZE),
            ("order", "118"),
            ("ordertype", "desc"),
        ];
        let v = client
            .post_form_json(
                SOURCE_ENDATA,
                "movie_boxoffice_yearly_first_week",
                YEAR_URL,
                &params,
                Some(ENDATA_HEADERS),
            )
            .await?;
        let total = total_pages(&v);
        out.extend(parse_movie_boxoffice_yearly_first_week(&v)?);
        if page >= total {
            break;
        }
        page += 1;
    }
    Ok(out)
}

pub fn parse_movie_boxoffice_yearly_first_week(
    resp: &Value,
) -> Result<Vec<MovieBoxofficeYearlyFirstWeek>> {
    if resp.get("status").and_then(|s| s.as_i64()) != Some(1) {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_ENDATA,
            message: "endata status != 1".into(),
        });
    }
    let data = resp
        .get("data")
        .and_then(|d| d.get("table1"))
        .and_then(|t| t.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_ENDATA,
            message: "missing data.table1".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(name) = fstr(item, "MovieName") else {
            continue;
        };
        let release_date = fstr(item, "ReleaseDate");
        out.push(MovieBoxofficeYearlyFirstWeek {
            rank: fint(item, "Irank"),
            movie_name: name,
            genre: fstr(item, "GenreMain"),
            week_box_office: fnum(item, "WeekBoxOffice").map(|x| x / 10000.0),
            week_box_percent: fnum(item, "WeekBoxPercent"),
            avg_show_audience_count: fnum(item, "AvgShowAudienceCount"),
            country: fstr(item, "Country").map(|c| c.replace(' ', "")),
            release_date: release_date.clone(),
            first_week_days: release_date.as_deref().and_then(first_week_days),
            source: SOURCE_ENDATA,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// movie_boxoffice_cinema_daily
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct MovieBoxofficeCinemaDaily {
    /// Rank within the day.
    pub rank: Option<i64>,
    /// Cinema name.
    pub cinema_name: String,
    /// Single-day box office (upstream `BoxOffice`, raw — not divided by 10000).
    pub box_office: Option<f64>,
    /// Number of shows.
    pub show_count: Option<i64>,
    /// Average audience per show.
    pub avg_show_audience_count: Option<f64>,
    /// Average ticket price.
    pub avg_box_office: Option<f64>,
    /// Attendance rate (%).
    pub attendance: Option<f64>,
    pub source: &'static str,
}

/// Cinema daily box office (`movie_boxoffice_cinema_daily`, 艺恩
/// `getcinemaboxoffice_day_list.do`, akshare line 581). Single-page response
/// (no upstream pagination).
///
/// `date` is an 8-digit string, e.g. `"20240219"`.
pub async fn movie_boxoffice_cinema_daily(
    client: &Client,
    date: &str,
) -> Result<Vec<MovieBoxofficeCinemaDaily>> {
    let date_str = fmt_date(date);
    let params: [(&str, &str); 13] = [
        ("r", "0.123456789"),
        ("bserviceprice", "0"),
        ("datetype", "Day"),
        ("date", &date_str),
        ("sdate", &date_str),
        ("edate", &date_str),
        ("citylevel", ""),
        ("lineid", ""),
        ("columnslist", "100,101,102,103,109,108,117"),
        ("pageindex", "1"),
        ("pagesize", CINEMA_PAGE_SIZE),
        ("order", "102"),
        ("ordertype", "desc"),
    ];
    let v = client
        .post_form_json(
            SOURCE_ENDATA,
            "movie_boxoffice_cinema_daily",
            CINEMA_URL,
            &params,
            Some(CINEMA_HEADERS),
        )
        .await?;
    parse_movie_boxoffice_cinema_daily(&v)
}

pub fn parse_movie_boxoffice_cinema_daily(resp: &Value) -> Result<Vec<MovieBoxofficeCinemaDaily>> {
    if resp.get("status").and_then(|s| s.as_i64()) != Some(1) {
        return Err(Error::UpstreamChanged {
            origin: SOURCE_ENDATA,
            message: "endata status != 1".into(),
        });
    }
    let data = resp
        .get("data")
        .and_then(|d| d.get("table1"))
        .and_then(|t| t.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_ENDATA,
            message: "missing data.table1".into(),
        })?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(name) = fstr(item, "CinemaName") else {
            continue;
        };
        out.push(MovieBoxofficeCinemaDaily {
            rank: fint(item, "Irank"),
            cinema_name: name,
            box_office: fnum(item, "BoxOffice"),
            show_count: fint(item, "ShowCount"),
            avg_show_audience_count: fnum(item, "AvgShowAudienceCount"),
            avg_box_office: fnum(item, "AvgBoxOffice"),
            attendance: fnum(item, "Attendance"),
            source: SOURCE_ENDATA,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(p).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn current_week_monday() {
        // 2020-10-21 is a Wednesday; its week's Monday is 2020-10-19.
        assert_eq!(get_current_week("20201021"), Some("2020-10-19".to_string()));
        // 2020-10-19 itself is a Monday.
        assert_eq!(get_current_week("20201019"), Some("2020-10-19".to_string()));
    }

    #[test]
    fn parses_yearly_first_week() {
        let rows = parse_movie_boxoffice_yearly_first_week(&fixture(
            "movie_boxoffice_yearly_first_week.json",
        ))
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].movie_name, "影片甲");
        assert_eq!(rows[0].rank, Some(1));
        assert_eq!(rows[0].week_box_office, Some(12.3456));
        assert_eq!(rows[0].week_box_percent, Some(12.3));
        // 2020-01-01 is Wednesday -> 7 - 2 = 5 first-week days.
        assert_eq!(rows[0].first_week_days, Some(5));
        assert_eq!(rows[0].country, Some("中国".to_string()));
        // 2020-02-15 is Saturday -> 7 - 5 = 2 first-week days.
        assert_eq!(rows[1].first_week_days, Some(2));
    }

    #[test]
    fn rejects_yearly_first_week_bad_status() {
        let mut v = fixture("movie_boxoffice_yearly_first_week.json");
        v["status"] = serde_json::json!(0);
        assert!(parse_movie_boxoffice_yearly_first_week(&v).is_err());
    }

    #[test]
    fn parses_cinema_daily() {
        let rows =
            parse_movie_boxoffice_cinema_daily(&fixture("movie_boxoffice_cinema_daily.json"))
                .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].cinema_name, "影院甲");
        assert_eq!(rows[0].rank, Some(1));
        assert_eq!(rows[0].box_office, Some(98765.0));
        assert_eq!(rows[0].show_count, Some(20));
        assert_eq!(rows[0].attendance, Some(12.3));
        assert_eq!(rows[1].cinema_name, "影院乙");
    }
}
