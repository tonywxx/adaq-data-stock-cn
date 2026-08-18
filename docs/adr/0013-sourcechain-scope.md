# SourceChain 采用范围（深化评审 C4）

记录架构深化评审 C4 的结论：`core::source::SourceChain`（即 ADR-0010 的多源 fallback 实现）保持当前范围，不向其余约 790 个单源端点推广。

**Why:** `SourceChain` 本身已是一个“深”抽象——fallback 循环（首成功即返回、失败透传末错误）集中在一处。若删除它，原本集中的 `if let Ok { return }` 链会回到 `stock::spot` / `stock::index` / `stock::hist` / `stock::intraday` 四个模块，复杂度是被“搬移”而非“集中”，违反删除测试。它当前仅被这 4 个真正具备多源的端点使用；其余端点天然单源，套用 `SourceChain` 纯属改动噪音，对可用性零收益。唯一的人体工学摩擦是调用处需 `Box::pin(...)` 包裹（类型抹除 future 的常规代价），改进收益极低。

**Trade-off:** 不强制统一全部端点的取数入口——未来新增单源端点也无需接入 `SourceChain`，其价值仅在“真正多源”处体现。若后续需要更强的 fallback 语义（超时预算、熔断、按源加权），应回过头增强 `SourceChain` 本身，而不是扩大采用面。
