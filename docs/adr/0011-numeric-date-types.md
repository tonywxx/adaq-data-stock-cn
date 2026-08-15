# 数值与时间类型

日期用 `chrono`(`NaiveDate` 用于交易日,`DateTime<Local>` 用于带时点的行情);价格/数值默认 `f64`(A股精度足够)。个别需精确的合计类字段留 `rust_decimal` 余地,但不全局强求。

**Why:** A股价格用 f64 足够;decimal 仅在明确需要处引入,避免全局解析/序列化负担。很多上游本身返回浮点。
**Trade-off:** 相比全用 `Decimal` 牺牲极端精度;相比全用 `String` 保留数值可用性。
