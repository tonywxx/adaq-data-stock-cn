//! A 股实时全景快照(东方财富 push2 clist)-> JSON / CSV
//!
//! 运行:`cargo run --example stock_spot`(需联网)

use adaq_data_stock_cn::{Client, convert, stock};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let spot = stock::stock_hist_em::stock_zh_a_spot_em(&client).await?;
    println!("A 股实时快照共 {} 行", spot.len());

    let sample = &spot[..spot.len().min(5)];
    println!("\n--- JSON(前 5) ---\n{}", convert::to_json(sample)?);
    println!("\n--- CSV(前 5) ---\n{}", convert::to_csv(sample)?);
    Ok(())
}
