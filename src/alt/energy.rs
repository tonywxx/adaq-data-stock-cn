//! Energy data (akshare `energy/energy_oil_em.py`, Eastmoney `datacenter-web`).
//!
//! Two prominent public functions are ported, both hitting the Eastmoney
//! data-center endpoint with a `reportName` pair (same shape as
//! `crate::economic::china`). No JS signing is required.
//!
//! - `energy_oil_hist` — historical gasoline/diesel price adjustments
//!   (Eastmoney `RPTA_WEB_YJ_BD`).
//! - `energy_oil_detail` — per-region gasoline/diesel prices on a given
//!   adjustment date (Eastmoney `RPTA_WEB_YJ_JH`).
//!
//! The upstream Eastmoney column codes are `DIM_DATE`, `VALUE`, `CY_JG`,
//! `QY_FD`, `CY_FD` (hist) and `CITYNAME` + `V0/V89/V92/V95` +
//! `ZDE0/ZDE89/ZDE92/ZDE95` + `QE0/QE89/QE92/QE95` (detail).

use serde_json::Value;

use crate::alt::{fnum, fstr};
use crate::core::client::{Client, SOURCE_EASTMONEY};
use crate::core::error::{Error, Result};

const BASE: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";
const TOKEN: &str = "894050c76af8597a853f5b408b759f5d";

/// Extract `result.data` (the row array) from a datacenter-web response.
fn data_array(resp: &Value) -> Result<&Vec<Value>> {
    resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing result.data".into(),
        })
}

// ---------------------------------------------------------------------------
// energy_oil_hist
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct EnergyOilHist {
    /// Adjustment date, e.g. `2024-01-18 00:00:00`.
    pub date: String,
    /// Gasoline price.
    pub gasoline_price: Option<f64>,
    /// Diesel price.
    pub diesel_price: Option<f64>,
    /// Gasoline price change vs. previous adjustment.
    pub gasoline_change: Option<f64>,
    /// Diesel price change vs. previous adjustment.
    pub diesel_change: Option<f64>,
    pub source: &'static str,
}

/// Historical gasoline/diesel price adjustments (`energy_oil_hist`, Eastmoney `RPTA_WEB_YJ_BD`).
pub async fn energy_oil_hist(client: &Client) -> Result<Vec<EnergyOilHist>> {
    let params: [(&str, &str); 11] = [
        ("reportName", "RPTA_WEB_YJ_BD"),
        ("columns", "ALL"),
        ("sortColumns", "dim_date"),
        ("sortTypes", "-1"),
        ("token", TOKEN),
        ("pageNumber", "1"),
        ("pageSize", "1000"),
        ("source", "WEB"),
        ("p", "1"),
        ("pageNo", "1"),
        ("pageNum", "1"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "energy_oil_hist", BASE, &params)
        .await?;
    parse_energy_oil_hist(&v)
}

pub(crate) fn parse_energy_oil_hist(resp: &Value) -> Result<Vec<EnergyOilHist>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = fstr(item, "DIM_DATE") else {
            continue;
        };
        out.push(EnergyOilHist {
            date,
            gasoline_price: fnum(item, "VALUE"),
            diesel_price: fnum(item, "CY_JG"),
            gasoline_change: fnum(item, "QY_FD"),
            diesel_change: fnum(item, "CY_FD"),
            source: SOURCE_EASTMONEY,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// energy_oil_detail
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct EnergyOilDetail {
    /// Adjustment date, e.g. `2022-05-17 00:00:00`.
    pub date: String,
    /// Region / city name.
    pub region: String,
    /// #0 gasoline price.
    pub v0: Option<f64>,
    /// #89 gasoline price.
    pub v89: Option<f64>,
    /// #92 gasoline price.
    pub v92: Option<f64>,
    /// #95 gasoline price.
    pub v95: Option<f64>,
    /// #0 gasoline change.
    pub zde0: Option<f64>,
    /// #89 gasoline change.
    pub zde89: Option<f64>,
    /// #92 gasoline change.
    pub zde92: Option<f64>,
    /// #95 gasoline change.
    pub zde95: Option<f64>,
    /// #0 diesel change.
    pub qe0: Option<f64>,
    /// #89 diesel change.
    pub qe89: Option<f64>,
    /// #92 diesel change.
    pub qe92: Option<f64>,
    /// #95 diesel change.
    pub qe95: Option<f64>,
    pub source: &'static str,
}

/// Per-region gasoline/diesel prices on an adjustment date
/// (`energy_oil_detail`, Eastmoney `RPTA_WEB_YJ_JH`).
///
/// `date` is an 8-digit string, e.g. `"20220517"` (matches akshare's default
/// and the values returned by [`energy_oil_hist`]).
pub async fn energy_oil_detail(client: &Client, date: &str) -> Result<Vec<EnergyOilDetail>> {
    let date_iso = if date.len() == 8 {
        format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..8])
    } else {
        date.to_string()
    };
    let filter = format!("(dim_date='{date_iso}')");
    let params: Vec<(&str, &str)> = vec![
        ("reportName", "RPTA_WEB_YJ_JH"),
        ("columns", "ALL"),
        ("filter", filter.as_str()),
        ("sortColumns", "cityname"),
        ("sortTypes", "1"),
        ("token", TOKEN),
        ("pageNumber", "1"),
        ("pageSize", "1000"),
        ("source", "WEB"),
    ];
    let v = client
        .get_json(SOURCE_EASTMONEY, "energy_oil_detail", BASE, &params)
        .await?;
    parse_energy_oil_detail(&v)
}

pub(crate) fn parse_energy_oil_detail(resp: &Value) -> Result<Vec<EnergyOilDetail>> {
    let data = data_array(resp)?;
    let mut out = Vec::with_capacity(data.len());
    for item in data {
        let Some(date) = fstr(item, "DIM_DATE") else {
            continue;
        };
        let Some(region) = fstr(item, "CITYNAME") else {
            continue;
        };
        out.push(EnergyOilDetail {
            date,
            region,
            v0: fnum(item, "V0"),
            v89: fnum(item, "V89"),
            v92: fnum(item, "V92"),
            v95: fnum(item, "V95"),
            zde0: fnum(item, "ZDE0"),
            zde89: fnum(item, "ZDE89"),
            zde92: fnum(item, "ZDE92"),
            zde95: fnum(item, "ZDE95"),
            qe0: fnum(item, "QE0"),
            qe89: fnum(item, "QE89"),
            qe92: fnum(item, "QE92"),
            qe95: fnum(item, "QE95"),
            source: SOURCE_EASTMONEY,
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
    fn parses_energy_oil_hist() {
        let rows = parse_energy_oil_hist(&fixture("energy_oil_hist.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2024-01-18 00:00:00");
        assert_eq!(rows[0].gasoline_price, Some(10170.0));
        assert_eq!(rows[0].diesel_price, Some(9095.0));
        assert_eq!(rows[0].gasoline_change, Some(200.0));
        assert_eq!(rows[0].diesel_change, Some(190.0));
        assert_eq!(rows[0].source, "eastmoney");
        assert_eq!(rows[1].date, "2023-12-20 00:00:00");
    }

    #[test]
    fn parses_energy_oil_detail() {
        let rows = parse_energy_oil_detail(&fixture("energy_oil_detail.json")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].date, "2022-05-17 00:00:00");
        assert_eq!(rows[0].region, "北京");
        assert_eq!(rows[0].v92, Some(9.01));
        assert_eq!(rows[0].v95, Some(9.27));
        assert_eq!(rows[1].region, "上海");
        assert_eq!(rows[1].v89, Some(8.4));
    }
}
