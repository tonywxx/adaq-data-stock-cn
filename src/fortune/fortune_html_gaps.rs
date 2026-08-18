//! Fortune / wealth-ranking HTML endpoints ported from akshare `fortune/`.
//!
//! These five functions all scrape HTML (akshare `pd.read_html` / BeautifulSoup):
//!
//! * [`forbes_rank`] — akshare `fortune/fortune_forbes_500.py:14`
//!   (forbeschina.com lists → detail table).
//! * [`fortune_rank`] — akshare `fortune/fortune_500.py:40`
//!   (fortunechina.com 500 index → year detail table).
//! * [`hurun_rank`] — akshare `fortune/fortune_hurun.py:16`
//!   (hurun.net dropdown → year `<select>` → `HsRankDetailsList` JSON).
//! * [`index_bloomberg_billionaires`] — akshare `fortune/fortune_bloomberg.py:65`
//!   (bloomberg.com `div.table-chart` rows).
//! * [`index_bloomberg_billionaires_hist`] — akshare `fortune/fortune_bloomberg.py:14`
//!   (areppim.com `table#bbXX` historical table).
//!
//! `hurun_rank` is technically a JSON endpoint, but reaching it requires two
//! HTML scrapes (the indicator dropdown and the year `<select>`), so it lives
//! here with the other HTML endpoints.

use std::collections::HashMap;

use scraper::{Html, Selector};

use crate::core::client::Client;
use crate::core::error::{Error, Result};

// ---------------------------------------------------------------------------
// shared HTML helpers
// ---------------------------------------------------------------------------

/// Parse a numeric cell, tolerating thousands separators (e.g. `485,651`).
fn parse_num(s: &str) -> Option<f64> {
    let cleaned: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    cleaned.parse::<f64>().ok()
}

/// Parse every `<table>` in `html` into a `Vec<table>` of `Vec<row>` of
/// `Vec<cell>` strings (mirrors `pd.read_html`, which returns all tables).
/// Rows/empty cells are kept so header position is preserved.
fn extract_tables(html: &str, endpoint: &'static str) -> Result<Vec<Vec<Vec<String>>>> {
    crate::core::html::tables(html, endpoint)
}

/// Resolve a possibly-relative `href` against a base URL (handles `../`).
/// Used for fortunechina's swiper-slide links, which are relative to the
/// redirect-target content page.
fn resolve_url(base: &str, rel: &str) -> String {
    if rel.starts_with("http://") || rel.starts_with("https://") {
        return rel.to_string();
    }
    let base_dir = match base.rfind('/') {
        Some(i) => &base[..=i],
        None => return rel.to_string(),
    };
    let mut segs: Vec<&str> = base_dir.split('/').filter(|s| !s.is_empty()).collect();
    for p in rel.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                if segs.len() > 2 {
                    segs.pop();
                }
            }
            other => segs.push(other),
        }
    }
    if let Some((scheme, rest)) = segs.split_first() {
        let scheme = scheme.trim_end_matches(':');
        format!("{}://{}", scheme, rest.join("/"))
    } else {
        rel.to_string()
    }
}

// ---------------------------------------------------------------------------
// forbes_rank
// ---------------------------------------------------------------------------

/// One entry in a Forbes China ranking (`forbes_rank`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ForbesRank {
    /// Rank (akshare column 1).
    pub rank: Option<f64>,
    /// Name (akshare column 2).
    pub name: String,
    /// Gender (akshare column 3, e.g. `男`/`女`).
    pub gender: String,
    /// Age (akshare column 4).
    pub age: Option<f64>,
    /// Affiliated firm / fund (akshare column 5).
    pub company: String,
    /// Title / position (akshare column 6).
    pub title: String,
}

/// 福布斯中国-榜单 (`forbes_rank`, akshare `fortune_forbes_500.py:14`).
///
/// Fetches the forbeschina.com lists index, resolves `symbol` → detail URL
/// (e.g. `2021福布斯中国创投人100`), then scrapes the detail table.
pub async fn forbes_rank(client: &Client, symbol: &str) -> Result<Vec<ForbesRank>> {
    let index_html = client
        .get_text("forbeschina", "forbes_rank_index", "https://www.forbeschina.com/lists", &[], None)
        .await?;
    let url_map = parse_forbes_index(&index_html, "forbes_rank_index")?;
    let url = url_map
        .get(symbol)
        .ok_or_else(|| Error::InvalidParam(format!("unknown forbes symbol: {symbol}")))?;
    let html = client
        .get_text("forbeschina", "forbes_rank", url, &[], None)
        .await?;
    parse_forbes_rank(&html, "forbes_rank")
}

/// Build the `name → absolute URL` map from the forbeschina lists index.
fn parse_forbes_index(html: &str, endpoint: &'static str) -> Result<HashMap<String, String>> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("div.col-sm-4 a")
        .map_err(|e| Error::Parse { endpoint, message: format!("selector: {e}") })?;
    let mut map = HashMap::new();
    for a in doc.select(&sel) {
        let Some(href) = a.value().attr("href") else { continue };
        let name = a.text().collect::<String>().trim().to_string();
        if name.is_empty() {
            continue;
        }
        let url = if href.starts_with("http") {
            href.to_string()
        } else {
            format!("https://www.forbeschina.com{href}")
        };
        map.insert(name, url);
    }
    if map.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no ranking links found".into() });
    }
    Ok(map)
}

/// Parse the Forbes China detail ranking table (`#data-view`).
///
/// NOTE: forbeschina renders the #1 entry inside `<thead>` (so akshare treats
/// it as the header). We treat every `<tr>` as a data row so all entries —
/// including #1 — are returned.
pub(crate) fn parse_forbes_rank(html: &str, endpoint: &'static str) -> Result<Vec<ForbesRank>> {
    let all = crate::core::html::tables_with(html, endpoint, "table#data-view")?;
    let rows = all
        .into_iter()
        .next()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: "data-view table not found".into(),
        })?;
    let mut out = Vec::new();
    for cells in &rows {
        if cells.len() < 6 {
            continue;
        }
        out.push(ForbesRank {
            rank: cells[0].parse::<f64>().ok(),
            name: cells[1].clone(),
            gender: cells[2].clone(),
            age: cells[3].parse::<f64>().ok(),
            company: cells[4].clone(),
            title: cells[5].clone(),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no ranking rows found".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// fortune_rank
// ---------------------------------------------------------------------------

/// One Fortune Global 500 company for a given year (`fortune_rank`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Fortune500Row {
    /// Rank this year (akshare `排名`).
    pub rank: Option<f64>,
    /// Rank last year (akshare `上年排名`).
    pub prev_rank: Option<f64>,
    /// Company name, Chinese (akshare `公司名称`).
    pub company: String,
    /// Revenue in million USD (akshare `营业收入`).
    pub revenue: Option<f64>,
    /// Profit in million USD (akshare `利润`).
    pub profit: Option<f64>,
    /// Country (akshare `国家`).
    pub country: String,
}

/// 财富 500 强排行榜 (`fortune_rank`, akshare `fortune_500.py:40`).
///
/// Fetches the fortunechina 500 index (which 301-redirects to the current
/// year page), builds the `year → URL` map from the `swiper-slide` carousel,
/// then scrapes the chosen year's table.
pub async fn fortune_rank(client: &Client, year: &str) -> Result<Vec<Fortune500Row>> {
    let index_html = client
        .get_text("fortunechina", "fortune_rank_index", "https://www.fortunechina.com/fortune500/index.htm", &[], None)
        .await?;
    let url_map = parse_fortune_index(&index_html, "fortune_rank_index")?;
    let url = url_map
        .get(year)
        .ok_or_else(|| Error::InvalidParam(format!("unknown fortune year: {year}")))?;
    let html = client
        .get_text("fortunechina", "fortune_rank", url, &[], None)
        .await?;
    parse_fortune_rank(&html, "fortune_rank")
}

/// Build the `year → absolute URL` map from the fortunechina 500 index carousel.
fn parse_fortune_index(html: &str, endpoint: &'static str) -> Result<HashMap<String, String>> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("a[data-year]")
        .map_err(|e| Error::Parse { endpoint, message: format!("selector: {e}") })?;
    let mut map = HashMap::new();
    for a in doc.select(&sel) {
        let Some(year) = a.value().attr("data-year") else { continue };
        let Some(href) = a.value().attr("href") else { continue };
        let url = resolve_url("https://www.fortunechina.com/fortune500/index.htm", href);
        map.insert(year.to_string(), url);
    }
    if map.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no year links found".into() });
    }
    Ok(map)
}

/// Parse a Fortune Global 500 year table (`tables[0]`; year ≥ 2010 path).
pub(crate) fn parse_fortune_rank(html: &str, endpoint: &'static str) -> Result<Vec<Fortune500Row>> {
    let tables = extract_tables(html, endpoint)?;
    let tbl = &tables[0];
    // row 0 is the header; data rows follow.
    let mut out = Vec::new();
    for cells in tbl.iter().skip(1) {
        if cells.len() < 6 {
            continue;
        }
        out.push(Fortune500Row {
            rank: parse_num(&cells[0]),
            prev_rank: parse_num(&cells[1]),
            company: cells[2].clone(),
            revenue: parse_num(&cells[3]),
            profit: parse_num(&cells[4]),
            country: cells[5].clone(),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no company rows found".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// hurun_rank
// ---------------------------------------------------------------------------

/// One Hurun ranking entry (`hurun_rank`).
///
/// Columns are indicator-dependent upstream; we normalise the common shape
/// (rank / wealth / name / company / industry) across all indicators.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HurunRow {
    /// Rank (akshare `排名`).
    pub rank: Option<f64>,
    /// Wealth / valuation (akshare `财富` / `企业估值` / `成交额`).
    pub wealth: Option<f64>,
    /// Rank change vs prior year (akshare `排名变化`).
    pub rank_change: Option<f64>,
    /// Person / CEO name (akshare `姓名` / `CEO`).
    pub name: String,
    /// Company (akshare `企业` / `企业信息`).
    pub company: String,
    /// Industry (akshare `行业`).
    pub industry: String,
}

/// 胡润排行榜 (`hurun_rank`, akshare `fortune/hurun.py:16`).
///
/// Scrapes the indicator dropdown to resolve `indicator → detail page`, then
/// scrapes the year `<select>` to resolve `year → num` code, then calls the
/// `HsRankDetailsList` JSON API.
pub async fn hurun_rank(client: &Client, indicator: &str, year: &str) -> Result<Vec<HurunRow>> {
    let drop_html = client
        .get_text("hurun", "hurun_rank_dropdown", "https://www.hurun.net/zh-CN/Rank/HsRankDetails?pagetype=rich", &[], None)
        .await?;
    let indicator_url = parse_hurun_indicator_url(&drop_html, "hurun_rank_dropdown", indicator)?;
    let detail_html = client
        .get_text("hurun", "hurun_rank_detail", &indicator_url, &[], None)
        .await?;
    let num = parse_hurun_year_num(&detail_html, "hurun_rank_detail", year)?;
    let list_url = "https://www.hurun.net/zh-CN/Rank/HsRankDetailsList";
    let resp = client
        .get_json(
            "hurun",
            "hurun_rank",
            list_url,
            &[
                ("num", num.as_str()),
                ("search", ""),
                ("offset", "0"),
                ("limit", "20000"),
            ],
        )
        .await?;
    parse_hurun_rank(&resp, indicator)
}

/// Resolve an indicator label to its detail-page URL (exact text match).
fn parse_hurun_indicator_url(html: &str, endpoint: &'static str, indicator: &str) -> Result<String> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("ul.dropdown-menu a")
        .map_err(|e| Error::Parse { endpoint, message: format!("selector: {e}") })?;
    for a in doc.select(&sel) {
        let name = a.text().collect::<String>().trim().to_string();
        if name == indicator {
            if let Some(href) = a.value().attr("href") {
                return Ok(if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("https://www.hurun.net{href}")
                });
            }
        }
    }
    Err(Error::InvalidParam(format!("unknown hurun indicator: {indicator}")))
}

/// Resolve a year label to its `num` code from the detail page `<select>`.
fn parse_hurun_year_num(html: &str, endpoint: &'static str, year: &str) -> Result<String> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("#exampleFormControlSelect1 option")
        .map_err(|e| Error::Parse { endpoint, message: format!("selector: {e}") })?;
    for opt in doc.select(&sel) {
        let text = opt.text().collect::<String>().split_whitespace().collect::<String>();
        let text = text.trim_end_matches('年').to_string();
        if text == year {
            if let Some(value) = opt.value().attr("value") {
                if let Some(num) = value.split("num=").nth(1) {
                    return Ok(num.to_string());
                }
            }
        }
    }
    Err(Error::InvalidParam(format!("unknown hurun year: {year}")))
}

/// Per-indicator column-key tuple: (rank, wealth, rank_change, name, company, industry).
fn hurun_keys(indicator: &str) -> (&str, &str, &str, &str, &str, &str) {
    match indicator {
        "胡润百富榜" => ("hs_Rank_Rich_Ranking", "hs_Rank_Rich_Wealth", "hs_Rank_Rich_Ranking_Change", "hs_Rank_Rich_ChaName_Cn", "hs_Rank_Rich_ComName_Cn", "hs_Rank_Rich_Industry_Cn"),
        "胡润全球富豪榜" => ("hs_Rank_Global_Ranking", "hs_Rank_Global_Wealth", "hs_Rank_Global_Ranking_Change", "hs_Rank_Global_ChaName_Cn", "hs_Rank_Global_ComName_Cn", "hs_Rank_Global_Industry_Cn"),
        "胡润印度榜" => ("hs_Rank_India_Ranking", "hs_Rank_India_Wealth", "hs_Rank_India_Ranking_Change", "hs_Rank_India_ChaName_Cn", "hs_Rank_India_ComName_Cn", "hs_Rank_India_Industry_Cn"),
        "胡润全球独角兽榜" => ("hs_Rank_Unicorn_Ranking", "hs_Rank_Unicorn_Wealth", "hs_Rank_Unicorn_Ranking_Change", "hs_Rank_Unicorn_ChaName_Cn", "hs_Rank_Unicorn_ComName_Cn", "hs_Rank_Unicorn_Industry_Cn"),
        "胡润中国500强民营企业" => ("hs_Rank_CTop500_Ranking", "hs_Rank_CTop500_Wealth", "hs_Rank_CTop500_Ranking_Change", "hs_Rank_CTop500_ChaName_Cn", "hs_Rank_CTop500_ComName_Cn", "hs_Rank_CTop500_Industry_Cn"),
        "胡润世界500强" => ("hs_Rank_GTop500_Ranking", "hs_Rank_GTop500_Wealth", "hs_Rank_GTop500_Ranking_Change", "hs_Rank_GTop500_ChaName_Cn", "hs_Rank_GTop500_ComName_Cn", "hs_Rank_GTop500_Industry_Cn"),
        "胡润艺术榜" => ("hs_Rank_Art_Ranking", "hs_Rank_Art_Turnover", "hs_Rank_Art_Ranking_Change", "hs_Rank_Art_Name_Cn", "", "hs_Rank_Art_ArtCategory_Cn"),
        "中国瞪羚企业榜" => ("", "", "", "hs_Rank_CGazelles_Name_Cn", "hs_Rank_CGazelles_ComName_Cn", "hs_Rank_CGazelles_Industry_Cn"),
        "全球瞪羚企业榜" => ("", "", "", "hs_Rank_GGazelles_Name_Cn", "hs_Rank_GGazelles_ComName_Cn", "hs_Rank_GGazelles_Industry_Cn"),
        "胡润Under30s创业领袖榜" => ("", "", "", "hs_Rank_U30_ChaName_Cn", "hs_Rank_U30_ComName_Cn", "hs_Rank_U30_Industry_Cn"),
        _ => ("hs_Rank_Rich_Ranking", "hs_Rank_Rich_Wealth", "hs_Rank_Rich_Ranking_Change", "hs_Rank_Rich_ChaName_Cn", "hs_Rank_Rich_ComName_Cn", "hs_Rank_Rich_Industry_Cn"),
    }
}

fn hurun_str<'a>(row: &'a serde_json::Value, key: &str) -> String {
    if key.is_empty() {
        return String::new();
    }
    row.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn hurun_f64(row: &serde_json::Value, key: &str) -> Option<f64> {
    if key.is_empty() {
        return None;
    }
    match row.get(key) {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Parse the `HsRankDetailsList` JSON `rows` for one indicator.
pub(crate) fn parse_hurun_rank(resp: &serde_json::Value, indicator: &str) -> Result<Vec<HurunRow>> {
    let rows = resp
        .get("rows")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged { origin: "hurun", message: "missing rows".into() })?;
    let (rk, wk, rck, nk, ck, ik) = hurun_keys(indicator);
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(HurunRow {
            rank: hurun_f64(row, rk),
            wealth: hurun_f64(row, wk),
            rank_change: hurun_f64(row, rck),
            name: hurun_str(row, nk),
            company: hurun_str(row, ck),
            industry: hurun_str(row, ik),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// index_bloomberg_billionaires
// ---------------------------------------------------------------------------

/// One live Bloomberg Billionaires Index entry (`index_bloomberg_billionaires`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BloombergBillionaire {
    /// Rank (akshare `rank`).
    pub rank: Option<f64>,
    /// Name (akshare `name`).
    pub name: String,
    /// Total net worth, e.g. `$858B` (akshare `total_net_worth`).
    pub total_net_worth: String,
    /// Last change, e.g. `-$5.65B` (akshare `last_change`).
    pub last_change: String,
    /// Year-to-date change, e.g. `+$239B` (akshare `YTD_change`).
    pub ytd_change: String,
    /// Country / region (akshare `country`).
    pub country: String,
    /// Industry (akshare `industry`).
    pub industry: String,
}

/// 彭博亿万富豪指数 (`index_bloomberg_billionaires`, akshare `fortune_bloomberg.py:65`).
///
/// Bloomberg blocks the default client UA, so a browser UA is sent.
pub async fn index_bloomberg_billionaires(client: &Client) -> Result<Vec<BloombergBillionaire>> {
    let headers: &[(&str, &str)] = &[(
        "user-agent",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.0 Safari/605.1.15",
    )];
    let html = client
        .get_text("bloomberg", "index_bloomberg_billionaires", "https://www.bloomberg.com/billionaires", &[], Some(headers))
        .await?;
    parse_index_bloomberg_billionaires(&html, "index_bloomberg_billionaires")
}

/// Parse the `div.table-chart` → `div.table-row` structure.
pub(crate) fn parse_index_bloomberg_billionaires(
    html: &str,
    endpoint: &'static str,
) -> Result<Vec<BloombergBillionaire>> {
    let doc = Html::parse_document(html);
    let chart_sel = Selector::parse("div.table-chart")
        .map_err(|e| Error::Parse { endpoint, message: format!("chart selector: {e}") })?;
    let row_sel = Selector::parse("div.table-row").unwrap();
    let cell_sel = Selector::parse("div.table-cell").unwrap();
    let chart = doc
        .select(&chart_sel)
        .next()
        .ok_or_else(|| Error::UpstreamChanged { origin: endpoint, message: "no table-chart".into() })?;
    let mut out = Vec::new();
    for row in chart.select(&row_sel) {
        let cells: Vec<String> = row
            .select(&cell_sel)
            .map(|c| c.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .collect();
        if cells.len() < 7 {
            continue;
        }
        out.push(BloombergBillionaire {
            rank: cells[0].parse::<f64>().ok(),
            name: cells[1].clone(),
            total_net_worth: cells[2].clone(),
            last_change: cells[3].clone(),
            ytd_change: cells[4].clone(),
            country: cells[5].clone(),
            industry: cells[6].clone(),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no billionaire rows".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// index_bloomberg_billionaires_hist
// ---------------------------------------------------------------------------

/// One historical Bloomberg Billionaires Index entry (`index_bloomberg_billionaires_hist`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BloombergBillionaireHist {
    /// Rank (akshare `rank`).
    pub rank: Option<f64>,
    /// Name (akshare `name`).
    pub name: String,
    /// Age (akshare `age`, present on some years' tables).
    pub age: Option<f64>,
    /// Country (akshare `country`).
    pub country: String,
    /// Total net worth in $Billion (akshare `total_net_worth`).
    pub total_net_worth: Option<f64>,
    /// Last change (akshare `last_change`, e.g. `-$395M`).
    pub last_change: String,
    /// Year-to-date change (akshare `ytd_change`, e.g. `-$3.85B`).
    pub ytd_change: String,
    /// Industry (akshare `industry`).
    pub industry: String,
}

/// 彭博亿万富豪指数历史数据 (`index_bloomberg_billionaires_hist`, akshare `fortune_bloomberg.py:14`).
pub async fn index_bloomberg_billionaires_hist(client: &Client, year: &str) -> Result<Vec<BloombergBillionaireHist>> {
    let yy = year.chars().rev().take(2).collect::<String>().chars().rev().collect::<String>();
    let url = format!("https://stats.areppim.com/listes/list_billionairesx{yy}xwor.htm");
    let html = client
        .get_text("areppim", "index_bloomberg_billionaires_hist", &url, &[], None)
        .await?;
    parse_index_bloomberg_billionaires_hist(&html, "index_bloomberg_billionaires_hist")
}

/// Parse the areppim historical table (`table[0]`); map columns by header name.
pub(crate) fn parse_index_bloomberg_billionaires_hist(
    html: &str,
    endpoint: &'static str,
) -> Result<Vec<BloombergBillionaireHist>> {
    let tables = extract_tables(html, endpoint)?;
    let tbl = &tables[0];
    if tbl.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "empty table".into() });
    }
    let header = &tbl[0];
    let col = |name: &str| -> Option<usize> {
        header
            .iter()
            .position(|h| h.split_whitespace().collect::<String>().eq_ignore_ascii_case(name))
    };
    let idx_rank = col("Rank");
    let idx_name = col("Name");
    let idx_age = col("Age");
    let idx_country = header
        .iter()
        .position(|h| h.contains("Country"))
        .or_else(|| col("Country"));
    let idx_nw = header
        .iter()
        .position(|h| h.contains("Net Worth") || h.contains("net worth"))
        .or_else(|| col("Total net worth$Billion"));
    let idx_lc = header.iter().position(|h| h.contains("Last change")).or_else(|| col("$ Last change"));
    let idx_yc = header.iter().position(|h| h.contains("YTD change")).or_else(|| col("$ YTD change"));
    let idx_ind = header.iter().position(|h| h.contains("Industry")).or_else(|| col("Industry"));

    let get = |cells: &[String], i: Option<usize>| -> String {
        i.and_then(|i| cells.get(i)).cloned().unwrap_or_default()
    };
    let get_f64 = |cells: &[String], i: Option<usize>| -> Option<f64> {
        i.and_then(|i| cells.get(i)).and_then(|s| s.trim().parse::<f64>().ok())
    };

    let mut out = Vec::new();
    for cells in tbl.iter().skip(1) {
        out.push(BloombergBillionaireHist {
            rank: get_f64(cells, idx_rank),
            name: get(cells, idx_name),
            age: get_f64(cells, idx_age),
            country: get(cells, idx_country),
            total_net_worth: get_f64(cells, idx_nw),
            last_change: get(cells, idx_lc),
            ytd_change: get(cells, idx_yc),
            industry: get(cells, idx_ind),
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged { origin: endpoint, message: "no billionaire rows".into() });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn load_html(name: &str) -> String {
        let bytes = std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)).unwrap();
        match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_string(),
            Err(_) => encoding_rs::GBK
                .decode_without_bom_handling_and_without_replacement(&bytes)
                .map(|c| c.into_owned())
                .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned()),
        }
    }

    fn load_json(name: &str) -> serde_json::Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    #[test]
    fn parses_forbes_rank() {
        let rows = parse_forbes_rank(&load_html("forbes_rank.html"), "forbes_rank").unwrap();
        // forbeschina renders the #1 entry in <thead>, so we keep all 100 rows.
        assert!(rows.len() >= 100, "expected ~100 rows, got {}", rows.len());
        assert_eq!(rows[0].rank, Some(1.0));
        assert_eq!(rows[0].name, "沈南鹏");
        assert_eq!(rows[0].company, "红杉中国");
    }

    #[test]
    fn parses_fortune_rank() {
        let rows = parse_fortune_rank(&load_html("fortune_rank.html"), "fortune_rank").unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].rank, Some(1.0));
        assert!(!rows[0].company.is_empty());
        assert!(rows[0].revenue.is_some());
    }

    #[test]
    fn parses_hurun_rank() {
        let v = load_json("hurun_rank.json");
        let rows = parse_hurun_rank(&v, "胡润百富榜").unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].rank, Some(1.0));
        assert!(!rows[0].name.is_empty());
        assert!(rows[0].wealth.is_some());
    }

    #[test]
    fn parses_index_bloomberg_billionaires() {
        let rows = parse_index_bloomberg_billionaires(&load_html("index_bloomberg_billionaires.html"), "index_bloomberg_billionaires").unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].rank, Some(1.0));
        assert_eq!(rows[0].name, "Elon Musk");
        assert!(rows[0].total_net_worth.contains('B'));
        assert!(!rows[0].industry.is_empty());
    }

    #[test]
    fn parses_index_bloomberg_billionaires_hist() {
        let rows = parse_index_bloomberg_billionaires_hist(&load_html("index_bloomberg_billionaires_hist.html"), "index_bloomberg_billionaires_hist").unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows[0].rank, Some(1.0));
        assert_eq!(rows[0].name, "Jeff Bezos");
        assert_eq!(rows[0].total_net_worth, Some(186.0));
        assert!(rows[0].last_change.contains('M') || rows[0].last_change.contains('B'));
    }
}
