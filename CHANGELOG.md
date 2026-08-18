# Changelog / 更新日志

All notable changes to this project are documented here. / 本项目所有值得注意的变更均记录于此。

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this
project adheres to [Semantic Versioning](https://semver.org/). / 格式遵循
[Keep a Changelog](https://keepachangelog.com/),版本号遵循
[语义化版本](https://semver.org/lang/zh-CN/)。

语言 / Language: each entry is given in **English** then **中文**.

## [Unreleased] / [未发布]

## [0.1.3] - 2026-08-17

- **chore**: Bump version to 0.1.3 and adjust the `repository` URL in `Cargo.toml`. / **杂务**:版本升至 0.1.3,并修正 `Cargo.toml` 中的 `repository` 仓库地址。

## [0.1.2] - 2026-08-17

- **refactor**: Extract HTML table parsing into shared utility functions across domains, reducing duplication and centralizing parsing logic. / **重构**:将 HTML 表格解析抽取为跨域共享的工具函数,减少重复并集中解析逻辑。

## [0.1.1] - 2026-08-17

- **fix**: Update CI workflows and documentation for the `libcurl-impersonate` integration. / **修复**:针对 `libcurl-impersonate` 集成更新 CI 工作流与文档。
- **fix**: Bump `actions/checkout` to v5 in the release workflow. / **修复**:发布工作流中将 `actions/checkout` 升级至 v5。
- **fix**: Correct the asset path used for packaging in the GitHub Actions release workflow. / **修复**:修正 GitHub Actions 发布工作流中用于打包的资源路径。

## [0.1.0] - 2026-08-16

First tagged release — the initial full Rust reimplementation of akshare. / 首个带标签的发布——akshare 的首次完整 Rust 重写。

### Added / 新增

- Initial pure-Rust reimplementation of akshare's core quote and fundamental domains. / 以纯 Rust 重实现 akshare 核心行情与基本面域。
- Ported ~944 of 1172 akshare top-level functions across ~40 domain modules (~85.8% public-endpoint coverage), reached via multi-batch porting waves (batch-4 through batch-16, long-tail wave, and gap-port wave). / 通过多轮批次移植(batch-4 至 batch-16、长尾批次、缺口批次),在约 40 个域模块中移植了 1172 个 akshare 顶层函数中的约 944 个(公开端点覆盖率约 85.8%)。
- `Client` with built-in retry/backoff, per-source rate limiting, concurrency cap, and optional on-disk cache. / 内置重试/退避、按源限流、并发上限与可选磁盘缓存的 `Client`。
- Typed struct output emitted as JSON / CSV (and Parquet behind the `parquet` feature). / 类型化结构体输出,可序列化为 JSON / CSV(Parquet 通过 `parquet` 特性提供)。
- Multi-source fallback chains (e.g. A-share daily history prefers Eastmoney, falls back to Tencent). / 多源降级链(如 A 股日线历史优先东方财富,失败回退腾讯)。
- Browser-impersonation HTTP backend (`ImpersonateClient` / `impersonate`) built on `curl-impersonate`, the Rust analog of Python `primp` (`curl_cffi`), with always-on GBK decoding for Sina/Baidu/jisilu pages. / 基于 `curl-impersonate` 的浏览器指纹模拟 HTTP 后端(`ImpersonateClient` / `impersonate`),即 Python `primp`(`curl_cffi`)的 Rust 等价实现,对新浪/百度/jisilu 的 GBK 页面始终解码。
- Vendored `libcurl-impersonate` dylib for macOS local builds (baked `LC_RPATH`, no sudo / `DYLD_LIBRARY_PATH` needed). / 为 macOS 本地构建内置 `libcurl-impersonate` dylib(烤入 `LC_RPATH`,无需 sudo 或 `DYLD_LIBRARY_PATH`)。
- JavaScript decoding functions for `hk_js` and `zh_js`. / 针对 `hk_js` 与 `zh_js` 的 JavaScript 解码函数。
- Runnable end-to-end examples under `examples/` (spot, history, convertible bonds, index, futures, Parquet export, impersonate smoke test). / `examples/` 下可运行的端到端示例(快照、历史、可转债、指数、期货、Parquet 导出、指纹模拟冒烟测试)。
- CI and tag-triggered release/publish GitHub Actions workflows. / CI 与标签触发的发布/发布至 crates.io 的 GitHub Actions 工作流。
- Coverage tracker and upstream-sync anchor in `docs/MAPPING.md` (944 `DONE` / 156 `DEFERRED` by design / 72 `INTERNAL` / 0 `UNKNOWN`). / `docs/MAPPING.md` 中的覆盖率追踪器与上游同步锚点(944 个 `DONE` / 156 个按设计 `DEFERRED` / 72 个 `INTERNAL` / 0 个 `UNKNOWN`)。
- Parity-tracker reconciliation: repaired broken `html_gaps` wave and corrected 3 mislabeled `INTERNAL` modules (`utils/demjson`, `futures/symbol_var`). / 对标表核对:修复损坏的 `html_gaps` 批次,并将 3 个被误标的 `INTERNAL` 模块(`utils/demjson`、`futures/symbol_var`)修正。

### Changed / 变更

- Reversed JS-execution endpoints to pure Rust per ADR-0005 (no JS engine embedded); deferred the remaining set gated by signing / tokens / HTML / Excel. / 依据 ADR-0005 将 JS 执行端点逆为纯 Rust(不内嵌 JS 引擎);其余受签名 / 令牌 / HTML / Excel 限制的接口按设计推迟。

[Unreleased]: https://github.com/tonywxx/adaq-data-stock-cn/compare/v0.1.3...HEAD
[0.1.3]: https://github.com/tonywxx/adaq-data-stock-cn/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/tonywxx/adaq-data-stock-cn/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/tonywxx/adaq-data-stock-cn/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/tonywxx/adaq-data-stock-cn/releases/tag/v0.1.0
