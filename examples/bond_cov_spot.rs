//! 沪深可转债实时快照(新浪)-> JSON
//!
//! 运行:`cargo run --example bond_cov_spot`(需联网)

use adaq_data_stock_cn::{Client, bond, convert};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let spot = bond::cov::bond_zh_hs_cov_spot(&client).await?;
    println!("可转债实时快照 {} 行", spot.len());

    let sample = &spot[..spot.len().min(3)];
    println!("{}", convert::to_json(sample)?);
    Ok(())
}
