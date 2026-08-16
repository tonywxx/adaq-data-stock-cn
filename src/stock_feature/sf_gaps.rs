//! `stock_feature` gap fillers — THS board names, Sina classification, Eniu HK indicators.
//!
//! Ports four akshare `stock_feature` functions whose upstreams are plain HTTP
//! (no `xq_a_token` gating):
//!
//! * [`stock_board_concept_name_ths`] — akshare `stock_feature/stock_board_concept_ths.py:71`
//!   (THS concept board `cate_inner` HTML).
//! * [`stock_board_industry_name_ths`] — akshare `stock_feature/stock_board_industry_ths.py:68`
//!   (THS industry board `cate_inner` HTML).
//! * [`stock_classify_sina`] — akshare `stock_feature/stock_classify_sina.py:48`
//!   (Sina `getHQNodeData` JSON).
//! * [`stock_hk_indicator_eniu`] — akshare `stock_feature/stock_a_indicator.py:54`
//!   (Eniu chart JSON).
//!
//! NOTE on THS HTML: `q.10jqka.com.cn` serves `gbk`-encoded HTML. reqwest (no
//! `encoding_rs` direct dependency) decodes the live body as UTF-8, so Chinese
//! *names* come back mojibake on the live path. The `cate_inner` structure and
//! numeric board *codes* are ASCII and always parse correctly. Fixtures here are
//! pre-decoded to UTF-8 so the parser logic + code extraction are verified; to
//! fix live names, add `encoding_rs` and decode `gbk` before `Html::parse_document`.
//! The optional cookie/`__stock_board_*_summary_ths` union (JS `v`-cookie +
//! paginated) is intentionally omitted (best-effort, see report).

use scraper::{Html, Selector};
use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_THS: &str = "ths";
const SOURCE_SINA: &str = "sina";
const SOURCE_ENIU: &str = "eniu";

/// Parse a JSON scalar into `f64`, tolerating string-encoded numbers.
fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// Read a string field by key (empty string when absent).
fn sfield(item: &Value, k: &str) -> String {
    item.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

/// Strip a single layer of inline HTML tags (e.g. `<font ...>沪港通</font>`).
fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

// ---------------------------------------------------------------------------
// THS board names (concept / industry) — `cate_inner` HTML scrape
// ---------------------------------------------------------------------------

/// One THS board entry: display `name` + numeric `code`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ThsBoardName {
    /// Board display name (akshare `name`). Live THS returns gbk→mojibake here.
    pub name: String,
    /// Board numeric code (akshare `code`, ASCII, always reliable).
    pub code: String,
}

/// Shared parser for the THS `cate_inner` link list (concept & industry share
/// the same DOM; only the URL/path differs).
fn parse_ths_cate_inner(
    html: &str,
    origin: &'static str,
    endpoint: &'static str,
) -> Result<Vec<ThsBoardName>> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse(".cate_inner a").map_err(|e| Error::Parse {
        endpoint,
        message: format!("invalid selector: {e}"),
    })?;
    let mut out = Vec::new();
    for a in doc.select(&sel) {
        let Some(href) = a.value().attr("href") else {
            continue;
        };
        // href like `/gn/detail/code/301558/` → board code `301558`.
        let code = href
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string();
        if code.is_empty() {
            continue;
        }
        let name: String = a.text().collect::<String>().trim().to_string();
        out.push(ThsBoardName { name, code });
    }
    if out.is_empty() {
        return Err(Error::UpstreamChanged {
            origin,
            message: "no board links found in cate_inner".into(),
        });
    }
    Ok(out)
}

/// 同花顺-概念板块-概念 (`stock_board_concept_name_ths`, akshare `stock_board_concept_ths.py:71`).
pub async fn stock_board_concept_name_ths(client: &Client) -> Result<Vec<ThsBoardName>> {
    let url = "https://q.10jqka.com.cn/gn/detail/code/307822/";
    let html = client
        .get_text(SOURCE_THS, "stock_board_concept_name_ths", url, &[], None)
        .await?;
    parse_ths_cate_inner(html.as_str(), SOURCE_THS, "stock_board_concept_name_ths")
}

/// 同花顺-行业板块-行业 (`stock_board_industry_name_ths`, akshare `stock_board_industry_ths.py:68`).
pub async fn stock_board_industry_name_ths(client: &Client) -> Result<Vec<ThsBoardName>> {
    let url = "https://q.10jqka.com.cn/thshy/detail/code/881272/";
    let html = client
        .get_text(
            SOURCE_THS,
            "stock_board_industry_name_ths",
            url,
            &[],
            None,
        )
        .await?;
    parse_ths_cate_inner(
        html.as_str(),
        SOURCE_THS,
        "stock_board_industry_name_ths",
    )
}

// ---------------------------------------------------------------------------
// Sina classification — `getHQNodes` tree + `getHQNodeData` stock lists
// ---------------------------------------------------------------------------

/// One stock within a Sina classification node.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockClassifySina {
    /// Exchange-prefixed symbol, e.g. `sh603228` (akshare `symbol`).
    pub symbol: String,
    /// Numeric code, e.g. `603228` (akshare `code`).
    pub code: String,
    /// Stock name (akshare `name`).
    pub name: String,
    /// Latest price (akshare `trade`).
    pub trade: Option<f64>,
    /// Absolute price change (akshare `pricechange`).
    pub pricechange: Option<f64>,
    /// Percent change (akshare `changepercent`).
    pub changepercent: Option<f64>,
    /// Volume (akshare `volume`).
    pub volume: Option<f64>,
    /// Turnover amount (akshare `amount`).
    pub amount: Option<f64>,
    /// Turnover ratio % (akshare `turnoverratio`).
    pub turnoverratio: Option<f64>,
    /// P/E (akshare `per`).
    pub per: Option<f64>,
    /// P/B (akshare `pb`).
    pub pb: Option<f64>,
    /// Total market cap (akshare `mktcap`).
    pub mktcap: Option<f64>,
    /// Negotiable (float) market cap (akshare `nmc`).
    pub nmc: Option<f64>,
    /// Classification node name this stock belongs to (akshare `class`).
    pub class: String,
}

/// Parse a single `Market_Center.getHQNodeData` response (JSON array of stock
/// objects) into [`StockClassifySina`] rows, tagging each with its `class` name.
pub(crate) fn parse_stock_classify_sina(resp: &Value, class: &str) -> Vec<StockClassifySina> {
    let Some(rows) = resp.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(rows.len());
    for item in rows {
        out.push(StockClassifySina {
            symbol: sfield(item, "symbol"),
            code: sfield(item, "code"),
            name: sfield(item, "name"),
            trade: item.get("trade").and_then(as_f64),
            pricechange: item.get("pricechange").and_then(as_f64),
            changepercent: item.get("changepercent").and_then(as_f64),
            volume: item.get("volume").and_then(as_f64),
            amount: item.get("amount").and_then(as_f64),
            turnoverratio: item.get("turnoverratio").and_then(as_f64),
            per: item.get("per").and_then(as_f64),
            pb: item.get("pb").and_then(as_f64),
            mktcap: item.get("mktcap").and_then(as_f64),
            nmc: item.get("nmc").and_then(as_f64),
            class: class.to_string(),
        });
    }
    out
}

/// Recursively collect leaf nodes `[name, _, code, _]` beneath a Sina class
/// container, returning `(class_name, node_code)` pairs.
fn collect_sina_leaves(v: &Value, out: &mut Vec<(String, String)>) {
    let Some(arr) = v.as_array() else {
        return;
    };
    // Leaf: >=3 elements, `name` at [0], `code` at [2], and [1] is NOT an array
    // (a class container looks like `[classname, [subtrees...]]`).
    if arr.len() >= 3 {
        if let (Some(name), Some(code)) = (
            arr.get(0).and_then(|x| x.as_str()),
            arr.get(2).and_then(|x| x.as_str()),
        ) {
            let is_container = arr.get(1).map_or(false, |x| x.is_array());
            if !is_container && !code.is_empty() {
                out.push((strip_html(name), code.to_string()));
                return;
            }
        }
    }
    for el in arr {
        collect_sina_leaves(el, out);
    }
}

/// Locate the leaf node `(name, code)` list for a Sina classification `symbol`.
fn find_sina_leaves(nodes: &Value, symbol: &str) -> Result<Vec<(String, String)>> {
    let classes = nodes
        .as_array()
        .and_then(|a| a.get(1))
        .and_then(|v| v.get(0))
        .and_then(|v| v.get(1))
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SINA,
            message: "unexpected getHQNodes shape".into(),
        })?;
    for c in classes {
        let cname = c.get(0).and_then(|v| v.as_str()).unwrap_or("");
        if strip_html(cname) == symbol {
            let mut out = Vec::new();
            collect_sina_leaves(c, &mut out);
            return Ok(out);
        }
    }
    Err(Error::Parse {
        endpoint: "stock_classify_sina",
        message: format!("symbol {symbol} not found in Sina classification tree"),
    })
}

/// 新浪财经-股票分类 (`stock_classify_sina`, akshare `stock_classify_sina.py:48`).
///
/// Mirrors akshare: fetch the `getHQNodes` tree, find the `symbol` class's leaf
/// nodes, then paginate each node's `getHQNodeData` and tag rows with the leaf
/// node name.
pub async fn stock_classify_sina(
    client: &Client,
    symbol: &str,
) -> Result<Vec<StockClassifySina>> {
    let nodes = client
        .get_json(
            SOURCE_SINA,
            "stock_classify_sina_nodes",
            "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodes",
            &[],
        )
        .await?;
    let leaves = find_sina_leaves(&nodes, symbol)?;

    const NODE_URL: &str =
        "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeData";
    let mut out = Vec::new();
    for (class_name, node_code) in leaves {
        // Page count: ceil(count / 80).
        let count: f64 = client
            .get_json(
                SOURCE_SINA,
                "stock_classify_sina_count",
                "https://vip.stock.finance.sina.com.cn/quotes_service/api/json_v2.php/Market_Center.getHQNodeStockCount",
                &[("node", node_code.as_str())],
            )
            .await?
            .as_f64()
            .unwrap_or(0.0);
        let pages = (count / 80.0).ceil().max(1.0) as u32;
        for page in 1..=pages {
            let data = client
                .get_json(
                    SOURCE_SINA,
                    "stock_classify_sina",
                    NODE_URL,
                    &[
                        ("page", page.to_string().as_str()),
                        ("num", "80"),
                        ("sort", "symbol"),
                        ("asc", "1"),
                        ("node", node_code.as_str()),
                        ("symbol", ""),
                        ("_s_r_a", "init"),
                    ],
                )
                .await?;
            out.extend(parse_stock_classify_sina(&data, &class_name));
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Eniu HK indicator — `chart/<indicator>/<symbol>` JSON
// ---------------------------------------------------------------------------

/// One dated observation of a HK stock indicator from Eniu.
///
/// Eniu returns `{date, <indicator>, price?}`; the value-series key varies by
/// indicator (`pe`, `pb`, `dv`, `roe`+`expect_roe`, `market_value`). All are
/// captured as optional columns so a single row type covers every indicator.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StockHkIndicatorEniuRow {
    /// Observation date (`YYYY-MM-DD`).
    pub date: String,
    /// 市盈率 (akshare `pe`).
    pub pe: Option<f64>,
    /// 市净率 (akshare `pb`).
    pub pb: Option<f64>,
    /// 股息率 (akshare `dv`).
    pub dv: Option<f64>,
    /// ROE (akshare `roe`).
    pub roe: Option<f64>,
    /// Expected ROE (akshare `expect_roe`).
    pub expect_roe: Option<f64>,
    /// Market value (akshare `market_value`).
    pub market_value: Option<f64>,
    /// Price (akshare `price`, absent for `marketvalueh`).
    pub price: Option<f64>,
}

/// Map an akshare `indicator` label to the Eniu chart path segment.
fn eniu_path(indicator: &str) -> &'static str {
    match indicator {
        "市盈率" => "peh",
        "市净率" => "pbh",
        "股息率" => "dvh",
        "ROE" => "roeh",
        _ => "marketvalueh",
    }
}

/// 亿牛网-港股指标 (`stock_hk_indicator_eniu`, akshare `stock_a_indicator.py:54`).
pub async fn stock_hk_indicator_eniu(
    client: &Client,
    symbol: &str,
    indicator: &str,
) -> Result<Vec<StockHkIndicatorEniuRow>> {
    let url = format!(
        "https://eniu.com/chart/{}/{symbol}",
        eniu_path(indicator)
    );
    let v = client
        .get_json(SOURCE_ENIU, "stock_hk_indicator_eniu", &url, &[])
        .await?;
    parse_stock_hk_indicator_eniu(&v)
}

/// Parse an Eniu chart JSON (`{date:[...], <series>:[...], ...}`) into rows.
pub(crate) fn parse_stock_hk_indicator_eniu(resp: &Value) -> Result<Vec<StockHkIndicatorEniuRow>> {
    let date = resp
        .get("date")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_ENIU,
            message: "missing date series".into(),
        })?;
    let n = date.len();
    let series = |k: &str, i: usize| -> Option<f64> {
        resp.get(k)
            .and_then(|v| v.as_array())
            .and_then(|a| a.get(i))
            .and_then(as_f64)
    };
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(StockHkIndicatorEniuRow {
            date: date
                .get(i)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            pe: series("pe", i),
            pb: series("pb", i),
            dv: series("dv", i),
            roe: series("roe", i),
            expect_roe: series("expect_roe", i),
            market_value: series("market_value", i),
            price: series("price", i),
        });
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

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    /// Load a JSON fixture as `Value` (sina / eniu).
    fn fixture_json(name: &str) -> Value {
        let p = fixture_path(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    /// Load an HTML fixture as text (ths — file stores decoded UTF-8 HTML).
    fn fixture_text(name: &str) -> String {
        let p = fixture_path(name);
        std::fs::read_to_string(p).unwrap()
    }

    #[test]
    fn parses_stock_board_concept_name_ths() {
        let rows = parse_ths_cate_inner(
            &fixture_text("stock_board_concept_name_ths.json"),
            SOURCE_THS,
            "stock_board_concept_name_ths",
        )
        .unwrap();
        assert!(rows.len() > 100, "expected many concept boards, got {}", rows.len());
        // 阿里巴巴概念 → 301558 (seen in the captured page).
        let ali = rows
            .iter()
            .find(|r| r.code == "301558")
            .expect("阿里巴巴概念 (301558) missing");
        assert_eq!(ali.name, "阿里巴巴概念");
    }

    #[test]
    fn parses_stock_board_industry_name_ths() {
        let rows = parse_ths_cate_inner(
            &fixture_text("stock_board_industry_name_ths.json"),
            SOURCE_THS,
            "stock_board_industry_name_ths",
        )
        .unwrap();
        assert!(rows.len() > 50, "expected many industry boards, got {}", rows.len());
        // 半导体 → 881121.
        let semi = rows
            .iter()
            .find(|r| r.code == "881121")
            .expect("半导体 (881121) missing");
        assert_eq!(semi.name, "半导体");
    }

    #[test]
    fn parses_stock_classify_sina() {
        // Fixture is one node's getHQNodeData response (热门概念 → 历史新高).
        let rows = parse_stock_classify_sina(&fixture_json("stock_classify_sina.json"), "历史新高");
        assert_eq!(rows.len(), 11);
        let first = &rows[0];
        assert_eq!(first.symbol, "sh603228");
        assert_eq!(first.code, "603228");
        assert_eq!(first.name, "景旺电子");
        assert_eq!(first.class, "历史新高");
        assert!((first.trade.unwrap() - 98.01).abs() < 1e-6);
        assert!(first.per.is_some());
        assert!(first.pb.is_some());
    }

    #[test]
    fn parses_stock_hk_indicator_eniu() {
        let rows = parse_stock_hk_indicator_eniu(&fixture_json("stock_hk_indicator_eniu.json")).unwrap();
        assert_eq!(rows.len(), 3740);
        assert_eq!(rows[0].date, "2007-04-12");
        assert!((rows[0].pe.unwrap() - 176.47).abs() < 1e-6);
        assert!(rows[0].price.is_some());
        // Last row sanity.
        assert!(rows.last().unwrap().date.len() == 10);
    }
}
