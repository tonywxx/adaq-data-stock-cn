//! Movie box-office data (akshare `movie/movie_yien.py`) — 艺恩 (endata) JSON.
//!
//! The 艺恩 public API is **POST** with a `Referer` header, served at
//! `ys.endata.cn/enlib-api/...`. It returns `data.table1` (rows for the current
//! page) and `data.table2[0].TotalPage` (total page count). We call it through
//! `Client::post_form_json` and follow the akshare pagination loop.
//!
//! Note: the older `decrypt`/`jm.js` JS path in the module is NOT used here —
//! the `*_list.do` endpoints return plain JSON and need no JS signing.

use serde_json::Value;

use crate::alt::{fint, fnum, fstr};
use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_ENDATA: &str = "endata";
const DAY_URL: &str = "https://ys.endata.cn/enlib-api/api/movie/getMovie_BoxOffice_Day_List.do";
const REFERRER: &str = "https://ys.endata.cn/BoxOffice/Movie";
const PAGE_SIZE: &str = "500";

const ENDATA_HEADERS: &[(&str, &str)] = &[
    ("Accept", "application/json, text/plain, */*"),
    ("Content-Type", "application/x-www-form-urlencoded"),
    ("Origin", "https://ys.endata.cn"),
    ("Referer", REFERRER),
    (
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
         Chrome/138.0.0.0 Safari/537.36",
    ),
];

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

/// `[month_start, month_end]` (ISO) for a `YYYY-MM-DD` date.
fn month_bounds(date_iso: &str) -> (String, String) {
    let year = &date_iso[..4];
    let month = &date_iso[5..7];
    let last = match month {
        "01" | "03" | "05" | "07" | "08" | "10" | "12" => 31,
        "04" | "06" | "09" | "11" => 30,
        "02" => {
            let y: u32 = year.parse().unwrap_or(2024);
            if (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400) {
                29
            } else {
                28
            }
        }
        _ => 30,
    };
    (
        format!("{year}-{month}-01"),
        format!("{year}-{month}-{last:02}"),
    )
}

/// 艺恩 month-id encoding used by the monthly API.
fn month_id(date_iso: &str) -> i64 {
    let year: i64 = date_iso[..4].parse().unwrap_or(2024);
    let month: i64 = date_iso[5..7].parse().unwrap_or(1);
    month + (year - 2026) * 12 + 240
}

// ---------------------------------------------------------------------------
// movie_boxoffice_daily
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct MovieBoxofficeDaily {
    /// Rank within the day.
    pub rank: Option<i64>,
    /// Movie name.
    pub movie_name: String,
    /// Single-day box office (万元; upstream `BoxOffice` / 10000).
    pub box_office: Option<f64>,
    /// Month-over-month change (%).
    pub box_office_mom: Option<f64>,
    /// Cumulative box office (万元; upstream `TotalBoxOffice` / 10000).
    pub total_box_office: Option<f64>,
    /// Average ticket price.
    pub avg_box_office: Option<f64>,
    /// Average audience per show.
    pub avg_show_audience_count: Option<f64>,
    /// Days since release.
    pub release_day: Option<i64>,
    pub source: &'static str,
}

/// Single-day box office (`movie_boxoffice_daily`, 艺恩 `getMovie_BoxOffice_Day_List.do`).
///
/// `date` is an 8-digit string, e.g. `"20240219"` (akshare fetches the day
/// before "today"; we mirror the upstream pagination loop).
pub async fn movie_boxoffice_daily(client: &Client, date: &str) -> Result<Vec<MovieBoxofficeDaily>> {
    let date_str = fmt_date(date);
    let mut out = Vec::new();
    let mut page: u32 = 1;
    loop {
        let page_s = page.to_string();
        let params: [(&str, &str); 11] = [
            ("r", "0.123456789"),
            ("datetype", "Day"),
            ("date", &date_str),
            ("sdate", &date_str),
            ("edate", &date_str),
            ("bserviceprice", "1"),
            ("columnslist", "100,102,103,146,105,111,113,112,119"),
            ("pageindex", &page_s),
            ("pagesize", PAGE_SIZE),
            ("order", "103"),
            ("ordertype", "desc"),
        ];
        let v = client
            .post_form_json(SOURCE_ENDATA, "movie_boxoffice_daily", DAY_URL, &params, Some(ENDATA_HEADERS))
            .await?;
        let total = total_pages(&v);
        out.extend(parse_movie_boxoffice_daily(&v)?);
        if page >= total {
            break;
        }
        page += 1;
    }
    Ok(out)
}

pub(crate) fn parse_movie_boxoffice_daily(resp: &Value) -> Result<Vec<MovieBoxofficeDaily>> {
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
        out.push(MovieBoxofficeDaily {
            rank: fint(item, "Irank"),
            movie_name: name,
            box_office: fnum(item, "BoxOffice").map(|x| x / 10000.0),
            box_office_mom: fnum(item, "BoxOfficeMoM"),
            total_box_office: fnum(item, "TotalBoxOffice").map(|x| x / 10000.0),
            avg_box_office: fnum(item, "AvgBoxOffice"),
            avg_show_audience_count: fnum(item, "AvgShowAudienceCount"),
            release_day: fint(item, "ReleaseDay"),
            source: SOURCE_ENDATA,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// movie_boxoffice_realtime
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct MovieBoxofficeRealtime {
    /// Rank within the day.
    pub rank: Option<i64>,
    /// Movie name.
    pub movie_name: String,
    /// Real-time box office (万元; upstream `BoxOffice` / 10000).
    pub box_office: Option<f64>,
    /// Box-office share (%).
    pub box_office_percent: Option<f64>,
    /// Days since release.
    pub release_day: Option<i64>,
    /// Cumulative box office (万元; upstream `TotalBoxOffice` / 10000).
    pub total_box_office: Option<f64>,
    pub source: &'static str,
}

/// Real-time box office (`movie_boxoffice_realtime`, 艺恩 `getMovie_BoxOffice_Day_List.do`).
///
/// `date` is an 8-digit string (akshare uses today's date; we require it for a
/// deterministic, replayable call).
pub async fn movie_boxoffice_realtime(client: &Client, date: &str) -> Result<Vec<MovieBoxofficeRealtime>> {
    let date_str = fmt_date(date);
    let mut out = Vec::new();
    let mut page: u32 = 1;
    loop {
        let page_s = page.to_string();
        let params: [(&str, &str); 11] = [
            ("r", "0.123456789"),
            ("datetype", "Day"),
            ("date", &date_str),
            ("sdate", &date_str),
            ("edate", &date_str),
            ("bserviceprice", "1"),
            (
                "columnslist",
                "100,102,103,119,105,107,109,106,112,129,142,143,163,164,165",
            ),
            ("pageindex", &page_s),
            ("pagesize", PAGE_SIZE),
            ("order", "103"),
            ("ordertype", "desc"),
        ];
        let v = client
            .post_form_json(SOURCE_ENDATA, "movie_boxoffice_realtime", DAY_URL, &params, Some(ENDATA_HEADERS))
            .await?;
        let total = total_pages(&v);
        out.extend(parse_movie_boxoffice_realtime(&v)?);
        if page >= total {
            break;
        }
        page += 1;
    }
    Ok(out)
}

pub(crate) fn parse_movie_boxoffice_realtime(resp: &Value) -> Result<Vec<MovieBoxofficeRealtime>> {
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
        out.push(MovieBoxofficeRealtime {
            rank: fint(item, "Irank"),
            movie_name: name,
            box_office: fnum(item, "BoxOffice").map(|x| x / 10000.0),
            box_office_percent: fnum(item, "BoxOfficePercent"),
            release_day: fint(item, "ReleaseDay"),
            total_box_office: fnum(item, "TotalBoxOffice").map(|x| x / 10000.0),
            source: SOURCE_ENDATA,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// movie_boxoffice_monthly
// ---------------------------------------------------------------------------

const MONTH_URL: &str =
    "https://ys.endata.cn/enlib-api/api/movie/getMovie_BoxOffice_Month_List.do";

#[derive(Debug, Clone, serde::Serialize)]
pub struct MovieBoxofficeMonthly {
    /// Rank within the month.
    pub rank: Option<i64>,
    /// Movie name.
    pub movie_name: String,
    /// Monthly box office (万元; upstream `BoxOffice` / 10000).
    pub box_office: Option<f64>,
    /// Share within the month (%).
    pub box_office_percent: Option<f64>,
    /// Average ticket price.
    pub avg_box_office: Option<f64>,
    /// Average audience per show.
    pub avg_show_audience_count: Option<f64>,
    /// Release date.
    pub release_date: Option<String>,
    /// Days within the month.
    pub release_day: Option<i64>,
    pub source: &'static str,
}

/// Monthly box office (`movie_boxoffice_monthly`, 艺恩 `getMovie_BoxOffice_Month_List.do`).
pub async fn movie_boxoffice_monthly(client: &Client, date: &str) -> Result<Vec<MovieBoxofficeMonthly>> {
    let date_iso = fmt_date(date);
    let (ms, me) = month_bounds(&date_iso);
    let mid = month_id(&date_iso).to_string();
    let range = format!("{ms},{me}");
    let mut out = Vec::new();
    let mut page: u32 = 1;
    loop {
        let page_s = page.to_string();
        let params: [(&str, &str); 14] = [
            ("r", "0.123456789"),
            ("datetype", "Month"),
            ("date", &range),
            ("sdate", &ms),
            ("edate", &me),
            ("dateid", &mid),
            ("sdateid", &mid),
            ("edateid", &mid),
            ("bserviceprice", "1"),
            ("columnslist", "100,101,102,105,109,110,130,131"),
            ("pageindex", &page_s),
            ("pagesize", PAGE_SIZE),
            ("order", "102"),
            ("ordertype", "desc"),
        ];
        let v = client
            .post_form_json(SOURCE_ENDATA, "movie_boxoffice_monthly", MONTH_URL, &params, Some(ENDATA_HEADERS))
            .await?;
        let total = total_pages(&v);
        out.extend(parse_movie_boxoffice_monthly(&v)?);
        if page >= total {
            break;
        }
        page += 1;
    }
    Ok(out)
}

pub(crate) fn parse_movie_boxoffice_monthly(resp: &Value) -> Result<Vec<MovieBoxofficeMonthly>> {
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
        out.push(MovieBoxofficeMonthly {
            rank: fint(item, "Irank"),
            movie_name: name,
            box_office: fnum(item, "BoxOffice").map(|x| x / 10000.0),
            box_office_percent: fnum(item, "BoxOfficePercent"),
            avg_box_office: fnum(item, "AvgBoxOffice"),
            avg_show_audience_count: fnum(item, "AvgShowAudienceCount"),
            release_date: fstr(item, "ReleaseDate"),
            release_day: fint(item, "ReleaseDay"),
            source: SOURCE_ENDATA,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// movie_boxoffice_yearly
// ---------------------------------------------------------------------------

const YEAR_URL: &str =
    "https://ys.endata.cn/enlib-api/api/movie/getMovie_BoxOffice_Year_List.do";

#[derive(Debug, Clone, serde::Serialize)]
pub struct MovieBoxofficeYearly {
    /// Rank within the year.
    pub rank: Option<i64>,
    /// Movie name.
    pub movie_name: String,
    /// Genre (main).
    pub genre: Option<String>,
    /// Total box office (万元; upstream `TotalBoxOffice` / 10000).
    pub total_box_office: Option<f64>,
    /// Average ticket price.
    pub avg_box_office: Option<f64>,
    /// Average audience per show.
    pub avg_show_audience_count: Option<f64>,
    /// Country / region.
    pub country: Option<String>,
    /// Release date.
    pub release_date: Option<String>,
    pub source: &'static str,
}

/// Yearly box office (`movie_boxoffice_yearly`, 艺恩 `getMovie_BoxOffice_Year_List.do`).
pub async fn movie_boxoffice_yearly(client: &Client, date: &str) -> Result<Vec<MovieBoxofficeYearly>> {
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
            ("columnslist", "100,101,108,115,105,106,109,107"),
            ("pageindex", &page_s),
            ("pagesize", PAGE_SIZE),
            ("order", "115"),
            ("ordertype", "desc"),
        ];
        let v = client
            .post_form_json(SOURCE_ENDATA, "movie_boxoffice_yearly", YEAR_URL, &params, Some(ENDATA_HEADERS))
            .await?;
        let total = total_pages(&v);
        out.extend(parse_movie_boxoffice_yearly(&v)?);
        if page >= total {
            break;
        }
        page += 1;
    }
    Ok(out)
}

pub(crate) fn parse_movie_boxoffice_yearly(resp: &Value) -> Result<Vec<MovieBoxofficeYearly>> {
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
        out.push(MovieBoxofficeYearly {
            rank: fint(item, "Irank"),
            movie_name: name,
            genre: fstr(item, "GenreMain"),
            total_box_office: fnum(item, "TotalBoxOffice").map(|x| x / 10000.0),
            avg_box_office: fnum(item, "AvgBoxOffice"),
            avg_show_audience_count: fnum(item, "AvgShowAudienceCount"),
            country: fstr(item, "Country").map(|c| c.replace(' ', "")),
            release_date: fstr(item, "ReleaseDate"),
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
    fn parses_movie_boxoffice_daily() {
        let rows = parse_movie_boxoffice_daily(&fixture("movie_boxoffice_daily.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].movie_name, "影片甲");
        assert_eq!(rows[0].rank, Some(1));
        assert_eq!(rows[0].box_office, Some(120.0));
        assert_eq!(rows[0].total_box_office, Some(5000.0));
        assert_eq!(rows[0].box_office_mom, Some(-5.2));
        assert_eq!(rows[1].release_day, Some(5));
    }

    #[test]
    fn parses_movie_boxoffice_realtime() {
        let rows = parse_movie_boxoffice_realtime(&fixture("movie_boxoffice_realtime.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].movie_name, "影片甲");
        assert_eq!(rows[0].box_office, Some(150.0));
        assert_eq!(rows[0].box_office_percent, Some(25.3));
        assert_eq!(rows[1].total_box_office, Some(3080.0));
    }

    #[test]
    fn rejects_non_ok_status() {
        let mut v = fixture("movie_boxoffice_daily.json");
        v["status"] = serde_json::json!(0);
        assert!(parse_movie_boxoffice_daily(&v).is_err());
    }

    #[test]
    fn parses_movie_boxoffice_monthly() {
        let rows = parse_movie_boxoffice_monthly(&fixture("movie_boxoffice_monthly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].movie_name, "影片甲");
        assert_eq!(rows[0].box_office, Some(320.0));
        assert_eq!(rows[0].box_office_percent, Some(18.4));
        assert_eq!(rows[0].release_day, Some(12));
        assert_eq!(rows[1].movie_name, "影片乙");
    }

    #[test]
    fn parses_movie_boxoffice_yearly() {
        let rows = parse_movie_boxoffice_yearly(&fixture("movie_boxoffice_yearly.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].movie_name, "影片甲");
        assert_eq!(rows[0].genre, Some("剧情".to_string()));
        assert_eq!(rows[0].total_box_office, Some(45000.0));
        assert_eq!(rows[0].country, Some("中国".to_string()));
        assert_eq!(rows[1].movie_name, "影片丙");
    }
}
