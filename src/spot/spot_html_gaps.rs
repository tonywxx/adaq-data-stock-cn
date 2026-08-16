//! Spot commodity (搜猪 soozhu) data — akshare `spot/spot_hog_soozhu.py`.
//!
//! All eight functions hit `https://www.soozhu.com/price/data/center/`: a GET
//! to read the `csrfmiddlewaretoken`, then a form `POST` whose `act`/`indid`
//! selects the series. The response is JSON (akshare uses `r.json()`), so the
//! `parse_*` helpers here decode JSON rather than scrape `<table>`s.

use scraper::{Html, Selector};
use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_SOOZHU: &str = "soozhu";

/// Parse a JSON scalar into `f64`, tolerating string-encoded numbers.
fn to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
}

/// GET the soozhu data-center page, extract its CSRF token, then POST the
/// chosen `act`/`indid` and return the JSON body.
async fn soozhu_post(
    client: &Client,
    endpoint: &'static str,
    act: &str,
    indid: Option<&str>,
) -> Result<Value> {
    let page = client
        .get_text(
            SOURCE_SOOZHU,
            endpoint,
            "https://www.soozhu.com/price/data/center/",
            &[],
            None,
        )
        .await?;
    let doc = Html::parse_document(&page);
    let sel = Selector::parse("input[name=\"csrfmiddlewaretoken\"]").map_err(|e| Error::Parse {
        endpoint,
        message: format!("csrf selector: {e}"),
    })?;
    let token = doc
        .select(&sel)
        .next()
        .and_then(|e| e.value().attr("value"))
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SOOZHU,
            message: "csrfmiddlewaretoken not found in page".into(),
        })?;
    let mut params: Vec<(&str, &str)> = vec![("act", act), ("csrfmiddlewaretoken", token)];
    if let Some(id) = indid {
        params.push(("indid", id));
    }
    client
        .post_form_json(
            SOURCE_SOOZHU,
            endpoint,
            "https://www.soozhu.com/price/data/center/",
            &params,
            None,
        )
        .await
}

/// One province's live average hog price (akshare `spot_hog_soozhu`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SpotHogProvince {
    /// 省份 — province name.
    pub province: String,
    /// 价格 — live average price (元/公斤).
    pub price: Option<f64>,
    /// 涨跌幅 — daily change %.
    pub change_pct: Option<f64>,
}

/// One dated price observation shared by the trend endpoints.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SpotHogPrice {
    /// 日期 — observation date (`YYYY-MM-DD`).
    pub date: String,
    /// 价格 — price series value.
    pub price: Option<f64>,
}

/// 搜猪-生猪大数据-各省均价实时排行榜 (`spot_hog_soozhu`, akshare `spot_hog_soozhu.py:14`).
pub async fn spot_hog_soozhu(client: &Client) -> Result<Vec<SpotHogProvince>> {
    let v = soozhu_post(client, "spot_hog_soozhu", "mapdata", None).await?;
    let body = serde_json::to_string(&v).map_err(|e| Error::Parse {
        endpoint: "spot_hog_soozhu",
        message: format!("serialize: {e}"),
    })?;
    parse_spot_hog_soozhu(&body, "spot_hog_soozhu")
}

pub(crate) fn parse_spot_hog_soozhu(html: &str, endpoint: &'static str) -> Result<Vec<SpotHogProvince>> {
    let v: Value = serde_json::from_str(html).map_err(|e| Error::Parse {
        endpoint,
        message: format!("json: {e}"),
    })?;
    let vlist = v
        .get("vlist")
        .and_then(|x| x.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SOOZHU,
            message: "missing vlist".into(),
        })?;
    let mut out = Vec::with_capacity(vlist.len());
    for item in vlist {
        let province = item
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let value = item.get("value").and_then(|x| x.as_array());
        let price = value.and_then(|a| a.first()).and_then(to_f64);
        let change_pct = value.and_then(|a| a.get(1)).and_then(to_f64);
        out.push(SpotHogProvince {
            province,
            price,
            change_pct,
        });
    }
    Ok(out)
}

/// Shared parser for `[日期, 价格]` JSON arrays (`nationlist` / `datalist`).
fn parse_date_price_array(
    v: &Value,
    key: &str,
    _endpoint: &'static str,
) -> Result<Vec<SpotHogPrice>> {
    let list = v
        .get(key)
        .and_then(|x| x.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_SOOZHU,
            message: format!("missing {key}"),
        })?;
    let mut out = Vec::with_capacity(list.len());
    for row in list {
        let a = match row.as_array() {
            Some(a) => a,
            None => continue,
        };
        let date = a
            .first()
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let price = a.get(1).and_then(to_f64);
        out.push(SpotHogPrice { date, price });
    }
    Ok(out)
}

/// 搜猪-生猪大数据-今年以来全国出栏均价走势 (`spot_hog_year_trend_soozhu`, akshare `spot_hog_soozhu.py:41`).
pub async fn spot_hog_year_trend_soozhu(client: &Client) -> Result<Vec<SpotHogPrice>> {
    let v = soozhu_post(client, "spot_hog_year_trend_soozhu", "yeartrend", None).await?;
    let body = serde_json::to_string(&v).map_err(|e| Error::Parse {
        endpoint: "spot_hog_year_trend_soozhu",
        message: format!("serialize: {e}"),
    })?;
    parse_spot_hog_year_trend_soozhu(&body, "spot_hog_year_trend_soozhu")
}

pub(crate) fn parse_spot_hog_year_trend_soozhu(html: &str, endpoint: &'static str) -> Result<Vec<SpotHogPrice>> {
    let v: Value = serde_json::from_str(html).map_err(|e| Error::Parse {
        endpoint,
        message: format!("json: {e}"),
    })?;
    parse_date_price_array(&v, "nationlist", endpoint)
}

/// Shared parser for the `datalist` trend endpoints (lean/thirds/crossbred/
/// corn/soybean/mixed-feed).
pub(crate) fn parse_spot_hog_datalist(html: &str, endpoint: &'static str) -> Result<Vec<SpotHogPrice>> {
    let v: Value = serde_json::from_str(html).map_err(|e| Error::Parse {
        endpoint,
        message: format!("json: {e}"),
    })?;
    parse_date_price_array(&v, "datalist", endpoint)
}

macro_rules! soozhu_datalist_fn {
    ($($name:ident => ($act:literal, $indid:literal, $line:literal)),* $(,)?) => {
        $(
            #[doc = concat!("搜猪-生猪大数据 price-trend series (`", stringify!($name), "`, akshare `spot_hog_soozhu.py:", $line, "`).")]
            pub async fn $name(client: &Client) -> Result<Vec<SpotHogPrice>> {
                let v = soozhu_post(client, stringify!($name), $act, Some($indid)).await?;
                let body = serde_json::to_string(&v).map_err(|e| Error::Parse {
                    endpoint: stringify!($name),
                    message: format!("serialize: {e}"),
                })?;
                parse_spot_hog_datalist(&body, stringify!($name))
            }
        )*
    };
}

soozhu_datalist_fn! {
    spot_hog_lean_price_soozhu => ("pricetrend", "4", "65"),
    spot_hog_three_way_soozhu => ("pricetrend", "4", "89"),
    spot_hog_crossbred_soozhu => ("pricetrend", "6", "113"),
    spot_corn_price_soozhu => ("pricetrend", "8", "137"),
    spot_soybean_price_soozhu => ("pricetrend", "9", "161"),
    spot_mixed_feed_soozhu => ("pricetrend", "11", "185"),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn load_html(name: &str) -> String {
        let bytes = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
        )
        .unwrap();
        match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_string(),
            Err(_) => encoding_rs::GBK
                .decode_without_bom_handling_and_without_replacement(&bytes)
                .unwrap_or_else(|| String::from_utf8_lossy(&bytes))
                .into_owned(),
        }
    }

    #[test]
    fn parses_spot_hog_soozhu() {
        let rows = parse_spot_hog_soozhu(&load_html("spot_hog_soozhu.html"), "spot_hog_soozhu").unwrap();
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].province, "北京");
        assert!((rows[0].price.unwrap() - 15.24).abs() < 1e-9);
        assert!((rows[0].change_pct.unwrap() - 0.32).abs() < 1e-9);
    }

    #[test]
    fn parses_spot_hog_year_trend_soozhu() {
        let rows =
            parse_spot_hog_year_trend_soozhu(&load_html("spot_hog_year_trend_soozhu.html"), "spot_hog_year_trend_soozhu")
                .unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].date, "2024-01-02");
        assert!((rows[0].price.unwrap() - 14.35).abs() < 1e-9);
    }

    #[test]
    fn parses_spot_hog_datalist_fns() {
        let cases = [
            "spot_hog_lean_price_soozhu",
            "spot_hog_three_way_soozhu",
            "spot_hog_crossbred_soozhu",
            "spot_corn_price_soozhu",
            "spot_soybean_price_soozhu",
            "spot_mixed_feed_soozhu",
        ];
        for name in cases {
            let rows = parse_spot_hog_datalist(&load_html(&format!("{name}.html")), name).unwrap();
            assert_eq!(rows.len(), 4, "wrong row count for {name}");
            assert_eq!(rows[0].date, "2024-01-02");
            assert!((rows[0].price.unwrap() - 14.50).abs() < 1e-9);
            assert_eq!(rows[3].date, "2024-01-05");
        }
    }
}
