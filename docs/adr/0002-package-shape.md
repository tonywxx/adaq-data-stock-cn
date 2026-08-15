# 以单 crate 库形态开发并发布到 crates.io

以单个 Rust library crate(`adaq-data-stock-cn`)开发,按模块清晰组织(`src/stock/`、`src/futures/` 等),目标是发布到 crates.io 供 AdaQ 平台安装调用。暂不做 workspace 拆分,暂不做 pyo3/FFI(目标里没有 Python)。

**Why:** 用户明确不用 Python,且希望平台能直接 `cargo add` 安装;早期拆 workspace 只增加摩擦。
**Trade-off:** 等端点超过数百或需要独立版本号时,再迁移到 workspace。当前为可反向的提前投入,故仅记录方向。
