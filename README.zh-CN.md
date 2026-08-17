# adaq-data-stock-cn

> 📖 English documentation: [README.md](README.md)

akshare 的 Rust 重写,作为量化平台 **AdaQ** 的 A 股市场数据层。

以纯 Rust 重新实现 akshare 的全部对外公开接口,覆盖东方财富、新浪、腾讯、各交易所、中国货币网等上游。所有端点返回**类型化结构体**,可经转换层落 **JSON / CSV / Parquet**。`cargo add` 即可作为库引入。

## 对标进度

完整对照见 [`docs/MAPPING.md`](docs/MAPPING.md)——它既是覆盖率追踪器,也是上游同步锚点(见 ADR-0012)。以下为当前统计(共 1172 个 akshare 顶层函数):

| 状态 | 数量 | 说明 |
|---|---:|---|
| `DONE` | 944 | 已完整移植为 Rust 端点 |
| `DEFERRED` | 156 | 受签名 / 令牌 / JS 执行(按 ADR-0005 逆为纯 Rust) / HTML / Excel 限制,按 ADR-0005/0008 设计内推迟 |
| `INTERNAL` | 72 | akshare 内部辅助函数,非对外数据端点,不计入覆盖口径 |
| `UNKNOWN` | 0 | 已全部清零(见 `git log` "clear all UNKNOWN") |

> 公开数据端点覆盖率 = 944 / (1172 − 72) ≈ **85.8%**。剩余 14.2% 为设计内推迟(需签名/令牌逆向,或把 JS 执行端点按 ADR-0005 逆为纯 Rust——不内嵌 JS 引擎),不含 72 个内部辅助函数。
> 注:3 个 akshare 内部模块(`utils/demjson`、`futures/symbol_var`)此前被误标为 `DONE`,已在本轮修正为 `INTERNAL`。

## 安装

```toml
[dependencies]
adaq-data-stock-cn = "0.1"
tokio = { version = "1", features = ["full"] }   # 异步运行时(示例用 macros)
```

可选特性:

- `parquet` — 启用 Parquet 导出(默认关闭,保持核心精简,见 ADR-0001/0014)。

```toml
adaq-data-stock-cn = { version = "0.1", features = ["parquet"] }
```

## 快速开始

所有端点的第一个参数是共享的 `Client`(内置重试/退避、按源限流、并发上限、可选磁盘缓存,见 ADR-0009)。

```rust
use adaq_data_stock_cn::{Client, convert, stock};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    // A 股实时全景快照(东方财富 push2 clist)
    let spot = stock::stock_hist_em::stock_zh_a_spot_em(&client).await?;
    println!("快照 {} 行", spot.len());

    // 取一条样例并序列化为 JSON / CSV
    let sample = &spot[..spot.len().min(5)];
    println!("{}", convert::to_json(sample)?);
    println!("{}", convert::to_csv(sample)?);
    Ok(())
}
```

## 输出格式

转换层(`adaq_data_stock_cn::convert`)把任意 `Serialize` 行类型序列化为三种格式:

| 函数 | 格式 | 备注 |
|---|---|---|
| `convert::to_json(rows)` | JSON 数组 | 始终可用 |
| `convert::to_csv(rows)` | CSV | 表头取自结构体字段名 |
| `convert::to_parquet(rows, path)` | Parquet 文件 | 需 `parquet` 特性 |

## 多源降级

部分端点内置源链(主源失败自动降级,见 ADR-0010)。例如 A 股日线历史优先东方财富,失败回退腾讯:

```rust
use adaq_data_stock_cn::{Client, stock};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    // symbol 为无前缀代码,period: daily/weekly/monthly,adjust: ""/"qfq"/"hfq"
    let hist = stock::hist::daily(&client, "600519", "daily", "", "20240101", "20241231").await?;
    println!("贵州茅台 {} 根日 K", hist.len());
    Ok(())
}
```

## 示例

可运行的端到端示例见 [`examples/`](examples/),通过 `cargo run --example <name>` 执行:

| 示例 | 端点 | 说明 |
|---|---|---|
| `stock_spot` | `stock_zh_a_spot_em` | A 股实时全景 → JSON / CSV |
| `stock_hist` | `stock::hist::daily` | A 股日线历史(多源降级)→ CSV |
| `bond_cov_spot` | `bond_zh_hs_cov_spot` | 沪深可转债实时快照 |
| `index_hist` | `index_zh_a_hist` | 指数历史日线 |
| `futures_spot` | `futures_zh_spot` | 期货实时快照 |
| `parquet_export` | `convert::to_parquet` | Parquet 导出(需 `--features parquet`,本轮已修复该特性编译) |
| `impersonate_smoke` | `ImpersonateClient` | 浏览器指纹模拟后端实测(新浪 GBK / 百度 / 腾讯 gtimg) |

> 示例会真实请求上游,需联网。无网环境下仅验证编译(`cargo build --examples`)即可。

## 浏览器指纹模拟(反爬)后端

默认 `Client` 走 `reqwest` + rustls,其 TLS/HTTP2 握手易被反爬中间盒按指纹拦截。本库另提供
**浏览器模拟 HTTP 后端**——Rust 版的 [`primp`](https://github.com/deedy5/primp)(`curl_cffi`)
实现:基于 `curl-impersonate` 重放真实 Chrome 的 ClientHello,使请求像真实浏览器。

```rust
use adaq_data_stock_cn::ImpersonateClient;

let client = ImpersonateClient::new(); // 模拟 Chrome 131,内置真实 UA/Accept/Accept-Language
let html = client
    .get_text("https://hq.sinajs.cn/list=sh600000",
               Some(&[adaq_data_stock_cn::core::impersonate::sina_referer()]))
    .await?;
```

- 模块:`src/core/impersonate.rs`,导出为 `crate::ImpersonateClient` / `crate::impersonate`。
- 原生库:已随仓库**内置** `native/libcurl-impersonate/`(macOS arm64/x86_64 dylib),经
  `build.rs` 把 `LC_RPATH` 烤进每个二进制,**无需 sudo、无需 `DYLD_LIBRARY_PATH`**。
- GBK 解码:新浪/百度/jisilu 返回 GBK 页面,本后端一律按 `encoding_rs::GBK` 解码(UTF-8/BOM 回退),
  避免底层库的严格 UTF-8 在中文页上 panic。
- 适用场景:被 TLS 指纹拦截的反爬源(Cloudflare/Akamai 类)。**注意**:本环境可达的源(新浪/百度/
  腾讯/东方财富/雪球)用默认 `reqwest` + 正确 `Referer` 头即可访问,不依赖模拟;
  且 `push2his.eastmoney.com` 会拒绝 Chrome h2 指纹,仍由默认 `Client` 服务。
- 推迟集不因此解锁:现有 `DEFERRED` 接口主要受 JS 执行(按 ADR-0005 逆为纯 Rust)/ 令牌 / HTML/Excel 限制(见
  [`docs/IMPERSONATE_RETRIAGE.md`](docs/IMPERSONATE_RETRIAGE.md)),而非 TLS 指纹。

## 文档

- 对标表(覆盖率 / 上游锚点):[`docs/MAPPING.md`](docs/MAPPING.md)
- 浏览器模拟重审:[`docs/IMPERSONATE_RETRIAGE.md`](docs/IMPERSONATE_RETRIAGE.md)
- 路线图:[`ROADMAP.md`](ROADMAP.md)
- 架构决策记录:[`docs/adr/`](docs/adr)
- 移植指引:[`docs/PORTING_GUIDE.md`](docs/PORTING_GUIDE.md)

## 许可证

Apache-2.0
