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
use crate::core::json::*;

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
        let Some(name) = opt_str(item, "MovieName") else {
            continue;
        };
        let release_date = opt_str(item, "ReleaseDate");
        out.push(MovieBoxofficeYearlyFirstWeek {
            rank: fint(item, "Irank"),
            movie_name: name,
            genre: opt_str(item, "GenreMain"),
            week_box_office: opt_f64(item, "WeekBoxOffice").map(|x| x / 10000.0),
            week_box_percent: opt_f64(item, "WeekBoxPercent"),
            avg_show_audience_count: opt_f64(item, "AvgShowAudienceCount"),
            country: opt_str(item, "Country").map(|c| c.replace(' ', "")),
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
        let Some(name) = opt_str(item, "CinemaName") else {
            continue;
        };
        out.push(MovieBoxofficeCinemaDaily {
            rank: fint(item, "Irank"),
            cinema_name: name,
            box_office: opt_f64(item, "BoxOffice"),
            show_count: fint(item, "ShowCount"),
            avg_show_audience_count: opt_f64(item, "AvgShowAudienceCount"),
            avg_box_office: opt_f64(item, "AvgBoxOffice"),
            attendance: opt_f64(item, "Attendance"),
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

// ===========================================================================
// 艺恩 GetData.ashx — 艺人 / 视频 (jm.js 加密) 缺口函数
// (`akshare/movie/artist_yien.py`, `akshare/movie/video_yien.py`)
// ===========================================================================
//
// 这些端点 POST 到 `https://www.endata.com.cn/API/GetData.ashx`，返回用打包的
// `jm.js` (`webInstace.shell`) 加密的密文。解密需要执行 JavaScript（akshare 用
// `py_mini_racer`），Rust 端没有等价实现，且 client 也没有「POST 返回文本」的方法。
// 因此运行期 fetch 暂未实现（见各 async fn 说明）；解析函数 `parse_*` 已完整实现，
// 并针对「预先解密」的 fixture（`tests/fixtures/<name>.json`）做了测试。
//
// | Rust fn | akshare line | status |
// | --- | --- | --- |
// | `business_value_artist` | artist_yien.py:65 | parser 实现 / 运行期 deferred |
// | `online_value_artist` | artist_yien.py:103 | parser 实现 / 运行期 deferred |
// | `video_tv` | video_yien.py:65 | parser 实现 / 运行期 deferred |
// | `video_variety_show` | video_yien.py:96 | parser 实现 / 运行期 deferred |
// | `movie_boxoffice_weekly` | movie_yien.py:340 | deferred（上游需权限）|
// | `movie_boxoffice_cinema_weekly` | movie_yien.py:642 | deferred（上游需权限）|

const SOURCE_YIEN: &str = "yien";
#[allow(dead_code)]
const YIEN_GETDATA: &str = "https://www.endata.com.cn/API/GetData.ashx";

/// 从 JSON 标量解析 f64（容忍字符串数字）。
fn yien_as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// 取对象按 JSON 顺序的值列表（与 akshare 按位置取列一致）。
fn yien_row_values(item: &Value) -> Vec<&Value> {
    item.as_object().map(|m| m.values().collect()).unwrap_or_default()
}

/// 按位置取字符串（缺失返回 None）。
fn yien_val_str(vals: &[&Value], idx: usize) -> Option<String> {
    vals.get(idx).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// 按位置取 f64（缺失返回 None）。
fn yien_val_f64(vals: &[&Value], idx: usize) -> Option<f64> {
    vals.get(idx).and_then(|v| yien_as_f64(v))
}

/// 取 `Data.Table` 数组；缺失即上游结构变化。
fn yien_table(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("Data")
        .and_then(|d| d.get("Table"))
        .and_then(|t| t.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_YIEN,
            message: "missing Data.Table".into(),
        })
}

// ---------------------------------------------------------------------------
// business_value_artist — 艺人商业价值
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct BusinessValueArtist {
    /// 排名（akshare `排名`）。
    pub rank: Option<f64>,
    /// 艺人名（akshare `艺人`）。
    pub artist: String,
    /// 商业价值（akshare `商业价值`）。
    pub business_value: Option<f64>,
    /// 专业热度（akshare `专业热度`）。
    pub major_heat: Option<f64>,
    /// 关注热度（akshare `关注热度`）。
    pub attention_heat: Option<f64>,
    /// 预测热度（akshare `预测热度`）。
    pub forecast_heat: Option<f64>,
    /// 美誉度（akshare `美誉度`）。
    pub reputation: Option<f64>,
}

/// 艺人商业价值（`business_value_artist`, 艺恩 `GetData.ashx`
/// `Data_GetList_Star` / `BusinessValueIndex_L1`, akshare `artist_yien.py:65`).
///
/// 运行期 deferred：上游响应经 `jm.js` 加密，需 JS 解密，Rust 端未实现。
/// 解析函数 `parse_business_value_artist` 已完整实现。
pub async fn business_value_artist(client: &Client) -> Result<Vec<BusinessValueArtist>> {
    let _ = client;
    Err(Error::UpstreamChanged {
        origin: SOURCE_YIEN,
        message: "艺恩 GetData.ashx 响应需 jm.js JS 解密，Rust 端未实现".into(),
    })
}

pub fn parse_business_value_artist(resp: &Value) -> Result<Vec<BusinessValueArtist>> {
    let table = yien_table(resp)?;
    let mut out = Vec::with_capacity(table.len());
    for row in table {
        let vals = yien_row_values(row);
        let Some(artist) = yien_val_str(&vals, 2) else {
            continue;
        };
        out.push(BusinessValueArtist {
            rank: yien_val_f64(&vals, 0),
            artist,
            business_value: yien_val_f64(&vals, 3),
            major_heat: yien_val_f64(&vals, 5),
            attention_heat: yien_val_f64(&vals, 6),
            forecast_heat: yien_val_f64(&vals, 7),
            reputation: yien_val_f64(&vals, 8),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// online_value_artist — 艺人流量价值
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct OnlineValueArtist {
    /// 排名（akshare `排名`）。
    pub rank: Option<f64>,
    /// 艺人名（akshare `艺人`）。
    pub artist: String,
    /// 流量价值（akshare `流量价值`）。
    pub flow_value: Option<f64>,
    /// 专业热度（akshare `专业热度`）。
    pub major_heat: Option<f64>,
    /// 关注热度（akshare `关注热度`）。
    pub attention_heat: Option<f64>,
    /// 预测热度（akshare `预测热度`）。
    pub forecast_heat: Option<f64>,
    /// 带货力（akshare `带货力`）。
    pub carrying_power: Option<f64>,
}

/// 艺人流量价值（`online_value_artist`, 艺恩 `GetData.ashx`
/// `Data_GetList_Star` / `FlowValueIndex_L1`, akshare `artist_yien.py:103`）。
///
/// 运行期 deferred（同 `business_value_artist`）。解析函数已实现。
pub async fn online_value_artist(client: &Client) -> Result<Vec<OnlineValueArtist>> {
    let _ = client;
    Err(Error::UpstreamChanged {
        origin: SOURCE_YIEN,
        message: "艺恩 GetData.ashx 响应需 jm.js JS 解密，Rust 端未实现".into(),
    })
}

pub fn parse_online_value_artist(resp: &Value) -> Result<Vec<OnlineValueArtist>> {
    let table = yien_table(resp)?;
    let mut out = Vec::with_capacity(table.len());
    for row in table {
        let vals = yien_row_values(row);
        let Some(artist) = yien_val_str(&vals, 2) else {
            continue;
        };
        out.push(OnlineValueArtist {
            rank: yien_val_f64(&vals, 0),
            artist,
            flow_value: yien_val_f64(&vals, 4),
            major_heat: yien_val_f64(&vals, 5),
            attention_heat: yien_val_f64(&vals, 6),
            forecast_heat: yien_val_f64(&vals, 7),
            carrying_power: yien_val_f64(&vals, 9),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// video_tv / video_variety_show — 电视剧集 / 综艺节目 播映指数
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct VideoRankRow {
    /// 排序（akshare `排序`）。
    pub rank: Option<f64>,
    /// 名称（akshare `名称`）。
    pub name: String,
    /// 类型（akshare `类型`）。
    pub genre: Option<String>,
    /// 播映指数（akshare `播映指数`）。
    pub broadcast_index: Option<f64>,
    /// 用户热度（akshare `用户热度`）。
    pub user_heat: Option<f64>,
    /// 媒体热度（akshare `媒体热度`）。
    pub media_heat: Option<f64>,
    /// 观看度（akshare `观看度`）。
    pub view_count: Option<f64>,
    /// 好评度（akshare `好评度`）。
    pub reputation: Option<f64>,
    /// 统计日期（akshare `统计日期` = `Data.Table1[0].MaxDate`）。
    pub stat_date: Option<String>,
}

/// 共享解析：`video_tv` 与 `video_variety_show` 的 `Data.Table` 结构相同，仅
/// `tvType` 不同（2=电视剧集, 8=综艺节目）。
fn parse_video_rank(resp: &Value) -> Result<Vec<VideoRankRow>> {
    let table = yien_table(resp)?;
    let stat_date = resp
        .get("Data")
        .and_then(|d| d.get("Table1"))
        .and_then(|t| t.as_array())
        .and_then(|a| a.first())
        .and_then(|o| o.get("MaxDate"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mut out = Vec::with_capacity(table.len());
    for row in table {
        let vals = yien_row_values(row);
        let Some(name) = yien_val_str(&vals, 1) else {
            continue;
        };
        out.push(VideoRankRow {
            rank: yien_val_f64(&vals, 0),
            name,
            genre: yien_val_str(&vals, 2),
            broadcast_index: yien_val_f64(&vals, 3),
            user_heat: yien_val_f64(&vals, 4),
            media_heat: yien_val_f64(&vals, 5),
            view_count: yien_val_f64(&vals, 6),
            reputation: yien_val_f64(&vals, 7),
            stat_date: stat_date.clone(),
        });
    }
    Ok(out)
}

/// 电视剧集播映指数（`video_tv`, 艺恩 `GetData.ashx`
/// `BoxOffice_GetTvData_PlayIndexRank` / `tvType=2`, akshare `video_yien.py:65`）。
///
/// 运行期 deferred（同 `business_value_artist`）。解析函数已实现。
pub async fn video_tv(client: &Client) -> Result<Vec<VideoRankRow>> {
    let _ = client;
    Err(Error::UpstreamChanged {
        origin: SOURCE_YIEN,
        message: "艺恩 GetData.ashx 响应需 jm.js JS 解密，Rust 端未实现".into(),
    })
}

pub fn parse_video_tv(resp: &Value) -> Result<Vec<VideoRankRow>> {
    parse_video_rank(resp)
}

/// 综艺节目播映指数（`video_variety_show`, 艺恩 `GetData.ashx`
/// `BoxOffice_GetTvData_PlayIndexRank` / `tvType=8`, akshare `video_yien.py:96`）。
///
/// 运行期 deferred（同 `business_value_artist`）。解析函数已实现。
pub async fn video_variety_show(client: &Client) -> Result<Vec<VideoRankRow>> {
    let _ = client;
    Err(Error::UpstreamChanged {
        origin: SOURCE_YIEN,
        message: "艺恩 GetData.ashx 响应需 jm.js JS 解密，Rust 端未实现".into(),
    })
}

pub fn parse_video_variety_show(resp: &Value) -> Result<Vec<VideoRankRow>> {
    parse_video_rank(resp)
}

// ---------------------------------------------------------------------------
// movie_boxoffice_weekly / movie_boxoffice_cinema_weekly — 周榜（上游需权限）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct MovieBoxofficeWeeklyRow {
    /// 排序。
    pub rank: Option<i64>,
    /// 影片名称。
    pub movie_name: String,
    /// 单周票房。
    pub box_office: Option<f64>,
    /// 票房占比。
    pub box_office_percent: Option<f64>,
    /// 上映天数。
    pub release_day: Option<i64>,
    /// 累计票房。
    pub total_box_office: Option<f64>,
    pub source: &'static str,
}

/// 单周票房（`movie_boxoffice_weekly`, akshare `movie_yien.py:340`）。
///
/// DEFERRED：上游艺恩公开周榜接口当前需权限，akshare 自身对该函数直接 raise
/// `APIError`（`_raise_week_permission_error`），无法匿名获取确定性数据。
pub async fn movie_boxoffice_weekly(
    client: &Client,
    date: &str,
) -> Result<Vec<MovieBoxofficeWeeklyRow>> {
    let _ = (client, date);
    Err(Error::UpstreamChanged {
        origin: SOURCE_ENDATA,
        message: "艺恩周票房公开接口需权限，akshare 亦返回权限错误".into(),
    })
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MovieBoxofficeCinemaWeeklyRow {
    /// 排序。
    pub rank: Option<i64>,
    /// 影院名称。
    pub cinema_name: String,
    /// 单周票房。
    pub box_office: Option<f64>,
    /// 单周场次。
    pub show_count: Option<i64>,
    /// 场均票价。
    pub avg_box_office: Option<f64>,
    /// 上座率。
    pub attendance: Option<f64>,
    pub source: &'static str,
}

/// 影院周票房（`movie_boxoffice_cinema_weekly`, akshare `movie_yien.py:642`）。
///
/// DEFERRED：同 `movie_boxoffice_weekly`，上游需权限，akshare 亦 raise。
pub async fn movie_boxoffice_cinema_weekly(
    client: &Client,
    date: &str,
) -> Result<Vec<MovieBoxofficeCinemaWeeklyRow>> {
    let _ = (client, date);
    Err(Error::UpstreamChanged {
        origin: SOURCE_ENDATA,
        message: "艺恩影院周票房公开接口需权限，akshare 亦返回权限错误".into(),
    })
}

#[cfg(test)]
mod yien_gaps_tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(p).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[ignore = "DEFERRED: endata response is JS-decrypted via py_mini_racer (jm.js); the fixture holds the encrypted payload, not the decrypted JSON this parser consumes (ADR-0005)"]
    #[test]
    fn parses_business_value_artist() {
        let rows = parse_business_value_artist(&fixture("business_value_artist.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].artist, "艺人甲");
        assert_eq!(rows[0].rank, Some(1.0));
        assert_eq!(rows[0].business_value, Some(95.6));
        assert_eq!(rows[0].major_heat, Some(88.1));
        assert_eq!(rows[0].attention_heat, Some(90.2));
        assert_eq!(rows[0].forecast_heat, Some(87.5));
        assert_eq!(rows[0].reputation, Some(92.3));
        assert_eq!(rows[2].artist, "艺人丙");
    }

    #[ignore = "DEFERRED: endata response is JS-decrypted via py_mini_racer (jm.js); the fixture holds the encrypted payload, not the decrypted JSON this parser consumes (ADR-0005)"]
    #[test]
    fn parses_online_value_artist() {
        let rows = parse_online_value_artist(&fixture("online_value_artist.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].artist, "艺人甲");
        assert_eq!(rows[0].flow_value, Some(93.1));
        assert_eq!(rows[0].carrying_power, Some(79.3));
        assert_eq!(rows[0].major_heat, Some(87.0));
        assert_eq!(rows[0].attention_heat, Some(91.2));
        assert_eq!(rows[0].forecast_heat, Some(85.6));
    }

    #[ignore = "DEFERRED: endata response is JS-decrypted via py_mini_racer (jm.js); the fixture holds the encrypted payload, not the decrypted JSON this parser consumes (ADR-0005)"]
    #[test]
    fn parses_video_tv() {
        let rows = parse_video_tv(&fixture("video_tv.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].name, "剧集甲");
        assert_eq!(rows[0].rank, Some(1.0));
        assert_eq!(rows[0].broadcast_index, Some(85.2));
        assert_eq!(rows[0].user_heat, Some(80.1));
        assert_eq!(rows[0].media_heat, Some(78.3));
        assert_eq!(rows[0].view_count, Some(76.5));
        assert_eq!(rows[0].reputation, Some(88.9));
        assert_eq!(rows[0].stat_date, Some("2024-01-15".to_string()));
    }

    #[ignore = "DEFERRED: endata response is JS-decrypted via py_mini_racer (jm.js); the fixture holds the encrypted payload, not the decrypted JSON this parser consumes (ADR-0005)"]
    #[test]
    fn parses_video_variety_show() {
        let rows = parse_video_variety_show(&fixture("video_variety_show.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].name, "综艺甲");
        assert_eq!(rows[0].genre, Some("真人秀".to_string()));
        assert_eq!(rows[0].broadcast_index, Some(88.5));
        assert_eq!(rows[0].view_count, Some(79.4));
        assert_eq!(rows[0].stat_date, Some("2024-01-15".to_string()));
    }

    #[test]
    fn yien_async_endpoints_deferred_or_error() {
        // 艺恩 GetData.ashx 需 jm.js JS 解密（Rust 未实现），周榜上游需权限：
        // 运行期均返回 Err。
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        rt.block_on(async {
            let client = crate::core::client::Client::new();
            assert!(business_value_artist(&client).await.is_err());
            assert!(online_value_artist(&client).await.is_err());
            assert!(video_tv(&client).await.is_err());
            assert!(video_variety_show(&client).await.is_err());
            assert!(movie_boxoffice_weekly(&client, "20240218").await.is_err());
            assert!(movie_boxoffice_cinema_weekly(&client, "20240219")
                .await
                .is_err());
        });
    }
}
