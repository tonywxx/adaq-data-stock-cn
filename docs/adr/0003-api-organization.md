# 地道 Rust 模块组织 + 过渡期 akshare→本库 映射表

按领域组织 API(`stock::spot::eastmoney::...`),参数用 newtype / builder 强制类型,不 1:1 照抄 akshare 扁平命名。保留一份 akshare 函数名 → 本库对应的映射表用于覆盖率审计;完全重构后可删可留。

**Why:** 千级端点规模下,地道组织远易于维护;映射表保证"对齐 akshare"可审计、可量化进度。
**Trade-off:** 失去按 akshare 原名直接查找的便利性,用映射表弥补。映射表是临时产物,不进长期架构。
