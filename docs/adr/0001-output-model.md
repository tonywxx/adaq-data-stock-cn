# 以类型化结构体为规范返回类型,配 JSON/Parquet/CSV 转换层

每个数据端点返回具体的 Rust 结构体(或 `Vec<struct>`)作为规范类型;另提供一层转换,把结构体序列化为 JSON、Parquet(本地存储)和 CSV(回测)。不采用统一的 Polars DataFrame 作为返回类型。

**Why:** 平台明确需要 JSON/Parquet/CSV 落地用于存储与回测;成百上千端点下,编译期类型安全远比动态 DataFrame 可维护,也便于"按需扩展"。
**Trade-off:** 相比直接返回 Polars DataFrame(最接近 akshare 手感),我们放弃了统一表格类型的便利,换取字段级类型安全与低风险重构。转换层按需补齐表格能力。
