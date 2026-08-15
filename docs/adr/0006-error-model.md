# crate 级 thiserror 错误枚举

对外库使用 `thiserror` 定义的 `Error` 枚举,变体含 `Http` / `Parse` / `RateLimited` / `UpstreamChanged` / `NotFound` / `InvalidParam`,并携带上下文(源、端点、URL)。

**Why:** 上游常变、接口常挂,类型化错误让调用方精确分支;附加上下文便于定位断点。
**Trade-off:** 相比 `anyhow` 写起来更啰嗦,但库不应向调用方暴露动态错误。
