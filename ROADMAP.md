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
| 12 | 新浪日线 `stock::daily_sina`(纯 Rust MD5 签名逆向) | ✅ DONE | 1 | 1 ✅ |
| 13 | 板块 `board`(东财行业/概念) | ✅ DONE | 4 | 4 ✅ |
| 14 | 资讯 `news`(百度财经日历 / 东方财富个股新闻 / OwnThink NLP) | ✅ DONE | 7 | 7 ✅ |
| 15 | LPR `lpr`(央行授权行) | ✅ DONE | 1 | 1 ✅ |
| 16 | 交易日历 `calendar`(新浪 `tool_trade_date` 纯 Rust 位打包解码) | ✅ DONE | 1 | 1 ✅ |
| 17 | 杂项 `stock::extra`(股东户数 / 分红 / 人气榜) | ✅ DONE | 3 | 3 ✅ |
| 18 | 另类数据 `alt`(油价 / 票房 / 外汇牌价) | ✅ DONE | 9 | 9 ✅ |
| 19 | A股杂项 `stock::misc`(分钟线 / 新股 / 停牌 / 概况) | ✅ DONE | 5 | 5 ✅ |
| 20 | 指数扩展 `stock::index::extra`(现货 / 日线 / 成分 / 腾讯日线) | ✅ DONE | 4 | 4 ✅ |
| 21 | 期货扩展 `futures::extra`(新浪日线 / 外盘 / 库存) | ✅ DONE | 4 | 4 ✅ |
| 22 | 基金扩展 `fund::extra`(名单 / 估值 / 历史 / 货币 / ETF分类) | ✅ DONE | 5 | 5 ✅ |
| 23 | 外汇扩展 `forex::extra`(中行牌价 / 中行历史 / 人民币掉期) | ✅ DONE | 3 | 3 ✅ |
| 24 | 数字货币扩展 `crypto::extra`(Binance/OKX 历史 / 现货 / 信息) | ✅ DONE | 4 | 4 ✅ |
| 25 | A股资金流 / 沪深港通 `stock::flow` | ✅ DONE | 5 | 5 ✅ |
| 26 | A股个股信息 / 主营构成 / 板块 `stock::holder` | ✅ DONE | 3 | 3 ✅ |
| 27 | 融资融券 / 业绩报表 `stock::margin` | ✅ DONE | 3 | 3 ✅ |
| 28 | 债券扩展 `bond::extra`(可转债 / 回购) | ✅ DONE | 4 | 4 ✅ |
| 29 | 宏观扩展 `economic::extra`(房价 / LPR / 景气 / 税收 / 物价 / FDI) | ✅ DONE | 6 | 6 ✅ |
| 30 | 期货主力 / 合约列表 `futures::main`(新浪) | ✅ DONE | 2 | 2 ✅ |
| 31 | 期权扩展 `option::extra`(东财 / 新浪 / 上交所) | ✅ DONE | 6 | 6 ✅ |
| 32 | 基金扩展2 `fund::more`(规模 / 分红 / 经理 / 持仓结构) | ✅ DONE | 6 | 6 ✅ |
| 33 | A股更多 `stock::more`(ST / 高低价 / 破净 / 账户 / 涨停池) | ✅ DONE | 5 | 5 ✅ |
| 34 | 指数更多 `stock::index::more`(全球现货 / 全球历史 / 中证 / 国证 PMI) | ✅ DONE | 5 | 5 ✅ |
| 35 | 宏观第二批 `economic::macro2`(PMI / 固投 / 工业 / 消费 / 美国 CPI / 美国 PHS) | ✅ DONE | 6 | 6 ✅ |
| 36 | 金属 / 外盘 `coin`(LME 实时 / SHFE 排名 / 外盘历史 / 国内期货历史 / 合约映射) | ✅ DONE | 5 | 5 ✅ |
| 37 | 国际宏观 `economic::macro_intl`(英 / 加 / 澳 / 日 / 德 / 瑞士 / 香港,东财 datacenter) | ✅ DONE | 60 | 60 ✅ |
| 38 | 指数 Caixin `index::cx`(财新趋势指数) | ✅ DONE | 16 | 16 ✅ |
| 39 | A股股东分析 `stock::gdfx`(东财限售 / 持股 / 十大股东) | ✅ DONE | 12 | 12 ✅ |
| 40 | A股龙虎榜 `stock::lhb`(东财每日 / 机构 / 营业部) | ✅ DONE | 10 | 10 ✅ |
| 41 | 基金业协会 `fund::amac`(AMAC 会员 / 产品 / 管理人) | ✅ DONE | 12 | 12 ✅ |
| 42 | 美国宏观 `economic::macro_usa`(Jin10 公开 JSON:钻机 / 原油 / CFTC / CME) | ✅ DONE | 7 | 7 ✅ |
| 43 | 中债指数 `bond::cbond`(中债登 composite / treasury / general) | ✅ DONE | 4 | 4 ✅ |
| 44 | 限售股解禁 `stock::restricted`(东财数据中心) | ✅ DONE | 4 | 4 ✅ |
| 45 | 估值指标 `stock::indicator`(百度股市通 / 东财估值 / 估值分析) | ✅ DONE | 3 | 3 ✅ |
| 46 | 期权新浪 `option::sina`(CFFEX 指数期权日线 JSONP) | ✅ DONE | 1 | 1 ✅ |
| 47 | 大宗交易 `stock::dzjy`(东财 datacenter:市场统计/每日明细/每日统计/行业/营业部) | ✅ DONE | 6 | 6 ✅ |
| 48 | 财务报表 `stock::financial`(东财 datacenter:资产负债/利润/现金流,归一化行) | ✅ DONE | 4 | 4 ✅ |
| 49 | 市盈率 `stock::sy`(东财 datacenter:概况/远期/市净率/行业) | ✅ DONE | 5 | 5 ✅ |
| 50 | 股权质押 `stock::gpzy`(东财 datacenter:概况/质押比例/分布统计/行业) | ✅ DONE | 7 | 7 ✅ |
| 51 | 可转债 `bond::cov`(新浪/东财:现货/日线/分钟/前复权分钟/资料) | ✅ DONE | 5 | 6 ✅ |
| 52 | 发行与申购 `stock::fundamental::registration`(东财 datacenter:注册制/IPO/盈利预测) | ✅ DONE | 11 | 12 ✅ |

**累计**:52 个领域、327 个公开函数、345 个离线解析测试,`cargo build` / `cargo test` / `cargo clippy` 全绿。

> 注:第 25-30 行领域数与函数数合计为 96 + 23 = 119;`stock::holder` 与 `stock::margin` 均含 `stock_yjbb_em` 同名实现,已统一保留 `stock::margin` 版本,`stock::holder` 中移除重复实现。
> 第 31-36 行新增 33 个函数 / 33 个离线解析测试,其中 `coin` 为新增顶层领域(金属 + 内外盘期货历史 / 排名),统一复用东财 `push2his` kline 解析;`coin_foreign_hist` 与 `coin_futures_hist` 共用 `parse_kline`,kline 字段布局对齐 akshare(`change_pct`=p[8]、`change`=p[9]、`open_interest`=p[12]、`position_chg`=p[13])。
> 第 37-41 行新增 110 个函数 / 110 个离线解析测试:`macro_intl` 原由三个 worker 分别按 UK/CA/AU 与 JP/DE 与 新兴市场经济体 落地,因 akshare 本 checkout(1.18.89)缺失 `macro_india/singapore/korea/brazil/mexico/turkey/russia/france/italy/...` 等文件,三个实现相互重叠,已合并为单一 `macro_intl` 模块(英/加/澳/日/德/瑞士/香港),删除重复子集 `macro_ukca`/`macro_jpde`/`macro_eu`。`index::cx` 与既有 `stock::index::more::index_pmi_cx` 通过 `category` 参数存在表面重叠,但对外函数名不同,均保留。
> 第 42-46 行新增 19 个函数 / 19 个离线解析测试:`macro_usa` 走 Jin10 公开 `cdn.jin10.com` 明文 JSON(`datacenter-api.jin10.com` 需 `x-csrf-token`,其余宏观函数暂 DEFERRED);`cbond` 走中债登 POST(fixture 断言按 serde_json 默认 BTreeMap 的键字典序升序对齐);`option::sina` 仅落地 `option_cffex_daily`(统一 `option_cffex_*_daily_sina`),并修正 `option/mod.rs` 误重导出本 checkout 不存在的 `option_sina_spot`;`index/funddb.rs` 因上游源文件 ABSENT 未创建。
> 第 47-52 行新增 38 个函数 / 40 个离线解析测试(由 research agent-11 的调研确定剩余 ~805 未移植函数中 ~600 可行的纯 HTTP 子集,本波取东财 datacenter / 新浪 JSON 族):`stock::dzjy`(大宗交易)、`stock::financial`(三大财务报表,归一化为 `(证券,项目,报告期,值)` 行)、`stock::sy`(市盈率/市净率)、`stock::gpzy`(股权质押)、`bond::cov`(可转债现货/日线/分钟/资料,`daily` 因新浪需 JS 解密改用东财 push2his 同列结构)、`stock::fundamental::registration`(注册制各板块 + IPO 申报/审核/辅导 + 盈利预测)。其中 `stock_zcfz_*`/`stock_lrb_*`/`stock_xjll_*` 与既有 `stock::fundamental::eastmoney` 的 `*_by_report_em` 来自不同 akshare 文件,无重复;`bond_cov_comparison`/`bond_zh_cov`/`bond_zh_cov_value_analysis` 已在 `bond::eastmoney` 移植,本波跳过。

### 已推迟 / 部分(DEFERRED / PARTIAL)
- 各领域中需 HTML 表解析 / JS 引擎 / 第三方鉴权的长尾端点:`stock_dividend`(cninfo `Accept-Enckey`)、`air`/`weather`/`epidemic`/`food`/`fortune`(纯页面抓取)、`futures_spot`(JS 签名)、`index_stock_info`(HTML 抓取)等,已在 `docs/MAPPING.md` 对应条目下标注跳过原因。
- `stock_dividend` 与 `stock_rank_em` 已实现联网路径但未纳入离线 fixtures 比对,待补 fixtures。
- 部分 akshare 函数在本 checkout 中已更名 / 重构(如 `fund_name`→`fund_open_fund_name_em`、`futures_zh_daily` 已有 Eastmoney 版),已就近对齐实现。

### 已推迟(DEFERRED)
- 各领域中需 HTML 表解析 / JS 引擎 / 第三方鉴权的长尾端点(已在 `docs/MAPPING.md` 对应条目下标注跳过原因),如 `air`(JS 签名)、`epidemic`、`food`、`weather`、`fortune` 等纯页面抓取类。
- `stock_dividend`(cninfo,需 `Accept-Enckey` JS 鉴权)与 `stock_rank_em`(JSON-POST)已实现联网路径但未纳入离线 fixtures 比对,待补 fixtures。

### 下一步候选
- 继续补齐长尾包:`news` / `nlp` / `event` / `lpr` / `stock_fundamental`(财务)等。
- 为 `rate_interbank` 等增加多源 fallback(目前为单源)。
- 实现 `scripts/sync-akshare`(ADR-0012 的对标更新机制)。
