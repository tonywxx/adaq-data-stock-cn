use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};
use crate::core::json::*;
use crate::forex::{ForexHistRow, ForexSpotQuote};

const FS: &str = "m:119,m:120,m:133";
const FIELDS: &str = "f12,f13,f14,f1,f2,f4,f3,f152,f17,f18,f15,f16";
const SPOT_BASE: &str = "https://push2.eastmoney.com/api/qt/clist/get";
const KLINE_BASE: &str = "https://push2his.eastmoney.com/api/qt/stock/kline/get";
const HIST_UT: &str = "f057cbcbce2a86e2866ab8877db1d059";

/// Real-time FX spot quotes from Eastmoney (`forex_spot_em`). Paginated via `clist/get`.
pub async fn spot(client: &Client) -> Result<Vec<ForexSpotQuote>> {
    let mut out = Vec::new();
    let mut pn: u32 = 1;
    loop {
        let pn_s = pn.to_string();
        let pz_s = "100".to_string();
        let params = [
            ("np", "1"),
            ("fltt", "2"),
            ("invt", "2"),
            ("fs", FS),
            ("fields", FIELDS),
            ("fid", "f3"),
            ("pn", pn_s.as_str()),
            ("pz", pz_s.as_str()),
            ("po", "1"),
            ("dect", "1"),
            ("wbp2u", "|0|0|0|web"),
        ];
        let v = client
            .get_json(SOURCE_EASTMONEY, "forex_spot_em", SPOT_BASE, &params)
            .await?;
        let diff = v
            .get("data")
            .and_then(|d| d.get("diff"))
            .and_then(|d| d.as_array())
            .ok_or_else(|| Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "missing data.diff".into(),
            })?;
        if diff.is_empty() {
            break;
        }
        out.extend(parse_spot_diff(&v)?);
        let total = v
            .get("data")
            .and_then(|d| d.get("total"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        if (pn as u64) * 100 >= total {
            break;
        }
        pn += 1;
        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
    }
    Ok(out)
}

/// Historical FX kline from Eastmoney (`forex_hist_em`). `symbol` is an akshare FX code (e.g. `USDCNH`).
pub async fn hist(client: &Client, symbol: &str) -> Result<Vec<ForexHistRow>> {
    let market = symbol_market(symbol)?;
    let secid = format!("{market}.{symbol}");
    let params = [
        ("secid", secid.as_str()),
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
        ("ut", HIST_UT),
        ("forcect", "1"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "forex_hist_em", KLINE_BASE, &params)
        .await?;
    parse_hist_klines(&v)
}

pub(crate) fn parse_spot_diff(resp: &Value) -> Result<Vec<ForexSpotQuote>> {
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
        out.push(parse_spot_item(item));
    }
    Ok(out)
}

pub(crate) fn parse_hist_klines(resp: &Value) -> Result<Vec<ForexHistRow>> {
    let data = resp.get("data").ok_or_else(|| Error::UpstreamChanged {
        origin: SOURCE_EASTMONEY,
        message: "missing data".into(),
    })?;
    let klines = data
        .get("klines")
        .and_then(|k| k.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.klines".into(),
        })?;
    let code = data
        .get("code")
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();
    let name = data
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();
    let _ = name;
    let mut out = Vec::with_capacity(klines.len());
    for k in klines {
        let s = k.as_str().ok_or_else(|| Error::Parse {
            endpoint: "forex_hist_em",
            message: "kline entry is not a string".into(),
        })?;
        let p: Vec<&str> = s.split(',').collect();
        out.push(ForexHistRow {
            symbol: code.clone(),
            date: p.first().map(|x| x.to_string()).unwrap_or_default(),
            open: p.get(1).and_then(|x| x.parse::<f64>().ok()),
            close: p.get(2).and_then(|x| x.parse::<f64>().ok()),
            high: p.get(3).and_then(|x| x.parse::<f64>().ok()),
            low: p.get(4).and_then(|x| x.parse::<f64>().ok()),
            amplitude: p.get(7).and_then(|x| x.parse::<f64>().ok()),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

fn parse_spot_item(item: &Value) -> ForexSpotQuote {
    ForexSpotQuote {
        code: opt_str_or(item, "f12", ""),
        name: opt_str_or(item, "f14", ""),
        price: opt_f64(item, "f2"),
        change: opt_f64(item, "f4"),
        pct_change: opt_f64(item, "f3"),
        open: opt_f64(item, "f17"),
        high: opt_f64(item, "f15"),
        low: opt_f64(item, "f16"),
        pre_close: opt_f64(item, "f18"),
        source: SOURCE_EASTMONEY,
    }
}

/// Maps an akshare FX symbol to its Eastmoney market code (from `akshare/forex/cons.py`).
fn symbol_market(symbol: &str) -> Result<u32> {
    const MAP: &[(&str, u32)] = &[
        ("EURCNYC", 120),
        ("JPYZAR", 119),
        ("NZDCNYC", 120),
        ("CNYRUBC", 120),
        ("AUDCNYC", 120),
        ("JPYGBP", 119),
        ("JPYSGD", 119),
        ("JPYCNH", 133),
        ("JPYAUD", 119),
        ("USDBRL", 119),
        ("JPYEUR", 119),
        ("JPYTRY", 119),
        ("JPYCAD", 119),
        ("CHFZAR", 119),
        ("JPYHKD", 119),
        ("SEKEUR", 119),
        ("JPYUSD", 119),
        ("GBPCNYC", 120),
        ("JPYNZD", 119),
        ("CHFGBP", 119),
        ("USDIDR", 119),
        ("CHFSGD", 119),
        ("USDPLN", 119),
        ("CHFCNH", 133),
        ("SEKUSD", 119),
        ("CHFAUD", 119),
        ("USDKRW", 119),
        ("EURPLN", 119),
        ("USDHUF", 119),
        ("CHFCAD", 119),
        ("USDTHB", 119),
        ("CHFEUR", 119),
        ("JPYCNYC", 120),
        ("EURHUF", 119),
        ("CHFHKD", 119),
        ("SGDCNYC", 120),
        ("CHFUSD", 119),
        ("USDINR", 119),
        ("USDCZK", 119),
        ("CHFNZD", 119),
        ("USDMXN", 119),
        ("GBPPLN", 119),
        ("USDZAR", 119),
        ("JPYCHF", 119),
        ("EURCZK", 119),
        ("EURZAR", 119),
        ("CADCNYC", 120),
        ("NOKEUR", 119),
        ("NZDGBP", 119),
        ("NOKUSD", 119),
        ("NZDSGD", 119),
        ("USDGBP", 119),
        ("HKDGBP", 119),
        ("NZDCNH", 133),
        ("NZDAUD", 119),
        ("HKDSGD", 119),
        ("CNYSARC", 120),
        ("USDSGD", 119),
        ("CNYAEDC", 120),
        ("EURGBP", 119),
        ("CADGBP", 119),
        ("USDCNH", 133),
        ("CNYTRYC", 120),
        ("CADSGD", 119),
        ("USDAUD", 119),
        ("GBPZAR", 119),
        ("EURSGD", 119),
        ("HKDCNH", 133),
        ("NZDCAD", 119),
        ("CADCNH", 133),
        ("HKDAUD", 119),
        ("NZDEUR", 119),
        ("EURCNH", 133),
        ("EURAUD", 119),
        ("NZDHKD", 119),
        ("CADAUD", 119),
        ("AUDGBP", 119),
        ("USDDKK", 119),
        ("HKDCAD", 119),
        ("USDCAD", 119),
        ("AUDSGD", 119),
        ("USDTRY", 119),
        ("EURTRY", 119),
        ("USDEUR", 119),
        ("NZDUSD", 119),
        ("SGDGBP", 119),
        ("USDHKD", 119),
        ("AUDCNH", 133),
        ("EURDKK", 119),
        ("USDARS", 119),
        ("USDSAR", 119),
        ("TRYUSD", 119),
        ("TRYEUR", 119),
        ("SARUSD", 119),
        ("INRUSD", 119),
        ("HUFUSD", 119),
        ("HUFEUR", 119),
        ("HKDUSD", 119),
        ("HKDEUR", 119),
        ("HKDCNYC", 120),
        ("EURCAD", 119),
        ("DKKUSD", 119),
        ("DKKEUR", 119),
        ("CNYMOPC", 120),
        ("CNHSGD", 133),
        ("CNHGBP", 133),
        ("CNHAUD", 133),
        ("CADEUR", 119),
        ("SGDCNH", 133),
        ("EURHKD", 119),
        ("CADHKD", 119),
        ("USDCNYC", 120),
        ("GBPSGD", 119),
        ("EURUSD", 119),
        ("SGDAUD", 119),
        ("HKDNZD", 119),
        ("USDNZD", 119),
        ("GBPCNH", 133),
        ("CADUSD", 119),
        ("AUDCAD", 119),
        ("CNYTHBC", 120),
        ("CNHEUR", 133),
        ("GBPAUD", 119),
        ("AUDEUR", 119),
        ("CADNZD", 119),
        ("EURNZD", 119),
        ("CNHCAD", 133),
        ("AUDHKD", 119),
        ("SGDCAD", 119),
        ("AUDUSD", 119),
        ("SGDEUR", 119),
        ("CNHHKD", 133),
        ("GBPCAD", 119),
        ("CNHUSD", 133),
        ("SGDHKD", 119),
        ("GBPEUR", 119),
        ("SGDUSD", 119),
        ("AUDNZD", 119),
        ("GBPHKD", 119),
        ("GBPUSD", 119),
        ("CNHNZD", 133),
        ("CHFCNYC", 120),
        ("SGDNZD", 119),
        ("ZARGBP", 119),
        ("USDNOK", 119),
        ("GBPNZD", 119),
        ("CZKEUR", 119),
        ("EURNOK", 119),
        ("CHFJPY", 119),
        ("NZDCHF", 119),
        ("PLNGBP", 119),
        ("HKDCHF", 119),
        ("ZARUSD", 119),
        ("USDCHF", 119),
        ("ZAREUR", 119),
        ("MXNUSD", 119),
        ("EURCHF", 119),
        ("CADCHF", 119),
        ("CZKUSD", 119),
        ("CNYKRWC", 120),
        ("CNHCHF", 133),
        ("AUDCHF", 119),
        ("PLNEUR", 119),
        ("CNYMXNC", 120),
        ("SGDCHF", 119),
        ("PLNUSD", 119),
        ("USDSEK", 119),
        ("GBPCHF", 119),
        ("EURSEK", 119),
        ("CNYMYRC", 120),
        ("NZDJPY", 119),
        ("ZARCHF", 119),
        ("USDJPY", 119),
        ("THBUSD", 119),
        ("HKDJPY", 119),
        ("EURJPY", 119),
        ("CADJPY", 119),
        ("AUDJPY", 119),
        ("TRYJPY", 119),
        ("CNHJPY", 133),
        ("SGDJPY", 119),
        ("GBPJPY", 119),
        ("CNYZARC", 120),
        ("ZARJPY", 119),
        ("USDRUB", 119),
        ("CNYDKKC", 120),
        ("CNYNOKC", 120),
        ("CNYHUFC", 120),
        ("CNYPLNC", 120),
        ("CNYSEKC", 120),
    ];
    MAP.iter()
        .find(|(s, _)| *s == symbol)
        .map(|(_, m)| *m)
        .ok_or_else(|| Error::InvalidParam(format!("unknown forex symbol: {symbol}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_forex_spot_fixture() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/forex_spot_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_spot_diff(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "USDCNH");
        assert_eq!(rows[0].name, "美元兑离岸人民币");
        assert_eq!(rows[0].price, Some(7.12));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].code, "EURCNY");
        assert_eq!(rows[1].pct_change, Some(-0.38));
    }

    #[test]
    fn parses_forex_hist_fixture() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/forex_hist_em.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_hist_klines(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].symbol, "USDCNH");
        assert_eq!(rows[0].date, "2024-01-02");
        assert_eq!(rows[0].open, Some(7.1000));
        assert_eq!(rows[0].close, Some(7.1200));
        assert_eq!(rows[0].amplitude, Some(0.40));
        assert_eq!(rows[1].close, Some(7.1100));
    }
}
