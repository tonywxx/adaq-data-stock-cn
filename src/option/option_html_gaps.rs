//! `option` gap fillers — HTML (BeautifulSoup / `pd.read_html`) endpoints.
//!
//! Ports nine akshare `option` functions whose upstreams return HTML that
//! akshare scrapes with BeautifulSoup or `pd.read_html`.
//!
//! * [`option_cffex_hs300_list_sina`] / [`option_cffex_sz50_list_sina`] /
//!   [`option_cffex_zz1000_list_sina`] — akshare `option/option_finance_sina.py`
//!   (Sina CFFEX option-contract lists, `#option_symbol` / `#option_suffix`).
//! * [`option_comm_symbol`] / [`option_comm_info`] — akshare
//!   `option/option_comm_qihuo.py` (9qihuo.com commodity-option fee tables).
//! * [`option_commodity_contract_sina`] / [`option_commodity_contract_table_sina`]
//!   — akshare `option/option_commodity_sina.py` (Sina commodity-option contracts).
//! * [`option_margin_symbol`] / [`option_margin`] — akshare `option/option_margin.py`
//!   (iweiai.com commodity-option margin tables).
//!
//! NOTE: 9qihuo / Sina / iweiai serve `gbk` HTML. The `load_html` test helper
//! decodes `gbk`→UTF-8 so the parser logic and Chinese names are verified; the
//! live `Client::get_text` path returns UTF-8 (so names may be mojibake while
//! ASCII codes/numbers stay reliable). Numeric columns are always parsed as
//! `Option<f64>` and ASCII contract codes are unaffected.

use scraper::{Html, Selector};
use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_SINA: &str = "sina";
const SOURCE_JIQIHUO: &str = "9qihuo";
const SOURCE_IWEIAI: &str = "iweiai";

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

/// Extract every `<table>` from an HTML document as `table → rows → cells`.
/// Empty rows (no cells) are skipped; tables with no rows are skipped.
fn extract_tables(html: &str, endpoint: &'static str) -> Result<Vec<Vec<Vec<String>>>> {
    crate::core::html::tables(html, endpoint)
}

/// Parse a JSON scalar into `Option<f64>`, tolerating `"-"` / string numbers.
fn jnum(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() || t == "-" {
                None
            } else {
                t.parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

/// Get the `i`-th cell of a row, trimmed; empty string when missing.
fn cell(row: &[String], i: usize) -> String {
    row.get(i).cloned().unwrap_or_default().trim().to_string()
}

/// Parse a cell into `Option<f64>` (`"-"` / empty → `None`).
fn f64opt(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() || t == "-" {
        None
    } else {
        t.replace(',', "").parse::<f64>().ok()
    }
}

/// Extract the two update timestamps embedded before the `a#dlink` tag in the
/// 9qihuo fee page (best-effort; empty when absent).
fn extract_9qihuo_times(html: &str) -> (String, String) {
    let mut comm = String::new();
    let mut price = String::new();
    if let Some(i) = html.find("手续费更新时间：") {
        let s = &html[i + "手续费更新时间：".len()..];
        if let Some(j) = s.find('，') {
            comm = s[..j].trim().to_string();
            let s2 = &s[j + "，".len()..];
            if let Some(k) = s2.find('。') {
                price = s2[..k].trim().trim_start_matches("价格更新时间：").trim().to_string();
            }
        }
    }
    (comm, price)
}

// ---------------------------------------------------------------------------
// CFFEX option-contract lists (Sina) — #option_symbol / #option_suffix
// ---------------------------------------------------------------------------

/// One contract of a CFFEX index option (沪深300 / 上证50 / 中证1000).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CffexListSinaRow {
    /// Index name (akshare `symbol`, e.g. 沪深300指数).
    pub symbol: String,
    /// Contract code (akshare `contract`, e.g. io2608).
    pub contract: String,
}

/// Shared parser for the Sina CFFEX option pages. `symbol_idx` selects which
/// entry of `#option_symbol li` names the option (0=上证50, 1=沪深300, 2=中证1000).
fn parse_cffex_list(
    html: &str,
    endpoint: &'static str,
    symbol_idx: usize,
) -> Result<Vec<CffexListSinaRow>> {
    let doc = Html::parse_document(html);
    let sym_sel = Selector::parse("#option_symbol li")
        .map_err(|e| Error::Parse { endpoint, message: format!("option_symbol selector: {e}") })?;
    let lis: Vec<_> = doc.select(&sym_sel).collect();
    if lis.len() <= symbol_idx {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: format!("option_symbol li[{symbol_idx}] missing"),
        });
    }
    let symbol = lis[symbol_idx].text().collect::<String>().trim().to_string();
    let suf_sel = Selector::parse("#option_suffix li")
        .map_err(|e| Error::Parse { endpoint, message: format!("option_suffix selector: {e}") })?;
    let mut out = Vec::new();
    for li in doc.select(&suf_sel) {
        let contract = li.text().collect::<String>().trim().to_string();
        if contract.is_empty() {
            continue;
        }
        out.push(CffexListSinaRow {
            symbol: symbol.clone(),
            contract,
        });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "no contracts in #option_suffix".into(),
        });
    }
    Ok(out)
}

/// 新浪财经-中金所-沪深300指数-所有合约
/// (`option_cffex_hs300_list_sina`, akshare `option/option_finance_sina.py:45`).
pub async fn option_cffex_hs300_list_sina(client: &Client) -> Result<Vec<CffexListSinaRow>> {
    let url = "https://stock.finance.sina.com.cn/futures/view/optionsCffexDP.php";
    let html = client
        .get_text(SOURCE_SINA, "option_cffex_hs300_list_sina", url, &[], None)
        .await?;
    parse_cffex_list(&html, "option_cffex_hs300_list_sina", 1)
}

/// 新浪财经-中金所-上证50指数-所有合约
/// (`option_cffex_sz50_list_sina`, akshare `option/option_finance_sina.py:28`).
pub async fn option_cffex_sz50_list_sina(client: &Client) -> Result<Vec<CffexListSinaRow>> {
    let url = "https://stock.finance.sina.com.cn/futures/view/optionsCffexDP.php/ho/cffex";
    let html = client
        .get_text(SOURCE_SINA, "option_cffex_sz50_list_sina", url, &[], None)
        .await?;
    parse_cffex_list(&html, "option_cffex_sz50_list_sina", 0)
}

/// 新浪财经-中金所-中证1000指数-所有合约
/// (`option_cffex_zz1000_list_sina`, akshare `option/option_finance_sina.py:61`).
pub async fn option_cffex_zz1000_list_sina(client: &Client) -> Result<Vec<CffexListSinaRow>> {
    let url = "https://stock.finance.sina.com.cn/futures/view/optionsCffexDP.php/mo/cffex";
    let html = client
        .get_text(SOURCE_SINA, "option_cffex_zz1000_list_sina", url, &[], None)
        .await?;
    parse_cffex_list(&html, "option_cffex_zz1000_list_sina", 2)
}

// ---------------------------------------------------------------------------
// 9qihuo commodity-option fee symbols & info
// ---------------------------------------------------------------------------

/// One commodity-option variety and its 9qihuo `heyue` code.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CommSymbolRow {
    /// Variety name (akshare `品种名称`).
    pub name: String,
    /// Variety code (akshare `品种代码`, the `heyue=` value).
    pub code: String,
}

/// Parse `div#inst_list a` → (name, code).
pub(crate) fn parse_option_comm_symbol(html: &str, endpoint: &'static str) -> Result<Vec<CommSymbolRow>> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("#inst_list a")
        .map_err(|e| Error::Parse { endpoint, message: format!("inst_list selector: {e}") })?;
    let mut out = Vec::new();
    for a in doc.select(&sel) {
        let name = a.text().collect::<String>().trim().to_string();
        let href = a.value().attr("href").unwrap_or("");
        let code = href
            .split("heyue=")
            .nth(1)
            .unwrap_or("")
            .split(['&', '#'])
            .next()
            .unwrap_or("")
            .to_string();
        if name.is_empty() || code.is_empty() {
            continue;
        }
        out.push(CommSymbolRow { name, code });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "no inst_list links found".into(),
        });
    }
    Ok(out)
}

/// 九期网-商品期权手续费-品种代码
/// (`option_comm_symbol`, akshare `option/option_comm_qihuo.py:18`).
pub async fn option_comm_symbol(client: &Client) -> Result<Vec<CommSymbolRow>> {
    let url = "https://www.9qihuo.com/qiquanshouxufei";
    let html = client
        .get_text(SOURCE_JIQIHUO, "option_comm_symbol", url, &[], None)
        .await?;
    parse_option_comm_symbol(&html, "option_comm_symbol")
}

/// One row of the 9qihuo commodity-option fee table for a variety.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CommInfoRow {
    /// Exchange name (akshare `交易所`, table header row 0).
    pub exchange: String,
    /// Option variety + strike (akshare `期权品种`).
    pub option_name: String,
    /// Latest price (akshare `现价`).
    pub current_price: Option<f64>,
    /// Limit up/down band (akshare `涨/跌停板`).
    pub limit: String,
    /// Volume (akshare `成交量`).
    pub volume: Option<f64>,
    /// Call/Put (akshare `类型`).
    pub kind: String,
    /// Buyer premium (akshare `买方权利金`).
    pub buyer_premium: String,
    /// Open commission (akshare `开仓手续费`).
    pub open_fee: String,
    /// Close-yesterday commission (akshare `平昨手续费`).
    pub close_yesterday_fee: String,
    /// Close-today commission (akshare `平今手续费`).
    pub close_today_fee: String,
    /// Exercise commission (akshare `行权手续费`).
    pub exercise_fee: String,
    /// Gross profit per tick (akshare `每跳毛利/元`).
    pub gross_per_tick: Option<f64>,
    /// Open+close commission (akshare `手续费(开+平)`).
    pub fee_open_close: String,
    /// Net profit per tick (akshare `每跳净利/元`).
    pub net_per_tick: Option<f64>,
    /// Remarks (akshare `备注`).
    pub remarks: String,
    /// Fee update time (akshare `手续费更新时间`).
    pub fee_update_time: String,
    /// Price update time (akshare `价格更新时间`).
    pub price_update_time: String,
}

/// Parse the 9qihuo `heyuetbl` table (the `#inst_list` page already gave the
/// `heyue` code; this parses the `?heyue=<code>` response).
pub(crate) fn parse_option_comm_info(html: &str, endpoint: &'static str) -> Result<Vec<CommInfoRow>> {
    let tables = extract_tables(html, endpoint)?;
    let table = tables
        .iter()
        .find(|t| t.len() > 2 && t[1].iter().any(|c| c.contains("期权品种")))
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: "fee table (期权品种) not found".into(),
        })?;
    let exchange = table.first().and_then(|r| r.first()).cloned().unwrap_or_default();
    let (fee_update_time, price_update_time) = extract_9qihuo_times(html);
    if table.len() < 4 {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "fee table has no data rows".into(),
        });
    }
    let mut out = Vec::with_capacity(table.len() - 3);
    for row in &table[3..] {
        out.push(CommInfoRow {
            exchange: exchange.clone(),
            option_name: cell(row, 0),
            current_price: f64opt(&cell(row, 1)),
            limit: cell(row, 2),
            volume: f64opt(&cell(row, 3)),
            kind: cell(row, 4),
            buyer_premium: cell(row, 5),
            open_fee: cell(row, 6),
            close_yesterday_fee: cell(row, 7),
            close_today_fee: cell(row, 8),
            exercise_fee: cell(row, 9),
            gross_per_tick: f64opt(&cell(row, 10)),
            fee_open_close: cell(row, 11),
            net_per_tick: f64opt(&cell(row, 12)),
            remarks: cell(row, 13),
            fee_update_time: fee_update_time.clone(),
            price_update_time: price_update_time.clone(),
        });
    }
    Ok(out)
}

/// 九期网-商品期权手续费
/// (`option_comm_info`, akshare `option/option_comm_qihuo.py:38`).
pub async fn option_comm_info(client: &Client, symbol: &str) -> Result<Vec<CommInfoRow>> {
    let symbols = option_comm_symbol(client).await?;
    let code = symbols
        .iter()
        .find(|r| r.name == symbol)
        .map(|r| r.code.clone())
        .ok_or_else(|| Error::Parse {
            endpoint: "option_comm_info",
            message: format!("variety {symbol} not found in 9qihuo list"),
        })?;
    let url = format!("https://www.9qihuo.com/qiquanshouxufei?heyue={code}");
    let html = client
        .get_text(SOURCE_JIQIHUO, "option_comm_info", &url, &[], None)
        .await?;
    parse_option_comm_info(&html, "option_comm_info")
}

// ---------------------------------------------------------------------------
// Sina commodity-option contracts
// ---------------------------------------------------------------------------

/// One contract month of a Sina commodity option variety.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CommContractSinaRow {
    /// Variety name (akshare `symbol`, e.g. 沪金期权).
    pub symbol: String,
    /// 1-based sequence (akshare `序号`).
    pub seq: i64,
    /// Contract code (akshare `合约`, e.g. au2610).
    pub contract: String,
}

/// Parse the Sina commodity-option symbol page (`#option_symbol .selected` +
/// `#option_suffix li`).
pub(crate) fn parse_option_commodity_contract_sina(
    html: &str,
    endpoint: &'static str,
) -> Result<Vec<CommContractSinaRow>> {
    let doc = Html::parse_document(html);
    let sym_sel = Selector::parse("#option_symbol .selected")
        .map_err(|e| Error::Parse { endpoint, message: format!("option_symbol selector: {e}") })?;
    let symbol = doc
        .select(&sym_sel)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_default();
    let suf_sel = Selector::parse("#option_suffix li")
        .map_err(|e| Error::Parse { endpoint, message: format!("option_suffix selector: {e}") })?;
    let mut out = Vec::new();
    let mut seq: i64 = 1;
    for li in doc.select(&suf_sel) {
        let contract = li.text().collect::<String>().trim().to_string();
        if contract.is_empty() {
            continue;
        }
        out.push(CommContractSinaRow {
            symbol: symbol.clone(),
            seq,
            contract,
        });
        seq += 1;
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "no contracts in #option_suffix".into(),
        });
    }
    Ok(out)
}

/// Parse the Sina commodity-option listing page into a (name → relative href)
/// map (used to resolve a variety name to its product page).
fn parse_commodity_listing(html: &str, endpoint: &'static str) -> Result<Vec<(String, String)>> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("#option_symbol li.active a")
        .map_err(|e| Error::Parse { endpoint, message: format!("listing selector: {e}") })?;
    let mut out = Vec::new();
    for a in doc.select(&sel) {
        let name = a.text().collect::<String>().trim().to_string();
        let href = a.value().attr("href").unwrap_or("").to_string();
        if name.is_empty() || href.is_empty() {
            continue;
        }
        out.push((name, href));
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "no active commodity options in listing".into(),
        });
    }
    Ok(out)
}

/// 新浪财经-商品期权-当前可查询期权品种的合约日期
/// (`option_commodity_contract_sina`, akshare `option/option_commodity_sina.py:16`).
pub async fn option_commodity_contract_sina(
    client: &Client,
    symbol: &str,
) -> Result<Vec<CommContractSinaRow>> {
    let listing = client
        .get_text(
            SOURCE_SINA,
            "option_commodity_contract_sina",
            "https://stock.finance.sina.com.cn/futures/view/optionsDP.php/pg_o/dce",
            &[],
            None,
        )
        .await?;
    let href = parse_commodity_listing(&listing, "option_commodity_contract_sina")?
        .into_iter()
        .find(|(n, _)| n == symbol)
        .map(|(_, h)| h)
        .ok_or_else(|| Error::Parse {
            endpoint: "option_commodity_contract_sina",
            message: format!("variety {symbol} not found in Sina listing"),
        })?;
    let url = format!("https://stock.finance.sina.com.cn{href}");
    let html = client
        .get_text(SOURCE_SINA, "option_commodity_contract_sina", &url, &[], None)
        .await?;
    parse_option_commodity_contract_sina(&html, "option_commodity_contract_sina")
}

/// One combined call/put quote row of a Sina commodity-option contract.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CommContractTableRow {
    pub call_buy_volume: Option<f64>,
    pub call_buy_price: Option<f64>,
    pub call_last: Option<f64>,
    pub call_sell_price: Option<f64>,
    pub call_sell_volume: Option<f64>,
    pub call_oi: Option<f64>,
    pub call_change: Option<f64>,
    pub strike: Option<f64>,
    pub call_code: String,
    pub put_buy_volume: Option<f64>,
    pub put_buy_price: Option<f64>,
    pub put_last: Option<f64>,
    pub put_sell_price: Option<f64>,
    pub put_sell_volume: Option<f64>,
    pub put_oi: Option<f64>,
    pub put_change: Option<f64>,
    pub put_code: String,
}

/// Parse the Sina `OptionService.getOptionData` JSON (call `up` + put `down`
/// arrays concatenated horizontally, 9 + 8 columns).
pub(crate) fn parse_option_commodity_contract_table_sina(
    json_text: &str,
    endpoint: &'static str,
) -> Result<Vec<CommContractTableRow>> {
    let v: Value = serde_json::from_str(json_text)
        .map_err(|e| Error::Parse { endpoint, message: format!("json: {e}") })?;
    let up = v
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get("up"))
        .and_then(|u| u.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: "missing result.data.up".into(),
        })?;
    let down = v
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get("down"))
        .and_then(|u| u.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: "missing result.data.down".into(),
        })?;
    let n = up.len().min(down.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let u = up[i].as_array().cloned().unwrap_or_default();
        let d = down[i].as_array().cloned().unwrap_or_default();
        out.push(CommContractTableRow {
            call_buy_volume: u.get(0).and_then(jnum),
            call_buy_price: u.get(1).and_then(jnum),
            call_last: u.get(2).and_then(jnum),
            call_sell_price: u.get(3).and_then(jnum),
            call_sell_volume: u.get(4).and_then(jnum),
            call_oi: u.get(5).and_then(jnum),
            call_change: u.get(6).and_then(jnum),
            strike: u.get(7).and_then(jnum),
            call_code: u.get(8).and_then(|v| v.as_str()).unwrap_or("").to_string(),
            put_buy_volume: d.get(0).and_then(jnum),
            put_buy_price: d.get(1).and_then(jnum),
            put_last: d.get(2).and_then(jnum),
            put_sell_price: d.get(3).and_then(jnum),
            put_sell_volume: d.get(4).and_then(jnum),
            put_oi: d.get(5).and_then(jnum),
            put_change: d.get(6).and_then(jnum),
            put_code: d.get(7).and_then(|v| v.as_str()).unwrap_or("").to_string(),
        });
    }
    Ok(out)
}

/// 新浪财经-商品期权-合约实时行情
/// (`option_commodity_contract_table_sina`, akshare `option/option_commodity_sina.py:55`).
pub async fn option_commodity_contract_table_sina(
    client: &Client,
    symbol: &str,
    contract: &str,
) -> Result<Vec<CommContractTableRow>> {
    let listing = client
        .get_text(
            SOURCE_SINA,
            "option_commodity_contract_table_sina",
            "https://stock.finance.sina.com.cn/futures/view/optionsDP.php/pg_o/dce",
            &[],
            None,
        )
        .await?;
    let href = parse_commodity_listing(&listing, "option_commodity_contract_table_sina")?
        .into_iter()
        .find(|(n, _)| n == symbol)
        .map(|(_, h)| h)
        .ok_or_else(|| Error::Parse {
            endpoint: "option_commodity_contract_table_sina",
            message: format!("variety {symbol} not found in Sina listing"),
        })?;
    let parts: Vec<&str> = href.split('/').collect();
    let product = parts.iter().rev().nth(1).copied().unwrap_or("");
    let exchange = parts.last().copied().unwrap_or("");
    let url = "https://stock.finance.sina.com.cn/futures/api/openapi.php/OptionService.getOptionData";
    let params = [
        ("type", "futures"),
        ("product", product),
        ("exchange", exchange),
        ("pinzhong", contract),
    ];
    let json = client
        .get_text(
            SOURCE_SINA,
            "option_commodity_contract_table_sina",
            url,
            &params,
            None,
        )
        .await?;
    parse_option_commodity_contract_table_sina(&json, "option_commodity_contract_table_sina")
}

// ---------------------------------------------------------------------------
// iweiai commodity-option margin
// ---------------------------------------------------------------------------

/// One commodity-option variety and its iweiai margin page URL.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MarginSymbolRow {
    /// Variety name (akshare `symbol`).
    pub symbol: String,
    /// Margin page URL (akshare `url`).
    pub url: String,
}

/// Parse `a[href*='qiquan']` → (symbol, url).
pub(crate) fn parse_option_margin_symbol(
    html: &str,
    endpoint: &'static str,
) -> Result<Vec<MarginSymbolRow>> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("a[href*='qiquan']")
        .map_err(|e| Error::Parse { endpoint, message: format!("qiquan selector: {e}") })?;
    let mut out = Vec::new();
    for a in doc.select(&sel) {
        let symbol = a.text().collect::<String>().trim().to_string();
        let url = a.value().attr("href").unwrap_or("").to_string();
        if symbol.is_empty() || url.is_empty() {
            continue;
        }
        out.push(MarginSymbolRow { symbol, url });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "no qiquan links found".into(),
        });
    }
    Ok(out)
}

/// 唯爱期货-期权保证金-品种代码和名称
/// (`option_margin_symbol`, akshare `option/option_margin.py:18`).
pub async fn option_margin_symbol(client: &Client) -> Result<Vec<MarginSymbolRow>> {
    let url = "https://www.iweiai.com/qiquan/yuanyou";
    let html = client
        .get_text(SOURCE_IWEIAI, "option_margin_symbol", url, &[], None)
        .await?;
    parse_option_margin_symbol(&html, "option_margin_symbol")
}

/// One row of the iweiai commodity-option margin table.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MarginRow {
    /// Underlying (akshare `合约标的`).
    pub underlying: String,
    /// Contract code (akshare `合约代码`).
    pub code: String,
    /// Settlement price (akshare `结算价`).
    pub settlement: Option<f64>,
    /// Contract multiplier (akshare `交易乘数`).
    pub multiplier: Option<f64>,
    /// Buyer premium (akshare `买方权利金`).
    pub buyer_premium: Option<f64>,
    /// Seller margin (akshare `卖方保证金`).
    pub seller_margin: Option<f64>,
    /// Fee unit (akshare `手续费单位`).
    pub fee_unit: String,
    /// Open commission (akshare `开仓手续费`).
    pub open_fee: Option<f64>,
    /// Close-today commission (akshare `平今手续费`).
    pub close_today_fee: Option<f64>,
    /// Close-yesterday commission (akshare `平昨手续费`).
    pub close_yesterday_fee: Option<f64>,
    /// Open+close-today commission (akshare `手续费(开+平今)`).
    pub fee_open_close_today: Option<f64>,
    /// Update time (akshare `更新时间`, from `<small>`).
    pub update_time: String,
}

/// Extract the first `最近更新` `<small>` text from the iweiai page.
fn extract_iweiai_time(html: &str) -> String {
    let mut start = 0;
    while let Some(i) = html[start..].find("<small") {
        let abs = start + i;
        if let Some(j) = html[abs..].find('>') {
            let inner = &html[abs + j + 1..];
            if let Some(k) = inner.find("</small>") {
                let text = inner[..k].trim().to_string();
                if text.contains("最近更新") {
                    return text;
                }
            }
        }
        start = abs + 1;
    }
    String::new()
}

/// Parse the iweiai margin table (first `<table>`) + update time.
pub(crate) fn parse_option_margin(html: &str, endpoint: &'static str) -> Result<Vec<MarginRow>> {
    let tables = extract_tables(html, endpoint)?;
    let table = tables
        .first()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: endpoint,
            message: "no margin table found".into(),
        })?;
    if table.len() < 2 {
        return Err(Error::UpstreamChanged {
            origin: endpoint,
            message: "margin table has no data rows".into(),
        });
    }
    let update_time = extract_iweiai_time(html);
    let mut out = Vec::with_capacity(table.len() - 1);
    for row in &table[1..] {
        out.push(MarginRow {
            underlying: cell(row, 0),
            code: cell(row, 1),
            settlement: f64opt(&cell(row, 2)),
            multiplier: f64opt(&cell(row, 3)),
            buyer_premium: f64opt(&cell(row, 4)),
            seller_margin: f64opt(&cell(row, 5)),
            fee_unit: cell(row, 6),
            open_fee: f64opt(&cell(row, 7)),
            close_today_fee: f64opt(&cell(row, 8)),
            close_yesterday_fee: f64opt(&cell(row, 9)),
            fee_open_close_today: f64opt(&cell(row, 10)),
            update_time: update_time.clone(),
        });
    }
    Ok(out)
}

/// 唯爱期货-期权保证金
/// (`option_margin`, akshare `option/option_margin.py:38`).
pub async fn option_margin(client: &Client, symbol: &str) -> Result<Vec<MarginRow>> {
    let symbols = option_margin_symbol(client).await?;
    let url = symbols
        .iter()
        .find(|r| r.symbol == symbol)
        .map(|r| r.url.clone())
        .ok_or_else(|| Error::Parse {
            endpoint: "option_margin",
            message: format!("variety {symbol} not found in iweiai list"),
        })?;
    let html = client
        .get_text(SOURCE_IWEIAI, "option_margin", &url, &[], None)
        .await?;
    parse_option_margin(&html, "option_margin")
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Load a fixture (bytes). Decodes `gbk`→UTF-8 when not valid UTF-8 so the
    /// HTML-table parsers and Chinese names are verified.
    fn load(name: &str) -> String {
        let bytes = std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name))
            .unwrap_or_else(|e| panic!("fixture {name}: {e}"));
        match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_string(),
            Err(_) => encoding_rs::GBK
                .decode_without_bom_handling_and_without_replacement(&bytes)
                .unwrap_or_else(|| String::from_utf8_lossy(&bytes))
                .into_owned(),
        }
    }

    #[test]
    fn parses_option_cffex_hs300_list_sina() {
        let rows = parse_cffex_list(&load("option_cffex_hs300_list_sina.html"), "option_cffex_hs300_list_sina", 1).unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].symbol.contains("300"));
        assert!(rows.iter().all(|r| r.contract.starts_with("io")));
    }

    #[test]
    fn parses_option_cffex_sz50_list_sina() {
        let rows = parse_cffex_list(&load("option_cffex_sz50_list_sina.html"), "option_cffex_sz50_list_sina", 0).unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].symbol.contains("50"));
        assert!(rows.iter().all(|r| r.contract.starts_with("ho")));
    }

    #[test]
    fn parses_option_cffex_zz1000_list_sina() {
        let rows = parse_cffex_list(&load("option_cffex_zz1000_list_sina.html"), "option_cffex_zz1000_list_sina", 2).unwrap();
        assert!(!rows.is_empty());
        assert!(rows.iter().all(|r| r.contract.starts_with("mo")));
    }

    #[test]
    fn parses_option_comm_symbol() {
        let rows = parse_option_comm_symbol(&load("option_comm_symbol.html"), "option_comm_symbol").unwrap();
        assert!(!rows.is_empty());
        let au = rows.iter().find(|r| r.code == "au_o").expect("au_o missing");
        assert!(au.name.contains("黄金"));
    }

    #[test]
    fn parses_option_comm_info() {
        let rows = parse_option_comm_info(&load("option_comm_info_si_o.html"), "option_comm_info").unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].exchange.contains("广州"));
        assert!(rows[0].option_name.contains("工业硅"));
        assert!(rows[0].current_price.is_some());
        assert!(!rows[0].fee_update_time.is_empty());
        assert!(!rows[0].price_update_time.is_empty());
    }

    #[test]
    fn parses_option_commodity_contract_sina() {
        let rows = parse_option_commodity_contract_sina(&load("option_commodity_au_o.html"), "option_commodity_contract_sina").unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].contract, "au2610");
        // The captured `#option_symbol .selected` text is the variety's display
        // name "黄金期权" (gold options), not the ticker "沪金".
        assert!(rows[0].symbol.contains("黄金"));
    }

    #[test]
    fn parses_option_commodity_contract_table_sina() {
        let rows = parse_option_commodity_contract_table_sina(&load("option_commodity_table_au_o.json"), "option_commodity_contract_table_sina").unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].call_code.starts_with("au2608C"));
        assert!(rows[0].strike.is_some());
        assert!(rows[0].put_code.contains("au2608"));
    }

    #[test]
    fn parses_option_margin_symbol() {
        let rows = parse_option_margin_symbol(&load("option_margin_symbol.html"), "option_margin_symbol").unwrap();
        assert!(!rows.is_empty());
        let oil = rows.iter().find(|r| r.symbol == "原油期权").expect("原油期权 missing");
        assert!(oil.url.contains("yuanyou"));
    }

    #[test]
    fn parses_option_margin() {
        let rows = parse_option_margin(&load("option_margin_yuanyou.html"), "option_margin").unwrap();
        assert!(!rows.is_empty());
        assert!(rows[0].underlying.contains("原油"));
        assert!(rows[0].settlement.is_some());
        assert!(!rows[0].update_time.is_empty());
    }
}
