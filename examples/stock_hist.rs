//! A 股日线历史(多源降级:东财 -> 腾讯)-> CSV
//!
//! 运行:`cargo run --example stock_hist`(需联网)

use adaq_data_stock_cn::{Client, convert, stock};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    // symbol 为无前缀代码;period: daily/weekly/monthly;adjust: ""/qfq/hfq
    let hist = stock::hist::daily(&client, "600519", "daily", "", "20240101", "20241231").await?;
    println!("贵州茅台日线 {} 根", hist.len());

    println!("{}", convert::to_csv(&hist)?);
    Ok(())
}
