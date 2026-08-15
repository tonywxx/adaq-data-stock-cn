# 异步优先(tokio + reqwest)

数据获取为 I/O 密集,平台常需并发拉取多标的。crate 以 async 为主(`async fn` + `reqwest` + `tokio`),阻塞封装按需后补。

**Why:** 量化并发拉数收益大;异步形态影响整个 API 设计,属难回头决定。
**Trade-off:** 调用方需处于 async 运行时;个别同步需求后续补阻塞封装,不在首版。
