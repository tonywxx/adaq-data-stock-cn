# 路线图

## 第一阶段(Milestone 1):东方财富 / 新浪 / 腾讯 A股行情 + 指数

**目标**:跑通全链路,验证四大机制——Client 与韧性、多源 fallback、结构体 → JSON/Parquet/CSV 转换、fixtures 测试 + 对标表。

### 范围(10 个端点,无需 JS 签名)

| 类别 | akshare 函数 | 源 | 返回形态 |
|---|---|---|---|
| A股实时快照 | `stock_zh_a_spot_em` | 东财 | 快照表 |
| A股实时快照 | `stock_zh_a_spot` | 新浪 | 快照表 |
| A股实时快照 | `stock_zh_a_spot_tx` | 腾讯 | 快照表 |
| A股历史日线 | `stock_zh_a_hist` | 东财 | 日线 |
| A股历史日线 | `stock_zh_a_hist_tx` | 腾讯 | 日线 |
| A股分时 | `stock_intraday_em` | 东财 | 分时序列 |
| A股分时 | `stock_intraday_sina` | 新浪 | 分时序列 |
| 指数实时快照 | `stock_zh_index_spot_em` | 东财 | 快照表 |
| 指数实时快照 | `stock_zh_index_spot_sina` | 新浪 | 快照表 |
| 指数历史 | `index_zh_a_hist` | 东财 | 日线 |

### 推迟
- `stock_zh_a_daily`(新浪日线):需 MiniRacer 跑 JS 签名,留到 ADR-0005 签名逆向阶段,或接受内嵌 JS 引擎(`rquickjs`)时再做。

### 里程碑"完成"定义
- 10 个端点均返回类型化结构体,经转换层可落 JSON / Parquet / CSV。
- 每个端点有 fixtures(真实响应样例)+ 解析测试;联网集成测试默认关闭(ADR-0007)。
- 多源 fallback 在至少一处主源失败时可降级到下一源(ADR-0010)。
- `docs/MAPPING.md` 对标表覆盖本里程碑全部端点,含 akshare 源文件/行锚点(ADR-0012)。
- `cargo build` / `cargo test` / `cargo clippy` 通过;可被 `cargo add` 安装。

## 后续阶段(方向,非承诺)
- 扩展其余领域:`stock_fundamental`、`futures`、`option`、`fund`、`bond`、宏观 / 经济 等,逐源补齐。
- 签名逆向:新浪日线等需 JS 的端点,逆为纯 Rust(ADR-0005)。
- 横向:更多源、更多标的(港股 / 美股 / ETF / 期货)。

---

## 进度(自动维护)

| 阶段 | 领域 | 状态 | 端点 | 测试 |
|---|---|---|---|---|
| M1 | A股行情 / 指数(东财 / 新浪 / 腾讯) | ✅ DONE | 10 | 11 ✅ |
| 2 | 外汇 `forex`(东财) | ✅ DONE | 2 | 2 ✅ |
| 3 | 利率 `rate`(东财 + 外汇交易中心) | ✅ DONE | 3 | 5 ✅ |
| 4 | 债券 `bond`(东财 + 外汇交易中心) | ✅ DONE | 4 | 4 ✅ |
| 5 | 数字货币 `crypto`(金十) | ✅ DONE | 3 | 3 ✅ |
| 6 | 宏观 `economic`(东财) | ✅ DONE | 4 | 4 ✅ |
| 7 | 期货 `futures`(东财) | ✅ DONE | 3 | 3 ✅ |
| 8 | 期权 `option`(东财 + 新浪) | ✅ DONE | 4 | 4 ✅ |
| 9 | 基金 `fund`(东财) | ✅ DONE | 4 | 4 ✅ |
| 10 | A股基本面 `stock::fundamental`(东财) | ✅ DONE | 4 | 4 ✅ |
| 11 | 跨市场 `stock::cross` 港股/美股(东财) | ✅ DONE | 4 | 4 ✅ |

**累计**:11 个领域、45 个公开函数、50 个离线解析测试,`cargo build` / `cargo test` / `cargo clippy` 全绿。

### 已推迟(DEFERRED)
- `stock_zh_a_daily`(新浪日线):需 JS 签名,留待纯 Rust 签名逆向(ADR-0005)。
- 各领域中需 HTML 表解析 / JS 引擎 / 第三方鉴权的长尾端点(已在 `docs/MAPPING.md` 对应条目下标注跳过原因)。

### 下一步候选
- 继续补齐长尾包:`news` / `nlp` / `event` / `lpr` / `stock_fundamental`(财务)等。
- 为 `rate_interbank` 等增加多源 fallback(目前为单源)。
- 实现 `scripts/sync-akshare`(ADR-0012 的对标更新机制)。
