# 东财 klines 解析去重（深化评审 C5）

记录架构深化评审 C5 的结论：不按报告原方案新建 `core::eastmoney::parse_klines(text, &FieldMap)` 泛型解析器来做去重，也不强行统一各端点的 `data.klines` 提取。

**Why:** 最初报告认为存在 3 份重复的东财 kline 解析器，但实测全仓约有 20 处 `data.klines` 解析，且每一处的列映射都不同（`HistRow` / `FuturesDailyRow` / `FundFlowHistRow` / `BondZhHsCovDaily` / `ReitsHistRow` / `ForexHistRow` ……，列数、语义、字段顺序各异）。报告建议的 `FieldMap` 泛型要覆盖约 20 种不兼容 schema，属于过度抽象，可读性与可维护性反而劣于当前的显式映射。唯一真正共享且稳定的只有“取出 `data.klines` 数组并 split(',')”这 3 行提取逻辑，在全仓以 `klines_array` / `em_klines_array` / `push2_klines` 等名字重复约 8 次；但这些副本在错误 `origin` 文案、取值写法上略有差异，统一会改变各端点 fixture 测试断言的错误 `origin`，收益极低而回归风险不低。

**Trade-off:** 保留各端点显式的 `parse_*` kline 解析（列映射本就源相关，应留在端点模块）。若日后确实需要共享，最安全的最小动作是新建 `core::eastmoney::extract_klines(resp) -> Result<&Vec<Value>>` 仅统一“取 `data.klines` 数组”这一步，并逐一核对 8 处调用方的错误 `origin` 断言；该动作收益有限，已判定为不值得，留待确有需求时再做。
