//! 指数历史日线(东方财富)-> CSV
//!
//! 运行:`cargo run --example index_hist`(需联网)

use adaq_data_stock_cn::{Client, convert, index};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    // 上证指数 000001;period: daily/weekly/monthly
    let hist =
        index::index_more::index_zh_a_hist(&client, "000001", "daily", "20240101", "20241231")
            .await?;
    println!("上证指数日线 {} 根", hist.len());

    println!("{}", convert::to_csv(&hist)?);
    Ok(())
}
