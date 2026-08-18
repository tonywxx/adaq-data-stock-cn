# 不重命名 core::convert 模块（深化评审 C7）

记录架构深化评审 C7 的结论：不把 `src/core/convert.rs`（序列化辅助，`to_json` / `to_csv` 等）重命名为 `io` / `serialize`。

**Why:** 报告把 C7 标为“推测性”，建议重命名以提升可读性。但实测：全仓 0 处使用 `std::convert`，且 `crate::core::convert` 仅以 `use crate::core::convert;`（非 glob）方式被 6 个模块引用，因此**不存在**与标准库 `std::convert`（`From` / `Into`）的命名遮蔽隐患——重命名的唯一动机是“名字更好看”。该模块已通过 `src/lib.rs:48` 的 `pub use core::convert;` 作为公共 API 对外暴露，重命名会破坏下游 API 兼容性，并需改动约 7–14 处引用，属于纯噪声改动、无功能收益。

**Trade-off:** 保留 `convert` 命名。若未来确实要改名，最小且安全的做法是在新模块中 `pub use` 旧路径做兼容别名，而非硬改名；但在缺少真实遮蔽/混淆证据前不值得做。
