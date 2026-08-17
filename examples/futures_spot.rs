//! 期货实时快照(东方财富)-> JSON
//!
//! 运行:`cargo run --example futures_spot`(需联网)

use adaq_data_stock_cn::{Client, convert, futures};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let spot = futures::spot::futures_zh_spot(&client).await?;
    println!("期货实时快照 {} 行", spot.len());

    let sample = &spot[..spot.len().min(3)];
    println!("{}", convert::to_json(sample)?);
    Ok(())
}
