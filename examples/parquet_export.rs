//! 将 A 股日线导出为 Parquet / CSV。
//!
//! Parquet 需 `parquet` 特性:
//!   cargo run --example parquet_export --features parquet
//! 未启用特性时回退为 CSV 输出(始终可编译)。

use adaq_data_stock_cn::{Client, convert, stock};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let hist = stock::hist::daily(&client, "600519", "daily", "", "20240101", "20241231").await?;
    println!("贵州茅台日线 {} 根", hist.len());

    #[cfg(feature = "parquet")]
    {
        let path = std::path::Path::new("600519_daily.parquet");
        convert::to_parquet(&hist, path)?;
        println!("已导出 Parquet -> {}", path.display());
    }

    #[cfg(not(feature = "parquet"))]
    {
        println!("未启用 parquet 特性,输出 CSV:\n");
        println!("{}", convert::to_csv(&hist)?);
        println!(
            "提示:以 `cargo run --example parquet_export --features parquet` 运行可导出 Parquet。"
        );
    }

    Ok(())
}
