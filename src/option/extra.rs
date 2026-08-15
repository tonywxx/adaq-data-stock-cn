//! Extra option-market endpoints (akshare `option` package, assorted).
//!
//! Ports of akshare option functions that are pure HTTP (no JS signing,
//! encryption, or HTML scraping): Eastmoney option-chain/list endpoints and
//! Sina/SSE option quote, contract, risk and statistics endpoints.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY, SOURCE_SINA};
use crate::core::error::{Error, Result};

/// SSE source identifier (no dedicated constant in `core::client`).
const SOURCE_SSE: &str = "sse";

// ---------------------------------------------------------------------------
// Eastmoney: current option chain (akshare `option_current_em`)
// ---------------------------------------------------------------------------

/// A single contract in Eastmoney's option-chain listing.
///
/// Mirrors akshare `option_current_em`. Field suffixes note the Eastmoney
/// `f`-code columns used upstream (akshare column in parentheses).
#[derive(Debug, Clone, serde::Serialize)]
pub struct EmOptionCurrentRow {
    /// 序号 (f1)
    pub rank: Option<f64>,
    /// 代码 (f12)
    pub code: String,
    /// 市场标识 (f13)
    pub market: String,
    /// 名称 (f14)
    pub name: String,
    /// 最新价 (f2)
    pub price: Option<f64>,
    /// 涨跌幅 (f3)
    pub pct_change: Option<f64>,
    /// 涨跌额 (f4)
    pub change: Option<f64>,
    /// 成交量 (f5)
    pub volume: Option<f64>,
    /// 成交额 (f6)
    pub amount: Option<f64>,
    /// 今开 (f17)
    pub open: Option<f64>,
    /// 持仓量 (f62)
    pub open_interest: Option<f64>,
    /// 行权价 (f161)
    pub exercise_price: Option<f64>,
    /// 剩余日 (f162)
    pub remaining_days: Option<f64>,
    /// 日增 (f163)
    pub oi_change: Option<f64>,
    pub source: &'static str,
}

/// Current option-chain listing from Eastmoney's `push2` clist API
/// (akshare `option_current_em`).
///
/// Fetches the first page (`pz=100`). `fs` selects all option markets
/// (m:10/12/140/141/151/163/226).
pub async fn option_current_em(client: &Client) -> Result<Vec<EmOptionCurrentRow>> {
    let params = [
        ("pn", "1"),
        ("pz", "100"),
        ("po", "1"),
        ("np", "1"),
        ("ut", "bd1d9ddb04089700cf9c27f6f7426281"),
        ("fltt", "2"),
        ("invt", "2"),
        ("fid", "f3"),
        ("fs", "m:10,m:12,m:140,m:141,m:151,m:163,m:226"),
        (
            "fields",
            "f1,f2,f3,f4,f5,f6,f12,f13,f14,f17,f62,f161,f162,f163",
        ),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "option_current_em",
            "https://23.push2.eastmoney.com/api/qt/clist/get",
            &params,
        )
        .await?;
    parse_current_em(&v)
}

pub(crate) fn parse_current_em(resp: &Value) -> Result<Vec<EmOptionCurrentRow>> {
    let diff = resp
        .get("data")
        .and_then(|d| d.get("diff"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })?;
    let mut out = Vec::with_capacity(diff.len());
    for item in diff {
        out.push(EmOptionCurrentRow {
            rank: num(item.get("f1")),
            code: as_string(item, "f12"),
            market: as_string(item, "f13"),
            name: as_string(item, "f14"),
            price: num(item.get("f2")),
            pct_change: num(item.get("f3")),
            change: num(item.get("f4")),
            volume: num(item.get("f5")),
            amount: num(item.get("f6")),
            open: num(item.get("f17")),
            open_interest: num(item.get("f62")),
            exercise_price: num(item.get("f161")),
            remaining_days: num(item.get("f162")),
            oi_change: num(item.get("f163")),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Eastmoney: CFFEX option list (akshare `option_current_cffex_em`)
// ---------------------------------------------------------------------------

/// A single CFFEX option contract from Eastmoney's `futsseapi` list.
///
/// Mirrors akshare `option_current_cffex_em`. Field suffixes note the upstream
/// `field` codes (akshare column in parentheses).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CffexOptionCurrentRow {
    /// 序号 (derived, 1-based)
    pub rank: Option<f64>,
    /// 代码 (dm)
    pub code: String,
    /// 市场标识 (sc)
    pub market: String,
    /// 名称 (name)
    pub name: String,
    /// 最新价 (p)
    pub price: Option<f64>,
    /// 涨跌额 (zde)
    pub change: Option<f64>,
    /// 涨跌幅 (zdf)
    pub pct_change: Option<f64>,
    /// 成交量 (vol)
    pub volume: Option<f64>,
    /// 成交额 (cje)
    pub amount: Option<f64>,
    /// 今开 (o)
    pub open: Option<f64>,
    /// 持仓量 (ccl)
    pub open_interest: Option<f64>,
    /// 行权价 (xqj)
    pub exercise_price: Option<f64>,
    /// 剩余日 (syr)
    pub remaining_days: Option<f64>,
    /// 日增 (rz)
    pub oi_change: Option<f64>,
    /// 昨结 (zjsj)
    pub prev_settle: Option<f64>,
    pub source: &'static str,
}

/// CFFEX option listing from Eastmoney's `futsseapi` list API
/// (akshare `option_current_cffex_em`).
///
/// The response is JSONP-wrapped (`blockName=callback`), so the wrapper is
/// stripped before parsing.
pub async fn option_current_cffex_em(client: &Client) -> Result<Vec<CffexOptionCurrentRow>> {
    let params = [
        ("orderBy", "zdf"),
        ("sort", "desc"),
        ("pageSize", "20000"),
        ("pageIndex", "0"),
        ("token", "58b2fa8f54638b60b87d69b31969089c"),
        (
            "field",
            "dm,sc,name,p,zsjd,zde,zdf,f152,vol,cje,ccl,xqj,syr,rz,zjsj,o",
        ),
        ("blockName", "callback"),
    ];
    let text = client
        .get_text(
            SOURCE_EASTMONEY,
            "option_current_cffex_em",
            "https://futsseapi.eastmoney.com/list/option/221",
            &params,
            None,
        )
        .await?;
    let v = to_value(&text)?;
    parse_current_cffex_em(&v)
}

pub(crate) fn parse_current_cffex_em(resp: &Value) -> Result<Vec<CffexOptionCurrentRow>> {
    let list = resp
        .get("list")
        .and_then(|l| l.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing list".into(),
        })?;
    let mut out = Vec::with_capacity(list.len());
    for (i, item) in list.iter().enumerate() {
        out.push(CffexOptionCurrentRow {
            rank: Some((i + 1) as f64),
            code: as_string(item, "dm"),
            market: as_string(item, "sc"),
            name: as_string(item, "name"),
            price: num(item.get("p")),
            change: num(item.get("zde")),
            pct_change: num(item.get("zdf")),
            volume: num(item.get("vol")),
            amount: num(item.get("cje")),
            open: num(item.get("o")),
            open_interest: num(item.get("ccl")),
            exercise_price: num(item.get("xqj")),
            remaining_days: num(item.get("syr")),
            oi_change: num(item.get("rz")),
            prev_settle: num(item.get("zjsj")),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// SSE: risk indicators (akshare `option_risk_indicator_sse`)
// ---------------------------------------------------------------------------

/// One options contract's Greek/risk indicators for a trading day (SSE).
///
/// Mirrors akshare `option_risk_indicator_sse`. Field suffixes note the SSE
/// `result` keys.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseRiskIndicatorRow {
    /// TRADE_DATE 交易日
    pub trade_date: String,
    /// SECURITY_ID 合约编码
    pub security_id: String,
    /// CONTRACT_ID 合约交易代码
    pub contract_id: String,
    /// CONTRACT_SYMBOL 合约简称
    pub contract_symbol: String,
    /// DELTA_VALUE
    pub delta: Option<f64>,
    /// THETA_VALUE
    pub theta: Option<f64>,
    /// GAMMA_VALUE
    pub gamma: Option<f64>,
    /// VEGA_VALUE
    pub vega: Option<f64>,
    /// RHO_VALUE
    pub rho: Option<f64>,
    /// IMPLC_VOLATLTY 隐含波动率
    pub implied_vol: Option<f64>,
    pub source: &'static str,
}

/// Options risk indicators for a date from SSE `commonQuery.do`
/// (akshare `option_risk_indicator_sse`).
///
/// `date` is `YYYYMMDD` (e.g. `"20240626"`), starting 20150209.
pub async fn option_risk_indicator_sse(
    client: &Client,
    date: &str,
) -> Result<Vec<SseRiskIndicatorRow>> {
    let params = [
        ("isPagination", "false"),
        ("trade_date", date),
        ("sqlId", "SSE_ZQPZ_YSP_GGQQZSXT_YSHQ_QQFXZB_DATE_L"),
        ("contractSymbol", ""),
    ];
    let headers = [("Referer", "https://www.sse.com.cn/")];
    let text = client
        .get_text(
            SOURCE_SSE,
            "option_risk_indicator_sse",
            "http://query.sse.com.cn/commonQuery.do",
            &params,
            Some(&headers),
        )
        .await?;
    let v: Value = serde_json::from_str(&text).map_err(Error::Json)?;
    parse_risk_indicator_sse(&v)
}

pub(crate) fn parse_risk_indicator_sse(resp: &Value) -> Result<Vec<SseRiskIndicatorRow>> {
    let result = resp
        .get("result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SSE,
            message: "missing result".into(),
        })?;
    let mut out = Vec::with_capacity(result.len());
    for item in result {
        out.push(SseRiskIndicatorRow {
            trade_date: as_string(item, "TRADE_DATE"),
            security_id: as_string(item, "SECURITY_ID"),
            contract_id: as_string(item, "CONTRACT_ID"),
            contract_symbol: as_string(item, "CONTRACT_SYMBOL"),
            delta: num(item.get("DELTA_VALUE")),
            theta: num(item.get("THETA_VALUE")),
            gamma: num(item.get("GAMMA_VALUE")),
            vega: num(item.get("VEGA_VALUE")),
            rho: num(item.get("RHO_VALUE")),
            implied_vol: num(item.get("IMPLC_VOLATLTY")),
            source: SOURCE_SSE,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// SSE: current-day contracts (akshare `option_current_day_sse`)
// ---------------------------------------------------------------------------

/// One currently-listed SSE option contract (disclosure, current day).
///
/// Mirrors akshare `option_current_day_sse`. Field suffixes note the SSE
/// `result` keys.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseContractRow {
    /// SECURITY_ID 合约编码
    pub security_id: String,
    /// CONTRACT_ID 合约交易代码
    pub contract_id: String,
    /// CONTRACT_SYMBOL 合约简称
    pub contract_symbol: String,
    /// SECURITYNAMEBYID 标的券名称及代码
    pub underlying: String,
    /// CALL_OR_PUT 类型 (认购/认沽)
    pub call_or_put: String,
    /// EXERCISE_PRICE 行权价
    pub exercise_price: Option<f64>,
    /// CONTRACT_UNIT 合约单位
    pub contract_unit: Option<f64>,
    /// END_DATE 期权行权日
    pub end_date: String,
    /// DELIVERY_DATE 行权交收日
    pub delivery_date: String,
    /// EXPIRE_DATE 到期日
    pub expire_date: String,
    /// START_DATE 开始日期
    pub start_date: String,
    pub source: &'static str,
}

/// Currently-listed SSE option contracts from SSE `commonQuery.do`
/// (akshare `option_current_day_sse`).
pub async fn option_current_day_sse(client: &Client) -> Result<Vec<SseContractRow>> {
    let params = [
        ("isPagination", "false"),
        ("expireDate", ""),
        ("securityId", ""),
        ("sqlId", "SSE_ZQPZ_YSP_GGQQZSXT_XXPL_DRHY_SEARCH_L"),
    ];
    let headers = [("Referer", "https://www.sse.com.cn/")];
    let text = client
        .get_text(
            SOURCE_SSE,
            "option_current_day_sse",
            "http://query.sse.com.cn/commonQuery.do",
            &params,
            Some(&headers),
        )
        .await?;
    let v: Value = serde_json::from_str(&text).map_err(Error::Json)?;
    parse_current_day_sse(&v)
}

pub(crate) fn parse_current_day_sse(resp: &Value) -> Result<Vec<SseContractRow>> {
    let result = resp
        .get("result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SSE,
            message: "missing result".into(),
        })?;
    let mut out = Vec::with_capacity(result.len());
    for item in result {
        out.push(SseContractRow {
            security_id: as_string(item, "SECURITY_ID"),
            contract_id: as_string(item, "CONTRACT_ID"),
            contract_symbol: as_string(item, "CONTRACT_SYMBOL"),
            underlying: as_string(item, "SECURITYNAMEBYID"),
            call_or_put: as_string(item, "CALL_OR_PUT"),
            exercise_price: num(item.get("EXERCISE_PRICE")),
            contract_unit: num(item.get("CONTRACT_UNIT")),
            end_date: as_string(item, "END_DATE"),
            delivery_date: as_string(item, "DELIVERY_DATE"),
            expire_date: as_string(item, "EXPIRE_DATE"),
            start_date: as_string(item, "START_DATE"),
            source: SOURCE_SSE,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// SSE: daily statistics (akshare `option_daily_stats_sse`)
// ---------------------------------------------------------------------------

/// Per-underlying daily options statistics for the SSE.
///
/// Mirrors akshare `option_daily_stats_sse`. Field suffixes note the SSE
/// `result` keys. Numeric values arrive with thousands separators, stripped
/// before parsing.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SseDailyStatsRow {
    /// SECURITY_CODE 合约标的代码
    pub security_code: String,
    /// SECURITY_ABBR 合约标的名称
    pub security_abbr: String,
    /// CONTRACT_VOLUME 合约数量
    pub contract_volume: Option<f64>,
    /// TOTAL_MONEY 总成交额
    pub total_money: Option<f64>,
    /// TOTAL_VOLUME 总成交量
    pub total_volume: Option<f64>,
    /// CALL_VOLUME 认购成交量
    pub call_volume: Option<f64>,
    /// PUT_VOLUME 认沽成交量
    pub put_volume: Option<f64>,
    /// CP_RATE 认沽/认购
    pub cp_rate: Option<f64>,
    /// LEAVES_QTY 未平仓合约总数
    pub leaves_qty: Option<f64>,
    /// LEAVES_CALL_QTY 未平仓认购合约数
    pub leaves_call_qty: Option<f64>,
    /// LEAVES_PUT_QTY 未平仓认沽合约数
    pub leaves_put_qty: Option<f64>,
    /// TRADE_DATE 交易日
    pub trade_date: String,
    pub source: &'static str,
}

/// Daily options statistics for a date from SSE `commonQuery.do`
/// (akshare `option_daily_stats_sse`).
///
/// `date` is `YYYYMMDD` (e.g. `"20240626"`).
pub async fn option_daily_stats_sse(
    client: &Client,
    date: &str,
) -> Result<Vec<SseDailyStatsRow>> {
    let params = [
        ("isPagination", "false"),
        ("sqlId", "COMMON_SSE_ZQPZ_YSP_QQ_SJTJ_MRTJ_CX"),
        ("tradeDate", date),
    ];
    let headers = [("Referer", "https://www.sse.com.cn/")];
    let text = client
        .get_text(
            SOURCE_SSE,
            "option_daily_stats_sse",
            "http://query.sse.com.cn/commonQuery.do",
            &params,
            Some(&headers),
        )
        .await?;
    let v: Value = serde_json::from_str(&text).map_err(Error::Json)?;
    parse_daily_stats_sse(&v)
}

pub(crate) fn parse_daily_stats_sse(resp: &Value) -> Result<Vec<SseDailyStatsRow>> {
    let result = resp
        .get("result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SSE,
            message: "missing result".into(),
        })?;
    let mut out = Vec::with_capacity(result.len());
    for item in result {
        out.push(SseDailyStatsRow {
            security_code: as_string(item, "SECURITY_CODE"),
            security_abbr: as_string(item, "SECURITY_ABBR"),
            contract_volume: num(item.get("CONTRACT_VOLUME")),
            total_money: num(item.get("TOTAL_MONEY")),
            total_volume: num(item.get("TOTAL_VOLUME")),
            call_volume: num(item.get("CALL_VOLUME")),
            put_volume: num(item.get("PUT_VOLUME")),
            cp_rate: num(item.get("CP_RATE")),
            leaves_qty: num(item.get("LEAVES_QTY")),
            leaves_call_qty: num(item.get("LEAVES_CALL_QTY")),
            leaves_put_qty: num(item.get("LEAVES_PUT_QTY")),
            trade_date: as_string(item, "TRADE_DATE"),
            source: SOURCE_SSE,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Sina: CFFEX index-option spot (akshare `option_cffex_*_spot_sina`)
// ---------------------------------------------------------------------------

/// A single call/put quote row from Sina's CFFEX index-option spot board.
///
/// Mirrors akshare `option_cffex_sz50_spot_sina` / `option_cffex_hs300_spot_sina`
/// / `option_cffex_zz1000_spot_sina`, which share one upstream endpoint
/// (`OptionService.getOptionData`) differing only by `product` (ho/io/mo).
/// `side` is `"call"` (看涨/up) or `"put"` (看跌/down). Exercise price is only
/// present on call rows.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CffexOptionSpotRow {
    /// "call" (看涨) or "put" (看跌)
    pub side: String,
    /// 买量
    pub bid_volume: Option<f64>,
    /// 买价
    pub bid_price: Option<f64>,
    /// 最新价
    pub last_price: Option<f64>,
    /// 卖价
    pub ask_price: Option<f64>,
    /// 卖量
    pub ask_volume: Option<f64>,
    /// 持仓量
    pub open_interest: Option<f64>,
    /// 涨跌
    pub change: Option<f64>,
    /// 行权价 (call rows only)
    pub exercise_price: Option<f64>,
    /// 合约标识
    pub symbol: Option<String>,
    pub source: &'static str,
}

/// Real-time CFFEX index-option (SSE 50 / CSI 300 / CSI 1000) spot board from
/// Sina's `OptionService.getOptionData` (akshare `option_cffex_*_spot_sina`).
///
/// `product` is `"ho"` (上证50), `"io"` (沪深300) or `"mo"` (中证1000);
/// `symbol` is the contract month code (e.g. `"ho2303"`).
pub async fn option_cffex_spot_sina(
    client: &Client,
    product: &str,
    symbol: &str,
) -> Result<Vec<CffexOptionSpotRow>> {
    let params = [
        ("type", "futures"),
        ("product", product),
        ("exchange", "cffex"),
        ("pinzhong", symbol),
    ];
    let text = client
        .get_text(
            SOURCE_SINA,
            "option_cffex_spot_sina",
            "https://stock.finance.sina.com.cn/futures/api/openapi.php/OptionService.getOptionData",
            &params,
            None,
        )
        .await?;
    let v = to_value(&text)?;
    parse_cffex_spot_sina(&v)
}

pub(crate) fn parse_cffex_spot_sina(resp: &Value) -> Result<Vec<CffexOptionSpotRow>> {
    let data = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "missing result.data".into(),
        })?;
    let mut out = Vec::new();

    if let Some(up) = data.get("up").and_then(|u| u.as_array()) {
        for row in up {
            out.push(parse_spot_row(row, "call")?);
        }
    }
    if let Some(down) = data.get("down").and_then(|d| d.as_array()) {
        for row in down {
            out.push(parse_spot_row(row, "put")?);
        }
    }
    Ok(out)
}

/// Parse one positional quote row. Call rows have 9 fields (index 7 = 行权价),
/// put rows have 8 (no separate exercise-price field).
fn parse_spot_row(row: &Value, side: &str) -> Result<CffexOptionSpotRow> {
    let arr = row
        .as_array()
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "spot row is not an array".into(),
        })?;
    let get = |i: usize| arr.get(i);
    Ok(CffexOptionSpotRow {
        side: side.to_string(),
        bid_volume: num(get(0)),
        bid_price: num(get(1)),
        last_price: num(get(2)),
        ask_price: num(get(3)),
        ask_volume: num(get(4)),
        open_interest: num(get(5)),
        change: num(get(6)),
        exercise_price: if side == "call" {
            num(get(7))
        } else {
            None
        },
        symbol: match if side == "call" { get(8) } else { get(7) } {
            Some(Value::String(s)) => Some(s.clone()),
            Some(Value::Number(n)) => Some(n.to_string()),
            _ => None,
        },
        source: SOURCE_SINA,
    })
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Parse a JSON or JSONP response body into a `Value`, stripping a
/// `callback({...})` wrapper if present.
fn to_value(text: &str) -> Result<Value> {
    let body = match (text.find('('), text.rfind(')')) {
        (Some(o), Some(c)) if c > o => &text[o + 1..c],
        _ => text,
    };
    serde_json::from_str(body).map_err(Error::Json)
}

/// Extract an `f64` from a JSON value that may be a number or a numeric string
/// (with optional thousands separators; `"-"` / empty => `None`).
fn num(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() || t == "-" {
                None
            } else {
                t.replace(',', "").parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

/// Extract a `String` from a JSON value (numbers stringified, missing => "").
fn as_string(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let txt = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&txt).unwrap()
    }

    #[test]
    fn parses_option_current_em_fixture() {
        let v = fixture("option_current_em.json");
        let rows = parse_current_em(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "10003720");
        assert_eq!(rows[0].name, "50ETF购1月3000");
        assert_eq!(rows[0].price, Some(0.1356));
        assert_eq!(rows[0].pct_change, Some(2.31));
        assert_eq!(rows[0].volume, Some(54_321.0));
        assert_eq!(rows[0].open_interest, Some(98_765.0));
        assert_eq!(rows[0].exercise_price, Some(3.0));
        assert_eq!(rows[0].remaining_days, Some(12.0));
        assert_eq!(rows[0].oi_change, Some(1200.0));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].code, "9000xxxx");
        assert_eq!(rows[1].pct_change, Some(-1.20));
    }

    #[test]
    fn parses_option_current_cffex_em_fixture() {
        let v = fixture("option_current_cffex_em.json");
        let rows = parse_current_cffex_em(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].rank, Some(1.0));
        assert_eq!(rows[0].code, "HO2303C2350");
        assert_eq!(rows[0].name, "HO2303-C-2350");
        assert_eq!(rows[0].price, Some(0.05));
        assert_eq!(rows[0].pct_change, Some(6.38));
        assert_eq!(rows[0].open_interest, Some(55_678.0));
        assert_eq!(rows[0].exercise_price, Some(2350.0));
        assert_eq!(rows[0].prev_settle, Some(0.047));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].code, "HO2303P2350");
        assert_eq!(rows[1].oi_change, Some(-300.0));
    }

    #[test]
    fn parses_option_risk_indicator_sse_fixture() {
        let v = fixture("option_risk_indicator_sse.json");
        let rows = parse_risk_indicator_sse(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].trade_date, "2024-06-26");
        assert_eq!(rows[0].security_id, "10003720");
        assert_eq!(rows[0].delta, Some(0.5123));
        assert_eq!(rows[0].theta, Some(-0.0123));
        assert_eq!(rows[0].gamma, Some(0.0456));
        assert_eq!(rows[0].vega, Some(0.0234));
        assert_eq!(rows[0].rho, Some(0.0012));
        assert_eq!(rows[0].implied_vol, Some(0.1876));
        assert_eq!(rows[1].rho, Some(-0.0011));
        assert_eq!(rows[1].source, "sse");
    }

    #[test]
    fn parses_option_current_day_sse_fixture() {
        let v = fixture("option_current_day_sse.json");
        let rows = parse_current_day_sse(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].security_id, "10003720");
        assert_eq!(rows[0].contract_id, "510050C2401M03000");
        assert_eq!(rows[0].underlying, "华夏上证50ETF(510050)");
        assert_eq!(rows[0].call_or_put, "认购");
        assert_eq!(rows[0].exercise_price, Some(3.0));
        assert_eq!(rows[0].contract_unit, Some(10_000.0));
        assert_eq!(rows[0].expire_date, "2024-01-24");
        assert_eq!(rows[1].call_or_put, "认沽");
        assert_eq!(rows[1].source, "sse");
    }

    #[test]
    fn parses_option_daily_stats_sse_fixture() {
        let v = fixture("option_daily_stats_sse.json");
        let rows = parse_daily_stats_sse(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].security_code, "510050");
        assert_eq!(rows[0].security_abbr, "华夏上证50ETF");
        assert_eq!(rows[0].contract_volume, Some(12.0));
        assert_eq!(rows[0].total_money, Some(1_234_567.89));
        assert_eq!(rows[0].total_volume, Some(56_789.0));
        assert_eq!(rows[0].call_volume, Some(32_123.0));
        assert_eq!(rows[0].put_volume, Some(24_666.0));
        assert_eq!(rows[0].cp_rate, Some(0.7689));
        assert_eq!(rows[0].leaves_qty, Some(1_234_567.0));
        assert_eq!(rows[1].leaves_put_qty, Some(800_000.0));
        assert_eq!(rows[0].trade_date, "2024-06-26");
        assert_eq!(rows[0].source, "sse");
    }

    #[test]
    fn parses_option_cffex_spot_sina_fixture() {
        let v = fixture("option_cffex_spot_sina.json");
        let rows = parse_cffex_spot_sina(&v).unwrap();
        assert_eq!(rows.len(), 4);
        // First two are calls, next two puts.
        assert_eq!(rows[0].side, "call");
        assert_eq!(rows[0].bid_volume, Some(10.0));
        assert_eq!(rows[0].last_price, Some(0.065));
        assert_eq!(rows[0].exercise_price, Some(2.35));
        assert_eq!(rows[0].symbol, Some("HO2303C2350".into()));
        assert_eq!(rows[1].side, "call");
        assert_eq!(rows[1].exercise_price, Some(2.40));
        assert_eq!(rows[2].side, "put");
        assert_eq!(rows[2].symbol, Some("HO2303P2350".into()));
        assert_eq!(rows[2].exercise_price, None);
        assert_eq!(rows[2].change, Some(-0.001));
        assert_eq!(rows[3].side, "put");
        assert_eq!(rows[0].source, "sina");
    }
}
