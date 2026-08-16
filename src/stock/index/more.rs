//! Additional index endpoints ported from `akshare` (Eastmoney / csindex / ccxe).
//!
//! This module ports *pure-HTTP* index functions only. Each `async fn` mirrors an
//! akshare `index_*` function: it builds the exact upstream URL + params, fetches
//! via [`Client`], and a `pub(crate) fn parse_*` turns the JSON into a row struct.
//! Source-specific, akshare-named surface (no normalization across sources).
//!
//! Skipped (require non-HTTP / non-JSON machinery — noted for the lead):
//! - `index_stock_info` — akshare scrapes a **Sina HTML** page (`read_html`); needs
//!   an HTML parser, which would require editing `Cargo.toml`.
//! - `stock_zh_index_value_csindex` — downloads an **`.xls`** (Excel) file via
//!   `pandas.read_excel`; needs an xlsx parser.
//! - `index_option_qvix*` — fetches **CSV** (`pandas.read_csv`, gbk-encoded) and
//!   slices by column position; needs a CSV parser + encoding handling.
//! - `index_realtime_fund_sw` — POSTs a **JSON request body**; the crate `Client`
//!   only exposes form-encoded POST / GET, so a JSON-body POST cannot be sent yet.

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

const EM_UT: &str = "bd1d9ddb04089700cf9c27f6f7426281";
const EM_SPOT_URL: &str = "https://push2.eastmoney.com/api/qt/clist/get";
const EM_KLINE_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
const EM_TREND_URL: &str = "https://push2his.eastmoney.com/api/qt/stock/trends2/get";
const CSINDEX_HIST_URL: &str = "https://www.csindex.com.cn/csindex-home/perf/index-perf";
const CCXE_PMI_URL: &str = "https://yun.ccxe.com.cn/api/index/pro/cxIndexTrendInfo";

/// (akshare name, Eastmoney market, Eastmoney code) — mirror of akshare's
/// `index_global_em_symbol_map`, used by [`index_global_hist_em`] to resolve a
/// human-readable index name to an Eastmoney `secid`.
const GLOBAL_EM_SYMBOL_MAP: &[(&str, &str, &str)] = &[
    ("波罗的海BDI指数", "100", "BDI"),
    ("葡萄牙PSI20", "100", "PSI20"),
    ("菲律宾马尼拉", "100", "PSI"),
    ("泰国SET", "100", "SET"),
    ("俄罗斯RTS", "100", "RTS"),
    ("巴基斯坦卡拉奇", "100", "KSE100"),
    ("越南胡志明", "100", "VNINDEX"),
    ("红筹指数", "124", "HSCCI"),
    ("印尼雅加达综合", "100", "JKSE"),
    ("希腊雅典ASE", "100", "ASE"),
    ("墨西哥BOLSA", "100", "MXX"),
    ("挪威OSEBX", "100", "OSEBX"),
    ("巴西BOVESPA", "100", "BVSP"),
    ("波兰WIG", "100", "WIG"),
    ("印度孟买SENSEX", "100", "SENSEX"),
    ("布拉格指数", "100", "PX"),
    ("荷兰AEX", "100", "AEX"),
    ("冰岛ICEX", "100", "ICEXI"),
    ("斯里兰卡科伦坡", "100", "CSEALL"),
    ("富时新加坡海峡时报", "100", "STI"),
    ("富时意大利MIB", "100", "MIB"),
    ("路透CRB商品指数", "100", "CRB"),
    ("比利时BFX", "100", "BFX"),
    ("富时AIM全股", "100", "AXX"),
    ("新西兰50", "100", "NZ50"),
    ("上证指数", "1", "000001"),
    ("国企指数", "100", "HSCEI"),
    ("沪深300", "1", "000300"),
    ("英国富时100", "100", "FTSE"),
    ("中小100", "0", "399005"),
    ("瑞士SMI", "100", "SSMI"),
    ("西班牙IBEX35", "100", "IBEX"),
    ("瑞典OMXSPI", "100", "OMXSPI"),
    ("爱尔兰综合", "100", "ISEQ"),
    ("韩国KOSPI", "100", "KS11"),
    ("深证成指", "0", "399001"),
    ("韩国KOSPI200", "100", "KOSPI200"),
    ("芬兰赫尔辛基", "100", "HEX"),
    ("恒生指数", "100", "HSI"),
    ("欧洲斯托克50", "100", "SX5E"),
    ("美元指数", "100", "UDI"),
    ("法国CAC40", "100", "FCHI"),
    ("台湾加权", "100", "TWII"),
    ("英国富时250", "100", "MCX"),
    ("富时马来西亚KLCI", "100", "KLSE"),
    ("OMX哥本哈根20", "100", "OMXC20"),
    ("道琼斯", "100", "DJIA"),
    ("奥地利ATX", "100", "ATX"),
    ("加拿大S&P/TSX", "100", "TSX"),
    ("德国DAX30", "100", "GDAXI"),
    ("创业板指", "0", "399006"),
    ("澳大利亚普通股", "100", "AORD"),
    ("标普500", "100", "SPX"),
    ("澳大利亚标普200", "100", "AS51"),
    ("日经225", "100", "N225"),
    ("纳斯达克", "100", "NDX"),
];

// ---------------------------------------------------------------------------
// index_global_spot_em — Eastmoney global index real-time spot (akshare name)
// ---------------------------------------------------------------------------

/// Real-time global index spot quotes (akshare `index_global_spot_em`).
///
/// Eastmoney `clist` push endpoint. Returns the curated global index list
/// (A-share benchmarks + major world indices). Pure HTTP JSON.
pub async fn index_global_spot_em(client: &Client) -> Result<Vec<GlobalSpotRow>> {
    // `fltt=1` returns raw values (price fields are *100); akshare divides by 100.
    let params = [
        ("np", "2"),
        ("fltt", "1"),
        ("invt", "2"),
        (
            "fs",
            "i:1.000001,i:0.399001,i:0.399005,i:0.399006,i:1.000300,i:100.HSI,i:100.HSCEI,i:124.HSCCI,i:100.TWII,i:100.N225,i:100.KOSPI200,i:100.KS11,i:100.STI,i:100.SENSEX,i:100.KLSE,i:100.SET,i:100.PSI,i:100.KSE100,i:100.VNINDEX,i:100.JKSE,i:100.CSEALL,i:100.SX5E,i:100.FTSE,i:100.MCX,i:100.AXX,i:100.FCHI,i:100.GDAXI,i:100.RTS,i:100.IBEX,i:100.PSI20,i:100.OMXC20,i:100.BFX,i:100.AEX,i:100.WIG,i:100.OMXSPI,i:100.SSMI,i:100.HEX,i:100.OSEBX,i:100.ATX,i:100.MIB,i:100.ASE,i:100.ICEXI,i:100.PX,i:100.ISEQ,i:100.DJIA,i:100.SPX,i:100.NDX,i:100.TSX,i:100.BVSP,i:100.MXX,i:100.AS51,i:100.AORD,i:100.NZ50,i:100.UDI,i:100.BDI,i:100.CRB",
        ),
        (
            "fields",
            "f12,f13,f14,f292,f1,f2,f4,f3,f152,f17,f18,f15,f16,f7,f124",
        ),
        ("fid", "f3"),
        ("pn", "1"),
        ("pz", "200"),
        ("po", "1"),
        ("dect", "1"),
        ("wbp2u", "|0|0|0|web"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "index_global_spot_em",
            EM_SPOT_URL,
            &params,
        )
        .await?;
    parse_global_spot(&v)
}

/// One global index spot row from [`index_global_spot_em`].
///
/// Numeric price fields are already divided by 100 (mirroring akshare), so they
/// are in human-readable units.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GlobalSpotRow {
    /// 代码 (f12) — index code, e.g. "000001" / "HSI"
    pub code: String,
    /// 名称 (f14) — index name
    pub name: String,
    /// 最新价 (f2) / 100
    pub price: Option<f64>,
    /// 涨跌额 (f4) / 100
    pub change: Option<f64>,
    /// 涨跌幅 % (f3) / 100
    pub pct_change: Option<f64>,
    /// 开盘价 (f17) / 100
    pub open: Option<f64>,
    /// 最高价 (f15) / 100
    pub high: Option<f64>,
    /// 最低价 (f16) / 100
    pub low: Option<f64>,
    /// 昨收价 (f18) / 100
    pub pre_close: Option<f64>,
    /// 振幅 % (f7) / 100
    pub amplitude: Option<f64>,
    /// 市场 (f13) — Eastmoney market id (1=SH, 0=SZ, 100=overseas, ...)
    pub market: Option<i64>,
    /// 最新行情时间 (f124) — unix epoch seconds
    pub update_time: Option<i64>,
}

/// Parse an Eastmoney `clist` global-spot response into [`GlobalSpotRow`]s.
pub(crate) fn parse_global_spot(resp: &Value) -> Result<Vec<GlobalSpotRow>> {
    let data = resp.get("data");
    let diff = match data.and_then(|d| d.get("diff")) {
        Some(Value::Array(a)) => a,
        Some(_) => {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "data.diff is not an array".into(),
            });
        }
        None => {
            if data.is_none() {
                return Ok(Vec::new());
            }
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data.diff".into(),
            });
        }
    };
    let mut out = Vec::with_capacity(diff.len());
    for item in diff {
        out.push(GlobalSpotRow {
            code: fstr(item, "f12"),
            name: fstr(item, "f14"),
            price: fnum_scale(item, "f2", 100.0),
            change: fnum_scale(item, "f4", 100.0),
            pct_change: fnum_scale(item, "f3", 100.0),
            open: fnum_scale(item, "f17", 100.0),
            high: fnum_scale(item, "f15", 100.0),
            low: fnum_scale(item, "f16", 100.0),
            pre_close: fnum_scale(item, "f18", 100.0),
            amplitude: fnum_scale(item, "f7", 100.0),
            market: item.get("f13").and_then(|v| v.as_i64()),
            update_time: item.get("f124").and_then(|v| v.as_i64()),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// index_global_hist_em — Eastmoney global index daily history (akshare name)
// ---------------------------------------------------------------------------

/// Historical daily OHLC for a global index (akshare `index_global_hist_em`).
///
/// `symbol` is a human-readable index name resolvable through akshare's global
/// symbol map (e.g. `"美元指数"`, `"恒生指数"`, `"标普500"`). It is mapped to an
/// Eastmoney `secid` and queried via the `kline` push endpoint. Pure HTTP JSON.
pub async fn index_global_hist_em(client: &Client, symbol: &str) -> Result<Vec<GlobalHistRow>> {
    let (market, code) = GLOBAL_EM_SYMBOL_MAP
        .iter()
        .find(|(name, _, _)| *name == symbol)
        .map(|(_, m, c)| (*m, *c))
        .ok_or_else(|| Error::InvalidParam(format!("unknown global index symbol: {symbol}")))?;
    let secid = format!("{market}.{code}");
    let secid_ref = secid.as_str();
    let params = [
        ("secid", secid_ref),
        ("klt", "101"),
        ("fqt", "1"),
        ("lmt", "50000"),
        ("end", "20500000"),
        ("iscca", "1"),
        ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8"),
        (
            "fields2",
            "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61,f62,f63,f64",
        ),
        ("ut", "f057cbcbce2a86e2866ab8877db1d059"),
        ("forcect", "1"),
    ];
    let v = client
        .get_json(
            SOURCE_EASTMONEY,
            "index_global_hist_em",
            EM_KLINE_URL,
            &params,
        )
        .await?;
    parse_global_hist(&v)
}

/// One global index daily bar from [`index_global_hist_em`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct GlobalHistRow {
    /// 日期 (f51) — trading date "YYYY-MM-DD"
    pub date: String,
    /// 指数代码 — Eastmoney code (from data.code)
    pub code: String,
    /// 指数名称 — from data.name
    pub name: String,
    /// 今开 (f52)
    pub open: Option<f64>,
    /// 最新价 / 收盘 (f53)
    pub close: Option<f64>,
    /// 最高 (f54)
    pub high: Option<f64>,
    /// 最低 (f55)
    pub low: Option<f64>,
    /// 振幅 % (f58)
    pub amplitude: Option<f64>,
}

/// Parse an Eastmoney `kline` global-history response into [`GlobalHistRow`]s.
pub(crate) fn parse_global_hist(resp: &Value) -> Result<Vec<GlobalHistRow>> {
    let data = resp.get("data");
    let klines = match data.and_then(|d| d.get("klines")) {
        Some(Value::Array(a)) => a,
        Some(_) => {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "data.klines is not an array".into(),
            });
        }
        None => {
            if data.is_none() {
                return Ok(Vec::new());
            }
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data.klines".into(),
            });
        }
    };
    let code = data
        .and_then(|d| d.get("code"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let name = data
        .and_then(|d| d.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let s = line.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "kline entry is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 8 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("global kline has {} fields, expected >= 8", p.len()),
            });
        }
        out.push(GlobalHistRow {
            date: p[0].to_string(),
            code: code.clone(),
            name: name.clone(),
            open: parse_f64(p[1]),
            close: parse_f64(p[2]),
            high: parse_f64(p[3]),
            low: parse_f64(p[4]),
            amplitude: parse_f64(p[7]),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// index_zh_a_hist_min_em — Eastmoney index minute bars (akshare name)
// ---------------------------------------------------------------------------

/// Intraday / minute index bars (akshare `index_zh_a_hist_min_em`).
///
/// `secid` is the Eastmoney security id, e.g. `"0.399006"` (创业板指) or
/// `"1.000300"` (沪深300). Unlike akshare (which resolves a bare numeric code via
/// an internal map), this port requires an explicit `secid` to avoid a second
/// fetch and ambiguous market inference.
///
/// `period` is `"1"` (1-minute, via the `trends2` endpoint) or one of
/// `"5"`/`"15"`/`"30"`/`"60"` (via the `kline` endpoint). `start_date` /
/// `end_date` (`"YYYYMMDD"`) are applied as `beg`/`end` for the kline branch;
/// for `period="1"` the `trends2` endpoint ignores them (returns the last 5
/// trading days) — they are intentionally unused there.
pub async fn index_zh_a_hist_min_em(
    client: &Client,
    secid: &str,
    period: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<IndexMinRow>> {
    if period == "1" {
        let params = [
            ("fields1", "f1,f2,f3,f4,f5,f6,f7,f8,f9,f10,f11,f12,f13"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58"),
            ("iscr", "0"),
            ("ndays", "5"),
            ("secid", secid),
        ];
        let v = client
            .get_json(
                SOURCE_EASTMONEY,
                "index_zh_a_hist_min_em",
                EM_TREND_URL,
                &params,
            )
            .await?;
        parse_min_trends(&v)
    } else {
        let params = [
            ("secid", secid),
            ("ut", EM_UT),
            ("fields1", "f1,f2,f3,f4,f5,f6"),
            ("fields2", "f51,f52,f53,f54,f55,f56,f57,f58,f59,f60,f61"),
            ("klt", period),
            ("fqt", "1"),
            ("beg", start_date),
            ("end", end_date),
        ];
        let v = client
            .get_json(
                SOURCE_EASTMONEY,
                "index_zh_a_hist_min_em",
                EM_KLINE_URL,
                &params,
            )
            .await?;
        parse_min_kline(&v)
    }
}

/// One minute index bar from [`index_zh_a_hist_min_em`].
///
/// Fields differ by `period`: 1-minute (`trends2`) populates `avg` (均价);
/// 5/15/30/60-minute (`kline`) populates `amplitude` / `pct_change` / `change` /
/// `turnover`. The unused fields are `None`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexMinRow {
    /// 时间 — timestamp "YYYY-MM-DD HH:MM:SS"
    pub time: String,
    /// 开盘
    pub open: Option<f64>,
    /// 收盘
    pub close: Option<f64>,
    /// 最高
    pub high: Option<f64>,
    /// 最低
    pub low: Option<f64>,
    /// 成交量
    pub volume: Option<f64>,
    /// 成交额
    pub amount: Option<f64>,
    /// 均价 (trends2 / period = "1" only)
    pub avg: Option<f64>,
    /// 振幅 % (kline / period != "1" only)
    pub amplitude: Option<f64>,
    /// 涨跌幅 % (kline only)
    pub pct_change: Option<f64>,
    /// 涨跌额 (kline only)
    pub change: Option<f64>,
    /// 换手率 % (kline only)
    pub turnover: Option<f64>,
}

/// Parse an Eastmoney `trends2` response (period = "1") into [`IndexMinRow`]s.
pub(crate) fn parse_min_trends(resp: &Value) -> Result<Vec<IndexMinRow>> {
    let data = resp.get("data");
    let trends = match data.and_then(|d| d.get("trends")) {
        Some(Value::Array(a)) => a,
        Some(_) => {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "data.trends is not an array".into(),
            });
        }
        None => {
            if data.is_none() {
                return Ok(Vec::new());
            }
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data.trends".into(),
            });
        }
    };
    let mut out = Vec::with_capacity(trends.len());
    for line in trends {
        let s = line.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "trend entry is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 8 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("trend has {} fields, expected >= 8", p.len()),
            });
        }
        out.push(IndexMinRow {
            time: p[0].to_string(),
            open: parse_f64(p[1]),
            close: parse_f64(p[2]),
            high: parse_f64(p[3]),
            low: parse_f64(p[4]),
            volume: parse_f64(p[5]),
            amount: parse_f64(p[6]),
            avg: parse_f64(p[7]),
            amplitude: None,
            pct_change: None,
            change: None,
            turnover: None,
        });
    }
    Ok(out)
}

/// Parse an Eastmoney `kline` response (period != "1") into [`IndexMinRow`]s.
pub(crate) fn parse_min_kline(resp: &Value) -> Result<Vec<IndexMinRow>> {
    let data = resp.get("data");
    let klines = match data.and_then(|d| d.get("klines")) {
        Some(Value::Array(a)) => a,
        Some(_) => {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "data.klines is not an array".into(),
            });
        }
        None => {
            if data.is_none() {
                return Ok(Vec::new());
            }
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data.klines".into(),
            });
        }
    };
    let mut out = Vec::with_capacity(klines.len());
    for line in klines {
        let s = line.as_str().ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "kline entry is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        if p.len() < 11 {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: format!("minute kline has {} fields, expected >= 11", p.len()),
            });
        }
        out.push(IndexMinRow {
            time: p[0].to_string(),
            open: parse_f64(p[1]),
            close: parse_f64(p[2]),
            high: parse_f64(p[3]),
            low: parse_f64(p[4]),
            volume: parse_f64(p[5]),
            amount: parse_f64(p[6]),
            avg: None,
            amplitude: parse_f64(p[7]),
            pct_change: parse_f64(p[8]),
            change: parse_f64(p[9]),
            turnover: parse_f64(p[10]),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// stock_zh_index_hist_csindex — csindex.com.cn index history (akshare name)
// ---------------------------------------------------------------------------

/// Historical OHLC for an index from the China Securities Index (CSI) website
/// (akshare `stock_zh_index_hist_csindex`).
///
/// `symbol` is the CSI index code, e.g. `"000928"` or `"H30374"`. `start_date` /
/// `end_date` are `"YYYYMMDD"`. Pure HTTP JSON (csindex returns the data array
/// directly under `data`).
pub async fn stock_zh_index_hist_csindex(
    client: &Client,
    symbol: &str,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<CsindexHistRow>> {
    let params = [
        ("indexCode", symbol),
        ("startDate", start_date),
        ("endDate", end_date),
    ];
    let v = client
        .get_json(
            "csindex",
            "stock_zh_index_hist_csindex",
            CSINDEX_HIST_URL,
            &params,
        )
        .await?;
    parse_csindex_hist(&v)
}

/// One CSI index daily bar from [`stock_zh_index_hist_csindex`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct CsindexHistRow {
    /// 日期 — trading date (string as returned by CSI)
    pub date: String,
    /// 指数代码 — CSI index code
    pub index_code: String,
    /// 指数中文简称 — index short Chinese name
    pub name: String,
    /// 开盘
    pub open: Option<f64>,
    /// 最高
    pub high: Option<f64>,
    /// 最低
    pub low: Option<f64>,
    /// 收盘
    pub close: Option<f64>,
    /// 涨跌
    pub change: Option<f64>,
    /// 涨跌幅 %
    pub pct_change: Option<f64>,
    /// 成交量
    pub volume: Option<f64>,
    /// 成交金额
    pub amount: Option<f64>,
    /// 样本数量 — number of constituent samples
    pub sample_count: Option<f64>,
    /// 滚动市盈率 — trailing PE
    pub pe_ttm: Option<f64>,
}

/// Parse a CSI `index-perf` response into [`CsindexHistRow`]s.
pub(crate) fn parse_csindex_hist(resp: &Value) -> Result<Vec<CsindexHistRow>> {
    let data = resp.get("data");
    let arr = match data {
        Some(Value::Array(a)) => a,
        Some(_) => {
            return Err(Error::UpstreamChanged {
                origin: "csindex",
                message: "data is not an array".into(),
            });
        }
        None => return Ok(Vec::new()),
    };
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(CsindexHistRow {
            date: fstr(item, "日期"),
            index_code: fstr(item, "指数代码"),
            name: fstr(item, "指数中文简称"),
            open: fnum(item, "开盘"),
            high: fnum(item, "最高"),
            low: fnum(item, "最低"),
            close: fnum(item, "收盘"),
            change: fnum(item, "涨跌"),
            pct_change: fnum(item, "涨跌幅"),
            volume: fnum(item, "成交量"),
            amount: fnum(item, "成交金额"),
            sample_count: fnum(item, "样本数量"),
            pe_ttm: fnum(item, "滚动市盈率"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// index_pmi_cx — Caixin (ccxe) PMI / index trend (akshare name)
// ---------------------------------------------------------------------------

/// Caixin macro index trend (akshare `index_pmi_*_cx` family).
///
/// `category` selects the series: `"com"` (综合PMI), `"man"` (制造业PMI),
/// `"ser"` (服务业PMI), `"dei"` (数字经济指数), `"ii"` (产业指数), `"si"` (溢出指数),
/// `"fi"` (融合指数). Pure HTTP JSON (ccxe `cxIndexTrendInfo`).
///
/// The `日期` field is a millisecond epoch (returned raw as `date`); callers
/// convert to a calendar date.
pub async fn index_pmi_cx(client: &Client, category: &str) -> Result<Vec<PmiCxRow>> {
    let (type_param, value_key) = pmi_cat(category)
        .ok_or_else(|| Error::InvalidParam(format!("unknown pmi category: {category}")))?;
    let params = [("type", type_param)];
    let v = client
        .get_json("ccxe", "index_pmi_cx", CCXE_PMI_URL, &params)
        .await?;
    parse_pmi_cx(&v, value_key)
}

/// One Caixin PMI / index point from [`index_pmi_cx`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct PmiCxRow {
    /// 日期 — millisecond epoch (raw from upstream)
    pub date: Option<i64>,
    /// the series value (e.g. 综合PMI) — key depends on `category`
    pub value: Option<f64>,
    /// 变化值 — change vs previous period
    pub change: Option<f64>,
}

/// (category, ccxe `type` param, JSON value key).
const PMI_CX_CATS: &[(&str, &str, &str)] = &[
    ("com", "com", "综合PMI"),
    ("man", "man", "制造业PMI"),
    ("ser", "ser", "服务业PMI"),
    ("dei", "dei", "数字经济指数"),
    ("ii", "ii", "产业指数"),
    ("si", "si", "溢出指数"),
    ("fi", "fi", "融合指数"),
];

fn pmi_cat(category: &str) -> Option<(&'static str, &'static str)> {
    PMI_CX_CATS
        .iter()
        .find(|(c, _, _)| *c == category)
        .map(|(_, t, k)| (*t, *k))
}

/// Parse a ccxe `cxIndexTrendInfo` response into [`PmiCxRow`]s.
pub(crate) fn parse_pmi_cx(resp: &Value, value_key: &str) -> Result<Vec<PmiCxRow>> {
    let data = resp.get("data");
    let arr = match data {
        Some(Value::Array(a)) => a,
        Some(_) => {
            return Err(Error::UpstreamChanged {
                origin: "ccxe",
                message: "data is not an array".into(),
            });
        }
        None => return Ok(Vec::new()),
    };
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(PmiCxRow {
            date: item.get("日期").and_then(|v| v.as_i64()),
            value: item.get(value_key).and_then(num),
            change: item.get("变化值").and_then(num),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

fn fstr(item: &Value, k: &str) -> String {
    item.get(k)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn fnum(item: &Value, k: &str) -> Option<f64> {
    item.get(k).and_then(num)
}

/// Parse an Eastmoney field that may be a number or a numeric string (`"-"` / empty => None),
/// then divide by `scale` (used because `fltt=1` returns raw *100 values).
fn fnum_scale(item: &Value, k: &str, scale: f64) -> Option<f64> {
    item.get(k).and_then(num).map(|x| x / scale)
}

/// Parse a number-or-numeric-string leaf (`"-"` / empty => None).
fn num(v: &Value) -> Option<f64> {
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

fn parse_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        t.parse::<f64>().ok()
    }
}

// ---------------------------------------------------------------------------
// offline parse tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let text = std::fs::read_to_string(path).expect("fixture missing");
        serde_json::from_str(&text).expect("fixture is not valid JSON")
    }

    #[test]
    fn test_parse_index_global_spot_em() {
        let v = fixture("index_global_spot_em.json");
        let rows = parse_global_spot(&v).expect("parse index_global_spot_em");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "000001");
        assert_eq!(rows[0].name, "上证指数");
        assert_eq!(rows[0].price, Some(3200.50));
        assert_eq!(rows[0].change, Some(38.00));
        assert_eq!(rows[0].pct_change, Some(1.20));
        assert_eq!(rows[0].open, Some(3195.00));
        assert_eq!(rows[0].high, Some(3210.00));
        assert_eq!(rows[0].low, Some(3180.00));
        assert_eq!(rows[0].pre_close, Some(3162.50));
        assert_eq!(rows[0].amplitude, Some(9.50));
        assert_eq!(rows[0].market, Some(1));
        assert_eq!(rows[0].update_time, Some(1709289600));
        assert_eq!(rows[1].code, "HSI");
        assert_eq!(rows[1].name, "恒生指数");
        assert_eq!(rows[1].price, Some(16500.00));
        assert_eq!(rows[1].pct_change, Some(-0.30));
        assert_eq!(rows[1].market, Some(100));
    }

    #[test]
    fn test_parse_index_global_hist_em() {
        let v = fixture("index_global_hist_em.json");
        let rows = parse_global_hist(&v).expect("parse index_global_hist_em");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2025-01-02");
        assert_eq!(rows[0].code, "UDI");
        assert_eq!(rows[0].name, "美元指数");
        assert_eq!(rows[0].open, Some(102.5));
        assert_eq!(rows[0].close, Some(103.1));
        assert_eq!(rows[0].high, Some(103.4));
        assert_eq!(rows[0].low, Some(102.3));
        assert_eq!(rows[0].amplitude, Some(0.30));
        assert_eq!(rows[1].date, "2025-01-03");
        assert_eq!(rows[1].close, Some(102.8));
    }

    #[test]
    fn test_parse_index_zh_a_hist_min_em_trends() {
        let v = fixture("index_zh_a_hist_min_em_trends.json");
        let rows = parse_min_trends(&v).expect("parse index_zh_a_hist_min_em trends");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].time, "2025-03-17 09:31:00");
        assert_eq!(rows[0].open, Some(2000.0));
        assert_eq!(rows[0].close, Some(2010.0));
        assert_eq!(rows[0].high, Some(2015.0));
        assert_eq!(rows[0].low, Some(1995.0));
        assert_eq!(rows[0].volume, Some(10000.0));
        assert_eq!(rows[0].amount, Some(500000.0));
        assert_eq!(rows[0].avg, Some(2008.0));
        assert_eq!(rows[0].amplitude, None);
        assert_eq!(rows[1].close, Some(2012.0));
    }

    #[test]
    fn test_parse_index_zh_a_hist_min_em_kline() {
        let v = fixture("index_zh_a_hist_min_em_kline.json");
        let rows = parse_min_kline(&v).expect("parse index_zh_a_hist_min_em kline");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].time, "2025-03-17 09:35:00");
        assert_eq!(rows[0].open, Some(3500.0));
        assert_eq!(rows[0].close, Some(3510.0));
        assert_eq!(rows[0].high, Some(3520.0));
        assert_eq!(rows[0].low, Some(3490.0));
        assert_eq!(rows[0].volume, Some(100000.0));
        assert_eq!(rows[0].amount, Some(5000000.0));
        assert_eq!(rows[0].avg, None);
        assert_eq!(rows[0].amplitude, Some(0.90));
        assert_eq!(rows[0].pct_change, Some(0.30));
        assert_eq!(rows[0].change, Some(10.0));
        assert_eq!(rows[0].turnover, Some(0.05));
        assert_eq!(rows[1].close, Some(3505.0));
    }

    #[test]
    fn test_parse_stock_zh_index_hist_csindex() {
        let v = fixture("index_csindex_hist.json");
        let rows = parse_csindex_hist(&v).expect("parse stock_zh_index_hist_csindex");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].index_code, "000928");
        assert_eq!(rows[0].name, "中证500");
        assert_eq!(rows[0].open, Some(1000.0));
        assert_eq!(rows[0].high, Some(1010.0));
        assert_eq!(rows[0].low, Some(990.0));
        assert_eq!(rows[0].close, Some(1005.0));
        assert_eq!(rows[0].change, Some(5.0));
        assert_eq!(rows[0].pct_change, Some(0.5));
        assert_eq!(rows[0].volume, Some(12345.0));
        assert_eq!(rows[0].amount, Some(67890.0));
        assert_eq!(rows[0].sample_count, Some(500.0));
        assert_eq!(rows[0].pe_ttm, Some(12.3));
        assert_eq!(rows[1].index_code, "000928");
        assert_eq!(rows[1].close, Some(1010.0));
    }

    #[test]
    fn test_parse_index_pmi_cx() {
        let v = fixture("index_pmi_cx.json");
        let rows = parse_pmi_cx(&v, "综合PMI").expect("parse index_pmi_cx");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, Some(1704153600000));
        assert_eq!(rows[0].value, Some(50.8));
        assert_eq!(rows[0].change, Some(0.3));
        assert_eq!(rows[1].value, Some(51.2));
        assert_eq!(rows[1].change, Some(-0.4));
    }

    #[test]
    fn test_pmi_cat_unknown() {
        assert!(pmi_cat("nope").is_none());
        assert_eq!(pmi_cat("com"), Some(("com", "综合PMI")));
        assert_eq!(pmi_cat("ser"), Some(("ser", "服务业PMI")));
    }
}
