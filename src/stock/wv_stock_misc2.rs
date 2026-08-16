//! 科创板 (Sina KCB) spot/daily, Eastmoney bid-ask, and A-share peer-comparison ports.
//!
//! | Rust function | akshare source | source / endpoint |
//! | --- | --- | --- |
//! | `stock_zh_kcb_spot` | `stock/stock_zh_kcb_sina.py:42` | Sina `Market_Center.getHQNodeData` (JSON array) |
//! | `stock_zh_kcb_daily` | `stock/stock_zh_kcb_sina.py:123` | Sina `KC_MarketDataService.getKLineData` + `getAmountBySymbol` |
//! | `stock_bid_ask_em` | `stock/stock_ask_bid_em.py:13` | Eastmoney push2 `stock/get` |
//! | `stock_zh_growth_comparison_em` | `stock/stock_zh_comparison_em.py:13` | Eastmoney datacenter `RPT_PCF10_INDUSTRY_GROWTH` |
//! | `stock_zh_dupont_comparison_em` | `stock/stock_zh_comparison_em.py:162` | Eastmoney datacenter `RPT_PCF10_INDUSTRY_DBFX` |
//!
//! Sina KCB history uses a JSONP-style response (`var _...=[...]`); we slice the
//! bracketed JSON array and parse it with `serde_json` (no `demjson`/JS needed).
//! The `qfq`/`hfq`/`*-factor` variants of `stock_zh_kcb_daily` are **DEFERRED**:
//! they require fetching Sina `hfq.js`/`qfq.js` files whose payload is a JS object
//! literal evaluated with `eval`/`demjson` — not a plain JSON GET.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY, SOURCE_SINA};
use crate::core::error::{Error, Result};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Extract `result.data` from an Eastmoney datacenter response.
fn em_data_array(resp: &Value) -> Result<Vec<Value>> {
    match resp.get("result") {
        Some(Value::Null) | None => Ok(Vec::new()),
        Some(r) => match r.get("data") {
            Some(Value::Null) | None => Ok(Vec::new()),
            Some(d) => d.as_array().cloned().ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "result.data is not an array".into(),
            }),
        },
    }
}

/// Read a string field (object value).
fn fstr(item: &Value, k: &str) -> Option<String> {
    item.get(k).and_then(|v| v.as_str()).map(str::to_string)
}

/// Read a numeric field, accepting either a JSON number or a numeric string.
fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// Fetch an Eastmoney F10 securities datacenter report. Returns the `result.data`
/// row array for the caller's parser.
#[allow(clippy::too_many_arguments)]
async fn em_sec_fetch(
    client: &Client,
    fn_name: &'static str,
    report_name: &str,
    columns: &str,
    filter: &str,
    sort_types: &str,
    sort_columns: &str,
    page_size: &str,
    v: &str,
) -> Result<Vec<Value>> {
    let mut params: Vec<(&str, &str)> = vec![
        ("reportName", report_name),
        ("columns", columns),
        ("quoteColumns", ""),
        ("filter", filter),
        ("pageNumber", "1"),
        ("pageSize", page_size),
        ("sortTypes", sort_types),
        ("sortColumns", sort_columns),
        ("source", "HSF10"),
        ("client", "PC"),
    ];
    if !v.is_empty() {
        params.push(("v", v));
    }
    let vv = client
        .get_json(SOURCE_EASTMONEY, fn_name, DC_SEC, &params)
        .await?;
    em_data_array(&vv)
}

/// Eastmoney F10 securities datacenter (the `RPT_PCF10_*` reports).
const DC_SEC: &str = "https://datacenter.eastmoney.com/securities/api/data/v1/get";

// ---------------------------------------------------------------------------
// stock_zh_kcb_spot  (stock/stock_zh_kcb_sina.py:42) — Sina JSON array
// ---------------------------------------------------------------------------

/// 东方财富-科创板-实时行情 (akshare `stock_zh_kcb_spot`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct KcbSpotRow {
    /// 代码 (index 0)
    pub code: String,
    /// 名称 (index 2)
    pub name: String,
    /// 最新价 (index 3)
    pub price: Option<f64>,
    /// 涨跌额 (index 4)
    pub change: Option<f64>,
    /// 涨跌幅 (index 5)
    pub pct_change: Option<f64>,
    /// 买入 (index 6)
    pub bid: Option<f64>,
    /// 卖出 (index 7)
    pub ask: Option<f64>,
    /// 昨收 (index 8)
    pub pre_close: Option<f64>,
    /// 今开 (index 9)
    pub open: Option<f64>,
    /// 最高 (index 10)
    pub high: Option<f64>,
    /// 最低 (index 11)
    pub low: Option<f64>,
    /// 成交量 (index 12)
    pub volume: Option<f64>,
    /// 成交额 (index 13)
    pub amount: Option<f64>,
    /// 时点 (index 14)
    pub timestamp: Option<String>,
    /// 市盈率 (index 15)
    pub pe: Option<f64>,
    /// 市净率 (index 16)
    pub pb: Option<f64>,
    /// 流通市值 (index 17)
    pub float_mv: Option<f64>,
    /// 总市值 (index 18)
    pub total_mv: Option<f64>,
    /// 换手率 (index 19)
    pub turnover: Option<f64>,
}

/// Sina KCB spot endpoint (returns a JSON array of positional rows).
const SINA_KCB_SPOT_URL: &str =
    "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeData";

/// Read a positional array element as `String`.
fn arr_str(item: &Value, idx: usize) -> Option<String> {
    item.get(idx).and_then(|v| v.as_str()).map(str::to_string)
}

/// Read a positional array element as `f64` (number or numeric string).
fn arr_num(item: &Value, idx: usize) -> Option<f64> {
    item.get(idx).and_then(|v| match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

/// Parse one Sina KCB spot row (positional JSON array).
fn parse_kcb_spot_item(item: &Value) -> KcbSpotRow {
    KcbSpotRow {
        code: arr_str(item, 0).unwrap_or_default(),
        name: arr_str(item, 2).unwrap_or_default(),
        price: arr_num(item, 3),
        change: arr_num(item, 4),
        pct_change: arr_num(item, 5),
        bid: arr_num(item, 6),
        ask: arr_num(item, 7),
        pre_close: arr_num(item, 8),
        open: arr_num(item, 9),
        high: arr_num(item, 10),
        low: arr_num(item, 11),
        volume: arr_num(item, 12),
        amount: arr_num(item, 13),
        timestamp: arr_str(item, 14),
        pe: arr_num(item, 15),
        pb: arr_num(item, 16),
        float_mv: arr_num(item, 17),
        total_mv: arr_num(item, 18),
        turnover: arr_num(item, 19),
    }
}

/// Parse the Sina KCB spot response (a JSON array of positional row arrays).
pub(crate) fn parse_kcb_spot(resp: &Value) -> Result<Vec<KcbSpotRow>> {
    let arr = resp.as_array().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "expected a JSON array".into(),
    })?;
    Ok(arr.iter().map(parse_kcb_spot_item).collect())
}

/// 新浪财经-科创板-实时行情数据 (akshare `stock_zh_kcb_spot`).
///
/// Sina paginates 80 rows/page; we walk pages until an empty page is returned.
pub async fn stock_zh_kcb_spot(client: &Client) -> Result<Vec<KcbSpotRow>> {
    let mut out = Vec::new();
    for page in 1..=100 {
        let page_s = page.to_string();
        let params: Vec<(&str, &str)> = vec![
            ("page", page_s.as_str()),
            ("num", "80"),
            ("sort", "symbol"),
            ("asc", "1"),
            ("node", "kcb"),
            ("symbol", ""),
            ("_s_r_a", "page"),
        ];
        let v = client
            .get_json(SOURCE_SINA, "stock_zh_kcb_spot", SINA_KCB_SPOT_URL, &params)
            .await?;
        let arr = v.as_array().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "expected a JSON array".into(),
        })?;
        if arr.is_empty() {
            break;
        }
        out.extend(parse_kcb_spot(&v)?);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_zh_kcb_daily  (stock/stock_zh_kcb_sina.py:123) — Sina K-line + amount
// ---------------------------------------------------------------------------

/// 东方财富-科创板-历史行情 (akshare `stock_zh_kcb_daily`, `adjust=""`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct KcbDailyRow {
    /// 日期 (index 0 of the kline row)
    pub date: String,
    /// 开盘 (index 1)
    pub open: Option<f64>,
    /// 最高 (index 2)
    pub high: Option<f64>,
    /// 最低 (index 3)
    pub low: Option<f64>,
    /// 收盘 (index 4)
    pub close: Option<f64>,
    /// 成交量 (index 5)
    pub volume: Option<f64>,
    /// 后成交量 (index 6, if present)
    pub after_volume: Option<f64>,
    /// 后成交额 (index 7, if present)
    pub after_amount: Option<f64>,
    /// 流通股本 (index 8, if present)
    pub outstanding_share: Option<f64>,
    /// 成交额 (from `getAmountBySymbol`, ×1e4)
    pub amount: Option<f64>,
    /// 换手率 (volume / amount)
    pub turnover: Option<f64>,
}

/// Sina KCB kline (JSONP `var _...=[...]`) and amount endpoints. `ZZSYMZZ` is a
/// placeholder replaced (twice) with the stock symbol in `stock_zh_kcb_daily`.
const SINA_KCB_HIST_URL: &str =
    "https://quotes.sina.cn/cn/api/jsonp.php/var%20_ZZSYMZZ=KC_MarketDataService.getKLineData?symbol=ZZSYMZZ";
const SINA_KCB_AMOUNT_URL: &str =
    "https://stock.finance.sina.com.cn/stock/api/jsonp.php/var%20KKE_ShareAmount_ZZSYMZZ=/StockService.getAmountBySymbol?_=20&symbol=ZZSYMZZ";

/// Slice the bracketed JSON array out of a Sina JSONP response body.
fn slice_json_array(text: &str) -> Result<Vec<Value>> {
    let start = text.find('[').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "no `[` in Sina JSONP response".into(),
    })?;
    let end = text.rfind(']').ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "no `]` in Sina JSONP response".into(),
    })?;
    let arr: Value = serde_json::from_str(&text[start..=end]).map_err(Error::Json)?;
    arr.as_array().cloned().ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_SINA,
        message: "sliced body is not a JSON array".into(),
    })
}

/// Parse `stock_zh_kcb_daily` from the raw kline rows and amount rows.
pub(crate) fn parse_kcb_daily(kline: &[Value], amount: &[Value]) -> Vec<KcbDailyRow> {
    // amount: list of [date, amount]; index by date.
    let mut amap: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for a in amount {
        let Some(d) = arr_str(a, 0) else { continue };
        if let Some(v) = arr_num(a, 1) {
            amap.insert(d, v * 1e4);
        }
    }
    let mut out = Vec::with_capacity(kline.len());
    for row in kline {
        let Some(date) = arr_str(row, 0) else { continue };
        let volume = arr_num(row, 5);
        let amount_val = amap.get(&date).copied();
        let turnover = match (volume, amount_val) {
            (Some(v), Some(a)) if a != 0.0 => Some(v / a),
            _ => None,
        };
        out.push(KcbDailyRow {
            date,
            open: arr_num(row, 1),
            high: arr_num(row, 2),
            low: arr_num(row, 3),
            close: arr_num(row, 4),
            volume,
            after_volume: row.get(6).and_then(|_| arr_num(row, 6)),
            after_amount: row.get(7).and_then(|_| arr_num(row, 7)),
            outstanding_share: row.get(8).and_then(|_| arr_num(row, 8)),
            amount: amount_val,
            turnover,
        });
    }
    out
}

/// 新浪财经-科创板-历史行情数据 (akshare `stock_zh_kcb_daily`).
///
/// Only `adjust=""` (no forward/backward adjustment) is supported here; the
/// `qfq`/`hfq`/`*-factor` variants require parsing Sina `hfq.js`/`qfq.js` JS
/// object literals (DEFERRED — not a plain JSON GET).
pub async fn stock_zh_kcb_daily(
    client: &Client,
    symbol: &str,
    adjust: &str,
) -> Result<Vec<KcbDailyRow>> {
    if !adjust.is_empty() {
        return Err(Error::InvalidParam(format!(
            "stock_zh_kcb_daily adjust=`{adjust}` is deferred (needs Sina hfq.js/qfq.js JS parse)"
        )));
    }
    let hist_url = SINA_KCB_HIST_URL.replace("ZZSYMZZ", symbol);
    let amount_url = SINA_KCB_AMOUNT_URL.replace("ZZSYMZZ", symbol);
    let hist_txt = client
        .get_text(SOURCE_SINA, "stock_zh_kcb_daily", &hist_url, &[], None)
        .await?;
    let amount_txt = client
        .get_text(SOURCE_SINA, "stock_zh_kcb_daily", &amount_url, &[], None)
        .await?;
    let kline = slice_json_array(&hist_txt)?;
    let amount = slice_json_array(&amount_txt)?;
    Ok(parse_kcb_daily(&kline, &amount))
}

// ---------------------------------------------------------------------------
// stock_bid_ask_em  (stock/stock_ask_bid_em.py:13) — Eastmoney push2 stock/get
// ---------------------------------------------------------------------------

/// 东方财富-行情报价 (akshare `stock_bid_ask_em`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BidAskRow {
    /// 卖五 (f31)
    pub sell_5: Option<f64>,
    /// 卖五量 (f32 × 100)
    pub sell_5_vol: Option<f64>,
    /// 卖四 (f33)
    pub sell_4: Option<f64>,
    /// 卖四量 (f34 × 100)
    pub sell_4_vol: Option<f64>,
    /// 卖三 (f35)
    pub sell_3: Option<f64>,
    /// 卖三量 (f36 × 100)
    pub sell_3_vol: Option<f64>,
    /// 卖二 (f37)
    pub sell_2: Option<f64>,
    /// 卖二量 (f38 × 100)
    pub sell_2_vol: Option<f64>,
    /// 卖一 (f39)
    pub sell_1: Option<f64>,
    /// 卖一量 (f40 × 100)
    pub sell_1_vol: Option<f64>,
    /// 买一 (f19)
    pub buy_1: Option<f64>,
    /// 买一量 (f20 × 100)
    pub buy_1_vol: Option<f64>,
    /// 买二 (f17)
    pub buy_2: Option<f64>,
    /// 买二量 (f18 × 100)
    pub buy_2_vol: Option<f64>,
    /// 买三 (f15)
    pub buy_3: Option<f64>,
    /// 买三量 (f16 × 100)
    pub buy_3_vol: Option<f64>,
    /// 买四 (f13)
    pub buy_4: Option<f64>,
    /// 买四量 (f14 × 100)
    pub buy_4_vol: Option<f64>,
    /// 买五 (f11)
    pub buy_5: Option<f64>,
    /// 买五量 (f12 × 100)
    pub buy_5_vol: Option<f64>,
    /// 最新 (f43)
    pub latest: Option<f64>,
    /// 均价 (f71)
    pub avg_price: Option<f64>,
    /// 涨幅 (f170)
    pub pct: Option<f64>,
    /// 涨跌 (f169)
    pub change: Option<f64>,
    /// 总手 (f47)
    pub total_hand: Option<f64>,
    /// 金额 (f48)
    pub amount: Option<f64>,
    /// 换手 (f168)
    pub turnover: Option<f64>,
    /// 量比 (f50)
    pub vol_ratio: Option<f64>,
    /// 最高 (f44)
    pub high: Option<f64>,
    /// 最低 (f45)
    pub low: Option<f64>,
    /// 今开 (f46)
    pub open: Option<f64>,
    /// 昨收 (f60)
    pub pre_close: Option<f64>,
    /// 涨停 (f51)
    pub limit_up: Option<f64>,
    /// 跌停 (f52)
    pub limit_down: Option<f64>,
    /// 外盘 (f49)
    pub outer: Option<f64>,
    /// 内盘 (f161)
    pub inner: Option<f64>,
}

/// Eastmoney push2 quote endpoint (used by `stock_bid_ask_em`).
const PUSH2_STOCK: &str = "https://push2.eastmoney.com/api/qt/stock/get";

/// Parse `stock_bid_ask_em` from a push2 `stock/get` response (`data` object).
pub(crate) fn parse_bid_ask(resp: &Value) -> Result<BidAskRow> {
    let data = resp
        .get("data")
        .filter(|v| !v.is_null())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data".into(),
        })?;
    let num = |k: &str| fnum(data, k);
    let vol = |k: &str| fnum(data, k).map(|x| x * 100.0);
    Ok(BidAskRow {
        sell_5: num("f31"),
        sell_5_vol: vol("f32"),
        sell_4: num("f33"),
        sell_4_vol: vol("f34"),
        sell_3: num("f35"),
        sell_3_vol: vol("f36"),
        sell_2: num("f37"),
        sell_2_vol: vol("f38"),
        sell_1: num("f39"),
        sell_1_vol: vol("f40"),
        buy_1: num("f19"),
        buy_1_vol: vol("f20"),
        buy_2: num("f17"),
        buy_2_vol: vol("f18"),
        buy_3: num("f15"),
        buy_3_vol: vol("f16"),
        buy_4: num("f13"),
        buy_4_vol: vol("f14"),
        buy_5: num("f11"),
        buy_5_vol: vol("f12"),
        latest: num("f43"),
        avg_price: num("f71"),
        pct: num("f170"),
        change: num("f169"),
        total_hand: num("f47"),
        amount: num("f48"),
        turnover: num("f168"),
        vol_ratio: num("f50"),
        high: num("f44"),
        low: num("f45"),
        open: num("f46"),
        pre_close: num("f60"),
        limit_up: num("f51"),
        limit_down: num("f52"),
        outer: num("f49"),
        inner: num("f161"),
    })
}

/// 东方财富-行情报价 (akshare `stock_bid_ask_em`).
pub async fn stock_bid_ask_em(client: &Client, symbol: &str) -> Result<Vec<BidAskRow>> {
    let market_code = if symbol.starts_with('6') { "1" } else { "0" };
    let secid = format!("{market_code}.{symbol}");
    let fields = "f120,f121,f122,f174,f175,f59,f163,f43,f57,f58,f169,f170,f46,f44,f51,\
                  f168,f47,f164,f116,f60,f45,f52,f50,f48,f167,f117,f71,f161,f49,f530,\
                  f135,f136,f137,f138,f139,f141,f142,f144,f145,f147,f148,f140,f143,f146,\
                  f149,f55,f62,f162,f92,f173,f104,f105,f84,f85,f183,f184,f185,f186,f187,\
                  f188,f189,f190,f191,f192,f107,f111,f86,f177,f78,f110,f262,f263,f264,f267,\
                  f268,f255,f256,f257,f258,f127,f199,f128,f198,f259,f260,f261,f171,f277,f278,\
                  f279,f288,f152,f250,f251,f252,f253,f254,f269,f270,f271,f272,f273,f274,f275,\
                  f276,f265,f266,f289,f290,f286,f285,f292,f293,f294,f295";
    let params: Vec<(&str, &str)> = vec![
        ("fltt", "2"),
        ("invt", "2"),
        ("fields", fields),
        ("secid", secid.as_str()),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "stock_bid_ask_em", PUSH2_STOCK, &params)
        .await?;
    Ok(vec![parse_bid_ask(&v)?])
}

// ---------------------------------------------------------------------------
// stock_zh_growth_comparison_em  (stock/stock_zh_comparison_em.py:13)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct GrowthComparisonZhRow {
    /// 代码 (CORRE_SECURITY_CODE)
    pub code: String,
    /// 简称 (CORRE_SECURITY_NAME)
    pub name: Option<String>,
    /// 基本每股收益增长率-3年复合 (MGSY_3Y)
    pub mgsy_3y: Option<f64>,
    /// 基本每股收益增长率-24A (MGSYTB)
    pub mgsy_24a: Option<f64>,
    /// 基本每股收益增长率-TTM (MGSYTTM)
    pub mgsy_ttm: Option<f64>,
    /// 基本每股收益增长率-25E (MGSY_1E)
    pub mgsy_1e: Option<f64>,
    /// 基本每股收益增长率-26E (MGSY_2E)
    pub mgsy_2e: Option<f64>,
    /// 基本每股收益增长率-27E (MGSY_3E)
    pub mgsy_3e: Option<f64>,
    /// 营业收入增长率-3年复合 (YYSR_3Y)
    pub yysr_3y: Option<f64>,
    /// 营业收入增长率-24A (YYSRTB)
    pub yysr_24a: Option<f64>,
    /// 营业收入增长率-TTM (YYSRTTM)
    pub yysr_ttm: Option<f64>,
    /// 营业收入增长率-25E (YYSR_1E)
    pub yysr_1e: Option<f64>,
    /// 营业收入增长率-26E (YYSR_2E)
    pub yysr_2e: Option<f64>,
    /// 营业收入增长率-27E (YYSR_3E)
    pub yysr_3e: Option<f64>,
    /// 净利润增长率-3年复合 (JLR_3Y)
    pub jlr_3y: Option<f64>,
    /// 净利润增长率-24A (JLRTB)
    pub jlr_24a: Option<f64>,
    /// 净利润增长率-TTM (JLRTTM)
    pub jlr_ttm: Option<f64>,
    /// 净利润增长率-25E (JLR_1E)
    pub jlr_1e: Option<f64>,
    /// 净利润增长率-26E (JLR_2E)
    pub jlr_2e: Option<f64>,
    /// 净利润增长率-27E (JLR_3E)
    pub jlr_3e: Option<f64>,
    /// 基本每股收益增长率-3年复合排名 (PAIMING)
    pub paiming: Option<f64>,
}

/// Parse `stock_zh_growth_comparison_em` rows from a datacenter response.
pub(crate) fn parse_growth_comparison_zh(items: &[Value]) -> Vec<GrowthComparisonZhRow> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(code) = fstr(item, "CORRE_SECURITY_CODE") else {
            continue;
        };
        out.push(GrowthComparisonZhRow {
            code,
            name: fstr(item, "CORRE_SECURITY_NAME"),
            mgsy_3y: fnum(item, "MGSY_3Y"),
            mgsy_24a: fnum(item, "MGSYTB"),
            mgsy_ttm: fnum(item, "MGSYTTM"),
            mgsy_1e: fnum(item, "MGSY_1E"),
            mgsy_2e: fnum(item, "MGSY_2E"),
            mgsy_3e: fnum(item, "MGSY_3E"),
            yysr_3y: fnum(item, "YYSR_3Y"),
            yysr_24a: fnum(item, "YYSRTB"),
            yysr_ttm: fnum(item, "YYSRTTM"),
            yysr_1e: fnum(item, "YYSR_1E"),
            yysr_2e: fnum(item, "YYSR_2E"),
            yysr_3e: fnum(item, "YYSR_3E"),
            jlr_3y: fnum(item, "JLR_3Y"),
            jlr_24a: fnum(item, "JLRTB"),
            jlr_ttm: fnum(item, "JLRTTM"),
            jlr_1e: fnum(item, "JLR_1E"),
            jlr_2e: fnum(item, "JLR_2E"),
            jlr_3e: fnum(item, "JLR_3E"),
            paiming: fnum(item, "PAIMING"),
        });
    }
    out
}

/// 东方财富-行情中心-同行比较-成长性比较 (akshare `stock_zh_growth_comparison_em`).
///
/// `symbol` is an akshare-style code like `"SZ000895"`; the secid filter is
/// built as `000895.SZ` (code after the 2-char market prefix).
pub async fn stock_zh_growth_comparison_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<GrowthComparisonZhRow>> {
    let (market, code) = symbol.split_at(2);
    let filter = format!(r#"(SECUCODE="{code}.{market}")"#);
    let data = em_sec_fetch(
        client,
        "stock_zh_growth_comparison_em",
        "RPT_PCF10_INDUSTRY_GROWTH",
        "ALL",
        &filter,
        "1",
        "PAIMING",
        "",
        "02747607708067783",
    )
    .await?;
    Ok(parse_growth_comparison_zh(&data))
}

// ---------------------------------------------------------------------------
// stock_zh_dupont_comparison_em  (stock/stock_zh_comparison_em.py:162)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct DupontComparisonZhRow {
    /// 代码 (CORRE_SECURITY_CODE)
    pub code: String,
    /// 简称 (CORRE_SECURITY_NAME)
    pub name: Option<String>,
    /// ROE-3年平均 (ROE_AVG)
    pub roe_avg: Option<f64>,
    /// ROE-22A (ROEPJ_L3)
    pub roe_l3: Option<f64>,
    /// ROE-23A (ROEPJ_L2)
    pub roe_l2: Option<f64>,
    /// ROE-24A (ROEPJ_L1)
    pub roe_l1: Option<f64>,
    /// 净利率-3年平均 (XSJLL_AVG)
    pub xsjll_avg: Option<f64>,
    /// 净利率-22A (XSJLL_L3)
    pub xsjll_l3: Option<f64>,
    /// 净利率-23A (XSJLL_L2)
    pub xsjll_l2: Option<f64>,
    /// 净利率-24A (XSJLL_L1)
    pub xsjll_l1: Option<f64>,
    /// 总资产周转率-3年平均 (TOAZZL_AVG)
    pub toazzl_avg: Option<f64>,
    /// 总资产周转率-22A (TOAZZL_L3)
    pub toazzl_l3: Option<f64>,
    /// 总资产周转率-23A (TOAZZL_L2)
    pub toazzl_l2: Option<f64>,
    /// 总资产周转率-24A (TOAZZL_L1)
    pub toazzl_l1: Option<f64>,
    /// 权益乘数-3年平均 (QYCS_AVG)
    pub qycs_avg: Option<f64>,
    /// 权益乘数-22A (QYCS_L3)
    pub qycs_l3: Option<f64>,
    /// 权益乘数-23A (QYCS_L2)
    pub qycs_l2: Option<f64>,
    /// 权益乘数-24A (QYCS_L1)
    pub qycs_l1: Option<f64>,
    /// ROE-3年平均排名 (PAIMING)
    pub paiming: Option<f64>,
}

/// Parse `stock_zh_dupont_comparison_em` rows from a datacenter response.
pub(crate) fn parse_dupont_comparison_zh(items: &[Value]) -> Vec<DupontComparisonZhRow> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(code) = fstr(item, "CORRE_SECURITY_CODE") else {
            continue;
        };
        out.push(DupontComparisonZhRow {
            code,
            name: fstr(item, "CORRE_SECURITY_NAME"),
            roe_avg: fnum(item, "ROE_AVG"),
            roe_l3: fnum(item, "ROEPJ_L3"),
            roe_l2: fnum(item, "ROEPJ_L2"),
            roe_l1: fnum(item, "ROEPJ_L1"),
            xsjll_avg: fnum(item, "XSJLL_AVG"),
            xsjll_l3: fnum(item, "XSJLL_L3"),
            xsjll_l2: fnum(item, "XSJLL_L2"),
            xsjll_l1: fnum(item, "XSJLL_L1"),
            toazzl_avg: fnum(item, "TOAZZL_AVG"),
            toazzl_l3: fnum(item, "TOAZZL_L3"),
            toazzl_l2: fnum(item, "TOAZZL_L2"),
            toazzl_l1: fnum(item, "TOAZZL_L1"),
            qycs_avg: fnum(item, "QYCS_AVG"),
            qycs_l3: fnum(item, "QYCS_L3"),
            qycs_l2: fnum(item, "QYCS_L2"),
            qycs_l1: fnum(item, "QYCS_L1"),
            paiming: fnum(item, "PAIMING"),
        });
    }
    out
}

/// 东方财富-行情中心-同行比较-杜邦分析比较 (akshare `stock_zh_dupont_comparison_em`).
pub async fn stock_zh_dupont_comparison_em(
    client: &Client,
    symbol: &str,
) -> Result<Vec<DupontComparisonZhRow>> {
    let (market, code) = symbol.split_at(2);
    let filter = format!(r#"(SECUCODE="{code}.{market}")"#);
    let data = em_sec_fetch(
        client,
        "stock_zh_dupont_comparison_em",
        "RPT_PCF10_INDUSTRY_DBFX",
        "ALL",
        &filter,
        "1",
        "PAIMING",
        "",
        "05086361194054821",
    )
    .await?;
    Ok(parse_dupont_comparison_zh(&data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    fn approx(a: Option<f64>, b: f64) -> bool {
        match a {
            Some(x) => (x - b).abs() < 1e-6,
            None => false,
        }
    }

    // ---- Sina KCB spot (JSON array of arrays) ----

    #[test]
    fn parse_kcb_spot_ok() {
        let rows = parse_kcb_spot(&fixture("stock_zh_kcb_spot.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "688399");
        assert_eq!(rows[0].name, "硕世生物");
        assert!(approx(rows[0].price, 12.34));
        assert!(approx(rows[0].pct_change, -4.32));
        assert!(approx(rows[0].bid, 12.30));
        assert!(approx(rows[0].amount, 1.25e8));
        assert_eq!(rows[0].timestamp, Some("2024-05-10 16:00:00".into()));
        assert!(approx(rows[1].total_mv, 1.2e10));
    }

    // ---- Sina KCB daily (kline array + amount array) ----

    #[test]
    fn parse_kcb_daily_ok() {
        let v = fixture("stock_zh_kcb_daily.json");
        let kline = v.get("kline").and_then(|x| x.as_array()).unwrap();
        let amount = v.get("amount").and_then(|x| x.as_array()).unwrap();
        let rows = parse_kcb_daily(kline, amount);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert!(approx(rows[0].open, 12.30));
        assert!(approx(rows[0].close, 12.50));
        // amount raw 12500 * 1e4 = 125000000
        assert!(approx(rows[0].amount, 125000000.0));
        // turnover = volume / amount = 1000000 / 125000000
        assert!(approx(rows[0].turnover, 0.008));
        assert!(approx(rows[1].high, 13.10));
    }

    // ---- Eastmoney bid/ask ----

    #[test]
    fn parse_bid_ask_ok() {
        let row = parse_bid_ask(&fixture("stock_bid_ask_em.json")).unwrap();
        assert!(approx(row.sell_1, 10.58));
        // volume fields are ×100
        assert!(approx(row.sell_1_vol, 1100.0 * 100.0));
        assert!(approx(row.buy_1, 10.45));
        assert!(approx(row.buy_1_vol, 800.0 * 100.0));
        assert!(approx(row.latest, 10.48));
        assert!(approx(row.pre_close, 10.30));
        assert!(approx(row.limit_up, 11.33));
        assert!(approx(row.limit_down, 9.27));
    }

    // ---- Eastmoney A-share peer comparison (datacenter) ----

    #[test]
    fn parse_growth_comparison_zh_ok() {
        let rows = parse_growth_comparison_zh(
            &em_data_array(&fixture("stock_zh_growth_comparison_em.json")).unwrap(),
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000895.SZ");
        assert_eq!(rows[0].name, Some("双汇发展".into()));
        assert!(approx(rows[0].mgsy_3y, 8.5));
        assert!(approx(rows[0].paiming, 3.0));
        assert!(approx(rows[1].jlr_ttm, 7.7));
    }

    #[test]
    fn parse_dupont_comparison_zh_ok() {
        let rows = parse_dupont_comparison_zh(
            &em_data_array(&fixture("stock_zh_dupont_comparison_em.json")).unwrap(),
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000895.SZ");
        assert!(approx(rows[0].roe_avg, 26.5));
        assert!(approx(rows[0].xsjll_l1, 9.8));
        assert!(approx(rows[0].qycs_avg, 1.6));
        assert!(approx(rows[1].paiming, 4.0));
    }
}
