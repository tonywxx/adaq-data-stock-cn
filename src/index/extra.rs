//! Misc. Chinese index endpoints (akshare `index` package, misc. sources).
//!
//! Pure-JSON / JSONP ports from several upstreams:
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `index_sugar_msweet` | `index/index_sugar.py:13` | 沐甜科技 中国食糖指数 (`msweet.com.cn/eportal/ui`) |
//! | `index_inner_quote_sugar_msweet` | `index/index_sugar.py:39` | 配额内进口糖估算 (`JinKongTang.json`) |
//! | `index_outer_quote_sugar_msweet` | `index/index_sugar.py:84` | 配额外进口糖估算 (`Jkpewlr.json`) |
//! | `index_kq_fz` | `index/index_kq_fz.py:14` | 柯桥纺织指数 (`kqindex.cn/flzs/table_data`, paged JSON) |
//! | `index_kq_fashion` | `index/index_kq_ss.py:13` | 柯桥时尚指数 (`api.idx365.com`) |
//! | `index_eri` | `index/index_eri.py:13` | 浙江排污权交易指数 (`zs.zjpwq.net`) |
//! | `index_yw` | `index/index_yw.py:18` | 义乌小商品指数 (`chinagoods.com`) |
//! | `index_price_cflp` | `index/index_cflp.py:13` | 公路物流运价指数 (`index.0256.cn`, POST JSON) |
//! | `index_volume_cflp` | `index/index_cflp.py:63` | 公路物流运量指数 (`index.0256.cn`, POST JSON) |
//! | `index_global_hist_sina` | `index/index_global_sina.py:30` | 新浪环球市场历史 (`gi.finance.sina.com.cn`) |
//! | `index_global_name_table` | `index/index_global_sina.py:15` | 新浪环球市场名称/代码表 (static) |
//!
//! ## DEFERRED
//!
//! * **`sw_index_first_info`** (`index/index_sw.py:38`) — HTML scrape via
//!   `BeautifulSoup` (`legulegu.com/stockdata/sw-industry-overview`); DOM parsing
//!   not available in this crate, and no JSON API is used by the source.
//! * **`sw_index_second_info`** (`index/index_sw.py:96`) — same HTML scrape.
//! * **`sw_index_third_info`** (`index/index_sw.py:158`) — same HTML scrape.
//! * **`sw_index_third_cons`** (`index/index_sw.py:220`) — HTML scrape via
//!   `pd.read_html` (table parse); no pure-JSON source.
//! * **`drewry_wci_index`** (`index/index_drewry.py:17`) — HTML scrape: a
//!   `window.infographicData=...` blob is sliced out of a `<script>` tag and
//!   decoded with `demjson`; requires HTML parsing + JS-object decoding.

use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};
use crate::core::json::*;

const SOURCE_MSWEET: &str = "msweet";
const SOURCE_KQ: &str = "kqindex";
const SOURCE_ERI: &str = "zjpwq";
const SOURCE_YW: &str = "chinagoods";
const SOURCE_CFLP: &str = "cflp";
const SOURCE_SINA: &str = "sina";

// ---------------------------------------------------------------------------
// Shared field helpers
// ---------------------------------------------------------------------------

/// `i`-th element of an array-valued field, parsed to `f64`.
fn arr_num(item: &Value, _key: &str, idx: usize) -> Option<f64> {
    item.as_array()
        .and_then(|a| a.get(idx))
        .and_then(|v| match v {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        })
}

// ===========================================================================
// index_sugar_msweet / inner / outer  (akshare index_sugar.py)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct SugarMsweetRow {
    pub date: String,
    pub composite_price: Option<f64>,
    pub raw_sugar_price: Option<f64>,
    pub spot_price: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SugarInnerQuoteRow {
    pub date: String,
    pub profit_space: Option<f64>,
    pub thai_sugar: Option<f64>,
    pub thai_ma5: Option<f64>,
    pub brazil_ma5: Option<f64>,
    pub profit_ma5: Option<f64>,
    pub brazil_ma10: Option<f64>,
    pub brazil_sugar: Option<f64>,
    pub liuzhou_spot: Option<f64>,
    pub guangzhou_spot: Option<f64>,
    pub thai_ma10: Option<f64>,
    pub profit_ma30: Option<f64>,
    pub profit_ma10: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SugarOuterQuoteRow {
    pub date: String,
    pub brazil_import_cost: Option<f64>,
    pub thai_profit_space: Option<f64>,
    pub brazil_profit_space: Option<f64>,
    pub thai_import_cost: Option<f64>,
    pub rizhao_spot: Option<f64>,
}

/// Parsed `msweet` table: category labels paired with a row-major matrix of
/// optional numeric cells (`data[i]` aligned to `category[i]`).
type MsweetTable = (Vec<String>, Vec<Vec<Option<f64>>>);

/// Parse the `category`+`data` shape shared by all three `msweet` endpoints.
/// `data[i]` holds `ncols` numeric values aligned to `category[i]`.
fn parse_msweet_rows(resp: &Value, ncols: usize) -> Result<MsweetTable> {
    let category = resp
        .get("category")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_MSWEET,
            message: "missing category array".into(),
        })?;
    let data =
        resp.get("data")
            .and_then(|v| v.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_MSWEET,
                message: "missing data array".into(),
            })?;
    let mut dates = Vec::with_capacity(category.len());
    let mut rows = Vec::with_capacity(data.len());
    for (i, c) in category.iter().enumerate() {
        dates.push(c.as_str().unwrap_or_default().to_string());
        let mut vals = Vec::with_capacity(ncols);
        let row = data.get(i);
        for j in 0..ncols {
            vals.push(row.and_then(|r| arr_num(r, "", j)));
        }
        rows.push(vals);
    }
    Ok((dates, rows))
}

pub(crate) fn parse_sugar_msweet(resp: &Value) -> Result<Vec<SugarMsweetRow>> {
    let (dates, rows) = parse_msweet_rows(resp, 3)?;
    Ok(dates
        .into_iter()
        .zip(rows)
        .map(|(date, v)| SugarMsweetRow {
            date,
            composite_price: v[0],
            raw_sugar_price: v[1],
            spot_price: v[2],
        })
        .collect())
}

pub(crate) fn parse_inner_quote_sugar_msweet(resp: &Value) -> Result<Vec<SugarInnerQuoteRow>> {
    let (dates, rows) = parse_msweet_rows(resp, 12)?;
    Ok(dates
        .into_iter()
        .zip(rows)
        .map(|(date, v)| SugarInnerQuoteRow {
            date,
            profit_space: v[0],
            thai_sugar: v[1],
            thai_ma5: v[2],
            brazil_ma5: v[3],
            profit_ma5: v[4],
            brazil_ma10: v[5],
            brazil_sugar: v[6],
            liuzhou_spot: v[7],
            guangzhou_spot: v[8],
            thai_ma10: v[9],
            profit_ma30: v[10],
            profit_ma10: v[11],
        })
        .collect())
}

pub(crate) fn parse_outer_quote_sugar_msweet(resp: &Value) -> Result<Vec<SugarOuterQuoteRow>> {
    let (dates, rows) = parse_msweet_rows(resp, 5)?;
    Ok(dates
        .into_iter()
        .zip(rows)
        .map(|(date, v)| SugarOuterQuoteRow {
            date,
            brazil_import_cost: v[0],
            thai_profit_space: v[1],
            brazil_profit_space: v[2],
            thai_import_cost: v[3],
            rizhao_spot: v[4],
        })
        .collect())
}

/// 沐甜科技数据中心-中国食糖指数 (akshare `index_sugar_msweet`).
pub async fn index_sugar_msweet(client: &Client) -> Result<Vec<SugarMsweetRow>> {
    let v = client
        .get_json(
            SOURCE_MSWEET,
            "index_sugar_msweet",
            "https://www.msweet.com.cn/eportal/ui",
            &[
                ("struts.portlet.action", "/portlet/price!getSTZSJson.action"),
                ("moduleId", "cb752447cfe24b44b18c7a7e9abab048"),
            ],
        )
        .await?;
    parse_sugar_msweet(&v)
}

/// 沐甜科技数据中心-配额内进口糖估算指数 (akshare `index_inner_quote_sugar_msweet`).
pub async fn index_inner_quote_sugar_msweet(client: &Client) -> Result<Vec<SugarInnerQuoteRow>> {
    let v = client
        .get_json(
            SOURCE_MSWEET,
            "index_inner_quote_sugar_msweet",
            "https://www.msweet.com.cn/datacenterapply/datacenter/json/JinKongTang.json",
            &[],
        )
        .await?;
    parse_inner_quote_sugar_msweet(&v)
}

/// 沐甜科技数据中心-配额外进口糖估算指数 (akshare `index_outer_quote_sugar_msweet`).
pub async fn index_outer_quote_sugar_msweet(client: &Client) -> Result<Vec<SugarOuterQuoteRow>> {
    let v = client
        .get_json(
            SOURCE_MSWEET,
            "index_outer_quote_sugar_msweet",
            "https://www.msweet.com.cn/datacenterapply/datacenter/json/Jkpewlr.json",
            &[],
        )
        .await?;
    parse_outer_quote_sugar_msweet(&v)
}

// ===========================================================================
// index_kq_fz  (akshare index_kq_fz.py)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct KqFzRow {
    /// 期次
    pub period: String,
    /// 指数 (价格指数 symbol) / 总景气指数 (景气指数 symbol)
    pub index_value: Option<f64>,
    /// 涨跌幅 (价格指数 / 景气指数 symbol)
    pub change: Option<f64>,
    /// 总景气指数 (景气指数 symbol)
    pub total_prosperity: Option<f64>,
    /// 流通景气指数 (景气指数 symbol)
    pub circulation_prosperity: Option<f64>,
    /// 生产景气指数 (景气指数 symbol)
    pub production_prosperity: Option<f64>,
    /// 价格指数 (外贸指数 symbol)
    pub price_index: Option<f64>,
    /// 价格指数-涨跌幅 (外贸指数 symbol)
    pub price_index_change: Option<f64>,
    /// 景气指数 (外贸指数 symbol)
    pub prosperity_index: Option<f64>,
    /// 景气指数-涨跌幅 (外贸指数 symbol)
    pub prosperity_index_change: Option<f64>,
}

const KQ_FZ_SYMBOLS: &[(&str, &str)] =
    &[("价格指数", "1_1"), ("景气指数", "1_2"), ("外贸指数", "2")];

/// Parse a `kqindex.cn` `result` array (keys vary by `symbol`).
pub(crate) fn parse_kq_fz(items: &[Value]) -> Vec<KqFzRow> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let period = opt_str(item, "期次").unwrap_or_default();
        out.push(KqFzRow {
            period,
            index_value: opt_f64(item, "指数"),
            change: opt_f64(item, "涨跌幅"),
            total_prosperity: opt_f64(item, "总景气指数"),
            circulation_prosperity: opt_f64(item, "流通景气指数"),
            production_prosperity: opt_f64(item, "生产景气指数"),
            price_index: opt_f64(item, "价格指数"),
            price_index_change: opt_f64(item, "价格指数-涨跌幅"),
            prosperity_index: opt_f64(item, "景气指数"),
            prosperity_index_change: opt_f64(item, "景气指数-涨跌幅"),
        });
    }
    out
}

/// 中国柯桥纺织指数 (akshare `index_kq_fz`). Pure JSON, paginated
/// (`page`+`result`); fetches every page and merges, mirroring akshare's loop.
pub async fn index_kq_fz(client: &Client, symbol: &str) -> Result<Vec<KqFzRow>> {
    let index_type = KQ_FZ_SYMBOLS
        .iter()
        .find(|(name, _)| *name == symbol)
        .map(|(_, code)| *code)
        .ok_or_else(|| Error::InvalidParam(format!("unknown kq_fz symbol: {symbol}")))?;

    let first = client
        .get_json(
            SOURCE_KQ,
            "index_kq_fz",
            "http://www.kqindex.cn/flzs/table_data",
            &[
                ("category", "0"),
                ("start", ""),
                ("end", ""),
                ("indexType", index_type),
                ("pageindex", "1"),
            ],
        )
        .await?;
    let total = first
        .get("page")
        .and_then(|v| v.as_i64())
        .unwrap_or(1)
        .max(1);
    let mut data: Vec<Value> = first
        .get("result")
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();
    for p in 2..=total {
        let pi = p.to_string();
        let v = client
            .get_json(
                SOURCE_KQ,
                "index_kq_fz",
                "http://www.kqindex.cn/flzs/table_data",
                &[
                    ("category", "0"),
                    ("start", ""),
                    ("end", ""),
                    ("indexType", index_type),
                    ("pageindex", pi.as_str()),
                ],
            )
            .await?;
        if let Some(arr) = v.get("result").and_then(|r| r.as_array()) {
            data.extend(arr.iter().cloned());
        }
    }
    Ok(parse_kq_fz(&data))
}

// ===========================================================================
// index_kq_fashion  (akshare index_kq_ss.py)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct KqFashionRow {
    /// 日期 (publishTime)
    pub date: String,
    /// 指数 (indexValue)
    pub index_value: Option<f64>,
    /// 涨跌值 (diff of 指数)
    pub change_value: Option<f64>,
    /// 涨跌幅 (pct_change of 指数)
    pub change_pct: Option<f64>,
}

const KQ_FASHION_SYMBOLS: &[(&str, &str)] = &[
    ("柯桥时尚指数", "root"),
    ("时尚创意指数", "01"),
    ("时尚设计人才数", "0101"),
    ("新花型推出数", "0102"),
    ("创意产品成交数", "0103"),
    ("创意企业数量", "0104"),
    ("时尚活跃度指数", "02"),
    ("电商运行数", "0201"),
    ("时尚平台拓展数", "0201"),
    ("新产品销售额占比", "0201"),
    ("企业合作占比", "0201"),
    ("品牌传播费用", "0201"),
    ("时尚推广度指数", "03"),
    ("国际交流合作次数", "0301"),
    ("企业参展次数", "0302"),
    ("外商驻点数量变化", "0302"),
    ("时尚评价指数", "04"),
];

pub(crate) fn parse_kq_fashion(resp: &Value) -> Vec<KqFashionRow> {
    let arr = resp
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::with_capacity(arr.len());
    for item in &arr {
        out.push(KqFashionRow {
            date: opt_str(item, "publishTime").unwrap_or_default(),
            index_value: opt_f64(item, "indexValue"),
            change_value: None,
            change_pct: None,
        });
    }
    out
}

/// Fill `change_value` / `change_pct` after sorting by date (akshare semantics).
fn kq_fashion_with_changes(mut rows: Vec<KqFashionRow>) -> Vec<KqFashionRow> {
    rows.sort_by(|a, b| a.date.cmp(&b.date));
    for i in 1..rows.len() {
        if let (Some(prev), Some(cur)) = (rows[i - 1].index_value, rows[i].index_value) {
            rows[i].change_value = Some(cur - prev);
            if prev != 0.0 {
                rows[i].change_pct = Some((cur - prev) / prev);
            }
        }
    }
    rows
}

/// 柯桥时尚指数 (akshare `index_kq_fashion`). Pure JSON (`api.idx365.com`).
pub async fn index_kq_fashion(client: &Client, symbol: &str) -> Result<Vec<KqFashionRow>> {
    let code = KQ_FASHION_SYMBOLS
        .iter()
        .find(|(name, _)| *name == symbol)
        .map(|(_, c)| *c)
        .ok_or_else(|| Error::InvalidParam(format!("unknown kq_fashion symbol: {symbol}")))?;
    let v = client
        .get_json(
            SOURCE_KQ,
            "index_kq_fashion",
            "http://api.idx365.com/index/project/34/data",
            &[("structCode", code)],
        )
        .await?;
    Ok(kq_fashion_with_changes(parse_kq_fashion(&v)))
}

// ===========================================================================
// index_eri  (akshare index_eri.py)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct EriRow {
    /// 日期
    pub date: String,
    /// 交易指数
    pub trade_index: Option<f64>,
    /// 成交量
    pub volume: Option<f64>,
    /// 成交额
    pub amount: Option<f64>,
}

/// Parse the `indexData` response: `(date, indexValue)` pairs.
fn parse_eri_index(items: &[Value]) -> Vec<(String, Option<f64>)> {
    items
        .iter()
        .map(|item| {
            let date = item
                .get("stage")
                .and_then(|s| s.get("publishTime"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            (date, opt_f64(item, "indexValue"))
        })
        .collect()
}

/// Parse the `dataStatistics` response: `(totalQuantity, totalCost)` pairs.
fn parse_eri_stat(items: &[Value]) -> Vec<(Option<f64>, Option<f64>)> {
    items
        .iter()
        .map(|item| (opt_f64(item, "totalQuantity"), opt_f64(item, "totalCost")))
        .collect()
}

/// 浙江省排污权交易指数 (akshare `index_eri`). Two pure-JSON GETs, merged by row.
pub async fn index_eri(client: &Client, symbol: &str) -> Result<Vec<EriRow>> {
    let cycle = match symbol {
        "季度" => "QUARTER",
        "月度" => "MONTH",
        _ => return Err(Error::InvalidParam(format!("unknown eri symbol: {symbol}"))),
    };
    let params: &[(&str, &str)] = &[
        ("cycle", cycle),
        ("regionId", "1"),
        ("structId", "1"),
        ("pageSize", "5000"),
        ("indexId", "1"),
        ("orderBy", "stage.publishTime"),
    ];
    let idx_val = client
        .get_json(
            SOURCE_ERI,
            "index_eri",
            "https://zs.zjpwq.net/pwq-index-webapi/indexData",
            params,
        )
        .await?;
    let stat_val = client
        .get_json(
            SOURCE_ERI,
            "index_eri",
            "https://zs.zjpwq.net/pwq-index-webapi/dataStatistics",
            params,
        )
        .await?;
    let idx_arr = idx_val
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    let stat_arr = stat_val
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    let idx = parse_eri_index(&idx_arr);
    let stat = parse_eri_stat(&stat_arr);
    Ok(idx
        .into_iter()
        .zip(stat)
        .map(|((date, trade_index), (volume, amount))| EriRow {
            date,
            trade_index,
            volume,
            amount,
        })
        .collect())
}

// ===========================================================================
// index_yw  (akshare index_yw.py)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct YwRow {
    /// 期数 (indextimeno)
    pub period: String,
    /// 景气指数 (月景气指数 symbol)
    pub prosperity_index: Option<f64>,
    /// 规模指数
    pub scale_index: Option<f64>,
    /// 效益指数
    pub benefit_index: Option<f64>,
    /// 市场信心指数
    pub confidence_index: Option<f64>,
    /// 价格指数 (周/月价格指数 symbol)
    pub price_index: Option<f64>,
    /// 场内价格指数
    pub in_park_price_index: Option<f64>,
    /// 网上价格指数
    pub online_price_index: Option<f64>,
    /// 订单价格指数
    pub order_price_index: Option<f64>,
    /// 出口价格指数
    pub export_price_index: Option<f64>,
}

const YW_SYMBOLS: &[(&str, &str)] = &[
    ("月景气指数", "bi"),
    ("周价格指数", "piweek"),
    ("月价格指数", "month"),
];

pub(crate) fn parse_yw(resp: &Value) -> Vec<YwRow> {
    let arr = resp
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    arr.iter()
        .map(|item| YwRow {
            period: opt_str(item, "indextimeno").unwrap_or_default(),
            prosperity_index: opt_f64(item, "totalindex"),
            scale_index: opt_f64(item, "scopeindex"),
            benefit_index: opt_f64(item, "benifitindex"),
            confidence_index: opt_f64(item, "confidentindex"),
            price_index: opt_f64(item, "totalpriceindex"),
            in_park_price_index: opt_f64(item, "stockdealpriceindex"),
            online_price_index: opt_f64(item, "netdealpriceindex"),
            order_price_index: opt_f64(item, "orderdealpriceindex"),
            export_price_index: opt_f64(item, "outdealpriceindex"),
        })
        .collect()
}

/// 义乌小商品指数 (akshare `index_yw`). Pure JSON (`apiserver.chinagoods.com`).
pub async fn index_yw(client: &Client, symbol: &str) -> Result<Vec<YwRow>> {
    let path = YW_SYMBOLS
        .iter()
        .find(|(name, _)| *name == symbol)
        .map(|(_, p)| *p)
        .ok_or_else(|| Error::InvalidParam(format!("unknown yw symbol: {symbol}")))?;
    let url = if path == "bi" {
        "https://apiserver.chinagoods.com/yiwuindex/v1/active/industry/class/history/bi?gcCode="
            .to_string()
    } else {
        format!(
            "https://apiserver.chinagoods.com/yiwuindex/v1/active/industry/class/history/{path}?gcCode="
        )
    };
    let v = client.get_json(SOURCE_YW, "index_yw", &url, &[]).await?;
    Ok(parse_yw(&v))
}

// ===========================================================================
// index_price_cflp / index_volume_cflp  (akshare index_cflp.py)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct CflpRow {
    /// 日期
    pub date: String,
    /// 定基指数
    pub fixed_base_index: Option<f64>,
    /// 环比指数
    pub mom_index: Option<f64>,
    /// 同比指数
    pub yoy_index: Option<f64>,
}

/// Parse a `cflp` POST response: `chart1.xLebal` (dates) + three `yLebal` series.
pub(crate) fn parse_cflp(resp: &Value) -> Result<Vec<CflpRow>> {
    let c1 = resp.get("chart1").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_CFLP,
        message: "missing chart1".into(),
    })?;
    let dates =
        c1.get("xLebal")
            .and_then(|v| v.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_CFLP,
                message: "missing chart1.xLebal".into(),
            })?;
    let y1 = c1.get("yLebal").and_then(|v| v.as_array());
    let y2 = resp
        .get("chart2")
        .and_then(|c| c.get("yLebal"))
        .and_then(|v| v.as_array());
    let y3 = resp
        .get("chart3")
        .and_then(|c| c.get("yLebal"))
        .and_then(|v| v.as_array());
    let mut out = Vec::with_capacity(dates.len());
    for (i, d) in dates.iter().enumerate() {
        let date = d.as_str().unwrap_or_default().to_string();
        let at = |arr: Option<&Vec<Value>>| -> Option<f64> {
            arr.and_then(|a| a.get(i)).and_then(|v| match v {
                Value::Number(n) => n.as_f64(),
                Value::String(s) => s.trim().parse::<f64>().ok(),
                _ => None,
            })
        };
        out.push(CflpRow {
            date,
            fixed_base_index: at(y1),
            mom_index: at(y2),
            yoy_index: at(y3),
        });
    }
    Ok(out)
}

const CFLP_PRICE_SYMBOLS: &[(&str, &str)] = &[
    ("周指数", "2"),
    ("月指数", "3"),
    ("季度指数", "4"),
    ("年度指数", "5"),
];

const CFLP_VOLUME_SYMBOLS: &[(&str, &str)] =
    &[("月指数", "3"), ("季度指数", "4"), ("年度指数", "5")];

fn cflp_headers() -> Option<&'static [(&'static str, &'static str)]> {
    Some(&[
        ("Origin", "http://index.0256.cn"),
        ("Referer", "http://index.0256.cn/expx.htm"),
    ])
}

/// 中国公路物流运价指数 (akshare `index_price_cflp`). POST form -> JSON.
pub async fn index_price_cflp(client: &Client, symbol: &str) -> Result<Vec<CflpRow>> {
    let exp_type = CFLP_PRICE_SYMBOLS
        .iter()
        .find(|(name, _)| *name == symbol)
        .map(|(_, code)| *code)
        .ok_or_else(|| Error::InvalidParam(format!("unknown cflp price symbol: {symbol}")))?;
    let v = client
        .post_form_json(
            SOURCE_CFLP,
            "index_price_cflp",
            "http://index.0256.cn/expcenter_trend.action",
            &[
                ("marketId", "1"),
                ("attribute1", "5"),
                ("exponentTypeId", exp_type),
                ("cateId", "2"),
                ("attribute2", "华北"),
                ("city", ""),
                ("startLine", ""),
                ("endLine", ""),
            ],
            cflp_headers(),
        )
        .await?;
    parse_cflp(&v)
}

/// 中国公路物流运量指数 (akshare `index_volume_cflp`). POST form -> JSON.
pub async fn index_volume_cflp(client: &Client, symbol: &str) -> Result<Vec<CflpRow>> {
    let exp_type = CFLP_VOLUME_SYMBOLS
        .iter()
        .find(|(name, _)| *name == symbol)
        .map(|(_, code)| *code)
        .ok_or_else(|| Error::InvalidParam(format!("unknown cflp volume symbol: {symbol}")))?;
    let v = client
        .post_form_json(
            SOURCE_CFLP,
            "index_volume_cflp",
            "http://index.0256.cn/volume_query.action",
            &[
                ("type", "1"),
                ("marketId", "1"),
                ("expTypeId", exp_type),
                ("startDate1", ""),
                ("endDate1", ""),
                ("city", ""),
                ("startDate3", ""),
                ("endDate3", ""),
            ],
            cflp_headers(),
        )
        .await?;
    parse_cflp(&v)
}

// ===========================================================================
// index_global_hist_sina / index_global_name_table  (akshare index_global_sina.py)
// ===========================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub struct GlobalHistSinaRow {
    pub date: String,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
    pub volume: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GlobalNameRow {
    /// 指数名称
    pub name: String,
    /// 代码
    pub code: String,
}

/// Static mirror of `akshare.index.cons.index_global_sina_symbol_map`.
const GLOBAL_SINA_MAP: &[(&str, &str)] = &[
    ("英国富时100指数", "UKX"),
    ("德国DAX 30种股价指数", "DAX"),
    ("俄罗斯MICEX指数", "INDEXCF"),
    ("法CAC40指数", "CAC"),
    ("瑞士股票指数", "SWI20"),
    ("富时意大利MIB指数", "FTSEMIB"),
    ("荷兰AEX综合指数", "AEX"),
    ("西班牙IBEX指数", "IBEX"),
    ("欧洲Stoxx50指数", "SX5E"),
    ("加拿大S&P/TSX综合指数", "GSPTSE"),
    ("墨西哥BOLSA指数", "MXX"),
    ("巴西BOVESPA股票指数", "IBOV"),
    ("中国台湾加权指数", "TWJQ"),
    ("日经225指数", "NKY"),
    ("首尔综合指数", "KOSPI"),
    ("印度尼西亚雅加达综合指数", "JCI"),
    ("印度孟买SENSEX指数", "SENSEX"),
    ("澳大利亚标准普尔200指数", "AS51"),
    ("新西兰NZSE 50指数", "NZ250"),
    ("埃及CASE 30指数", "CASE"),
];

/// Parse a Sina response that is either pure JSON or JSONP-wrapped
/// (`var x = {...};`). Mirrors akshare's `r.json()` while tolerating the JSONP
/// envelope some Sina endpoints emit.
fn sina_value_from_text(text: &str) -> Result<Value> {
    let t = text.trim();
    if let Ok(v) = serde_json::from_str::<Value>(t) {
        return Ok(v);
    }
    if let Some(eq) = t.find('=') {
        let mut body = t[eq + 1..].trim();
        if let Some(b) = body.strip_prefix('(') {
            body = b;
        }
        if let Some(b) = body.strip_suffix(';') {
            body = b;
        }
        if let Some(b) = body.strip_suffix(')') {
            body = b;
        }
        body = body.trim();
        if let Some(end) = body.rfind(['}', ']']) {
            return serde_json::from_str(&body[..end + 1]).map_err(Error::Json);
        }
    }
    serde_json::from_str(t).map_err(Error::Json)
}

pub(crate) fn parse_global_hist_sina(resp: &Value) -> Result<Vec<GlobalHistSinaRow>> {
    let arr = resp
        .get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "missing result.data".into(),
        })?;
    Ok(arr
        .iter()
        .map(|item| GlobalHistSinaRow {
            date: opt_str(item, "d").unwrap_or_default(),
            open: opt_f64(item, "o"),
            high: opt_f64(item, "h"),
            low: opt_f64(item, "l"),
            close: opt_f64(item, "c"),
            volume: opt_f64(item, "v"),
        })
        .collect())
}

/// 新浪财经-环球市场-历史行情 (akshare `index_global_hist_sina`).
pub async fn index_global_hist_sina(
    client: &Client,
    symbol: &str,
) -> Result<Vec<GlobalHistSinaRow>> {
    let code = GLOBAL_SINA_MAP
        .iter()
        .find(|(name, _)| *name == symbol)
        .map(|(_, code)| *code)
        .ok_or_else(|| Error::InvalidParam(format!("unknown global sina symbol: {symbol}")))?;
    let text = client
        .get_text(
            SOURCE_SINA,
            "index_global_hist_sina",
            "https://gi.finance.sina.com.cn/hq/daily",
            &[("symbol", code), ("num", "10000")],
            None,
        )
        .await?;
    let v = sina_value_from_text(&text)?;
    parse_global_hist_sina(&v)
}

/// Build the static name/code table (pure, no I/O).
pub(crate) fn build_global_name_table() -> Vec<GlobalNameRow> {
    GLOBAL_SINA_MAP
        .iter()
        .map(|(name, code)| GlobalNameRow {
            name: name.to_string(),
            code: code.to_string(),
        })
        .collect()
}

/// 新浪财经-环球市场-名称代码映射表 (akshare `index_global_name_table`).
pub async fn index_global_name_table(_client: &Client) -> Result<Vec<GlobalNameRow>> {
    Ok(build_global_name_table())
}

// ===========================================================================
// Tests
// ===========================================================================

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

    #[test]
    fn parse_sugar_msweet_ok() {
        let rows = parse_sugar_msweet(&fixture("index_sugar_msweet.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2024-01-01");
        assert!(approx(rows[0].composite_price, 100.0));
        assert!(approx(rows[0].raw_sugar_price, 12.88));
        assert!(approx(rows[0].spot_price, 5400.0));
        assert_eq!(rows[2].raw_sugar_price, None);
    }

    #[test]
    fn parse_inner_quote_sugar_msweet_ok() {
        let rows = parse_inner_quote_sugar_msweet(&fixture("index_inner_quote_sugar_msweet.json"))
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024/01/01");
        assert!(approx(rows[0].profit_space, 500.0));
        assert!(approx(rows[0].thai_sugar, 4045.2));
        assert!(approx(rows[0].brazil_sugar, 4200.0));
        assert!(approx(rows[1].liuzhou_spot, 6500.0));
    }

    #[test]
    fn parse_outer_quote_sugar_msweet_ok() {
        let rows = parse_outer_quote_sugar_msweet(&fixture("index_outer_quote_sugar_msweet.json"))
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024/01/01");
        assert!(approx(rows[0].brazil_import_cost, 5500.0));
        assert!(approx(rows[0].thai_profit_space, 200.0));
        assert!(approx(rows[1].rizhao_spot, 6800.0));
    }

    #[test]
    fn parse_kq_fz_price_ok() {
        let data = fixture("index_kq_fz.json")
            .get("result")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap();
        let rows = parse_kq_fz(&data);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].period, "2024-01期");
        assert!(approx(rows[0].index_value, 105.3));
        assert!(approx(rows[0].change, 0.5));
        assert!(approx(rows[1].index_value, 106.1));
    }

    #[test]
    fn parse_kq_fashion_ok() {
        let rows = parse_kq_fashion(&fixture("index_kq_fashion.json"));
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2024-01-01");
        assert!(approx(rows[0].index_value, 152.3));
        assert_eq!(rows[0].change_value, None);
        let filled = kq_fashion_with_changes(rows);
        assert!(approx(filled[1].change_value, 1.4));
        assert!(approx(filled[2].change_pct, 0.01041));
    }

    #[test]
    fn parse_eri_ok() {
        let idx = fixture("index_eri.json")
            .get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap();
        let idx_rows = parse_eri_index(&idx);
        assert_eq!(idx_rows.len(), 2);
        assert_eq!(idx_rows[0].0, "2024-01-31");
        assert!(approx(idx_rows[0].1, 845.6));
        assert!(approx(idx_rows[1].1, 850.2));
    }

    #[test]
    fn parse_yw_bi_ok() {
        let rows = parse_yw(&fixture("index_yw.json"));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].period, "2024-01");
        assert!(approx(rows[0].prosperity_index, 1001.2));
        assert!(approx(rows[0].scale_index, 800.0));
        assert!(approx(rows[1].confidence_index, 950.0));
    }

    #[test]
    fn parse_cflp_ok() {
        let rows = parse_cflp(&fixture("index_price_cflp.json")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].date, "2024-01-01");
        assert!(approx(rows[0].fixed_base_index, 1000.0));
        assert!(approx(rows[0].mom_index, 99.5));
        assert!(approx(rows[2].yoy_index, 101.2));
    }

    #[test]
    fn parse_global_hist_sina_ok() {
        let rows = parse_global_hist_sina(&fixture("index_global_hist_sina.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-02");
        assert!(approx(rows[0].open, 7500.0));
        assert!(approx(rows[0].close, 7550.0));
        assert!(approx(rows[1].volume, 1200000.0));
    }

    #[test]
    fn sina_jsonp_strip_ok() {
        let txt = "var _foo_ = {\"result\":{\"data\":[{\"d\":\"2024-01-02\",\"o\":\"7500.0\",\"h\":\"7600.0\",\"l\":\"7400.0\",\"c\":\"7550.0\",\"v\":\"1000000.0\"}]}};";
        let v = sina_value_from_text(txt).unwrap();
        let rows = parse_global_hist_sina(&v).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].date, "2024-01-02");
        assert!(approx(rows[0].close, 7550.0));
    }

    #[test]
    fn build_global_name_table_ok() {
        let rows = build_global_name_table();
        assert_eq!(rows.len(), GLOBAL_SINA_MAP.len());
        assert_eq!(rows[0].name, "英国富时100指数");
        assert_eq!(rows[0].code, "UKX");
        assert_eq!(rows.last().unwrap().code, "CASE");
    }
}
