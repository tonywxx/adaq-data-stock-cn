use serde_json::Value;
use std::path::PathBuf;

fn fixture(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let txt = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&txt).unwrap()
}

#[test]
fn hk_spot_fixture_parity() {
    let v = fixture("stock_hk_spot_em.json");
    let diff = v.get("data").and_then(|d| d.get("diff")).and_then(|d| d.as_array()).unwrap();
    assert_eq!(diff.len(), 2);
    assert_eq!(diff[0].get("f12").and_then(|x| x.as_str()).unwrap(), "00593");
}

#[test]
fn us_spot_fixture_parity() {
    let v = fixture("stock_us_spot_em.json");
    let diff_opt = v.get("data").and_then(|d| d.get("diff")).and_then(|d| d.as_array());
    if diff_opt.is_none() {
        return;
    }
    assert!(!diff_opt.unwrap().is_empty());
}

#[tokio::test]
#[ignore]
async fn live_hk_spot_smoke() {
    let client = adaq_data_stock_cn::Client::new();
    let rows = adaq_data_stock_cn::stock::cross::hk::stock_hk_spot_em(&client).await.unwrap();
    assert!(!rows.is_empty());
    assert!(!rows[0].code.is_empty());
}

#[tokio::test]
#[ignore]
async fn live_us_spot_smoke() {
    let client = adaq_data_stock_cn::Client::new();
    let rows = adaq_data_stock_cn::stock::cross::us::stock_us_spot_em(&client).await.unwrap();
    assert!(!rows.is_empty());
}

#[tokio::test]
#[ignore]
async fn live_hsgt_hold_stock_smoke() {
    let client = adaq_data_stock_cn::Client::new();
    let rows =
        adaq_data_stock_cn::stock::hsgt::stock_hsgt_hold_stock_em(&client, "北向", "5日排行", "2024-01-05")
            .await
            .unwrap();
    assert!(!rows.is_empty());
}
