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
| 25 | 沪深港通 `stock::hsgt`(东财 datacenter-web / push2) | ✅ DONE | 9 | 11 ✅ |
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
| 53 | 中国宏观扩展 `economic::macro_china2`(东财 datacenter:银行/保险/农产品/指数/房地产等 25 项) | ✅ DONE | 25 | 25 ✅ |
| 54 | 期权波动率 `index::qvix`(optbbs 纯 CSV:50ETF/300ETF/500ETF/创业板/科创/100ETF/指数 日/分钟) | ✅ DONE | 18 | 2 ✅ |
| 55 | 上交所期权 `option::sse`(新浪 JSONP:列表/到期/代码/行情/Greeks/分钟/日线) | ✅ DONE | 10 | 10 ✅ |
| 56 | 三大报表 `stock::financial_three`(东财 emweb/datacenter:资产负债/利润/现金流 年/季/退市) | ✅ DONE | 8 | 8 ✅ |
| 57 | 财务(同花顺/港股/美股) `stock::fundamental::finance_more`(10jqka + 东财) | ✅ DONE | 11 | 11 ✅ |
| 58 | A股实时行情 spot/min(东财 push2 clist/trends2/kline) `stock::stock_hist_em` | ✅ DONE | 13 | 13 ✅ |
| 59 | 基金列表/净值/规模(EM:申购/指数/开放/货币/理财/分级/ETF/估值/港股) `fund::em` | ✅ DONE | 11 | 12 ✅ |
| 60 | 各国央行利率(金十 `datacenter-api` 纯 JSON) `economic::macro_bank` | ✅ DONE | 11 | 1 ✅ |
| 61 | 宏观 NBS 中国 + 欧元区(Jin10 `reports/list_v2`) `economic::macro_nbs_euro` | ✅ DONE | 14 | 2 ✅ |
| 62 | 申万研究指数(申万研究 API:hist/min/component/realtime/analysis) `index::research_sw` | ✅ DONE | 8 | 8 ✅ |
| 63 | 商品期权(东财/郑商所 JSON + GFEX POST) `option::commodity` | ✅ DONE | 4 | 4 ✅ |
| 64 | 期货持仓排名(SHFE/GFEX JSON) `futures::cot` | ✅ DONE | 2 | 2 ✅ |
| 65 | 上海黄金交易所 `spot::sge`(SGE 行情/历史/基准价,新增顶层领域) | ✅ DONE | 5 | 5 ✅ |
| 66 | 乘联会汽车 `other::car_cpca`(CPCA chartlist JSON,新增顶层领域) | ✅ DONE | 6 | 7 ✅ |
| 67 | 艺恩票房 `alt::movie_yien`(endata POST JSON) | ✅ DONE | 3 | 2 ✅ |
| 68 | 货币/外汇 `currency`(currencyscoop/外汇交易中心,新增顶层领域) | ✅ DONE | 6 | 6 ✅ |
| 69 | REITs `reits`(东财 push2,新增顶层领域) | ✅ DONE | 2 | 2 ✅ |
| 70 | 期货衍生品 `futures_derivative`(交易所/新浪 JSON,新增顶层领域) | ✅ DONE | 8 | 9 ✅ |
| 71 | 文章/学术指标 `article`(FRED CSV,新增顶层领域) | ✅ DONE | 2 | 2 ✅ |
| 72 | 高频 `hf`(GitHub 公开 CSV,新增顶层领域) | ✅ DONE | 1 | 1 ✅ |
| 73 | 财富榜 `fortune`(新财富 JSONP,新增顶层领域) | ✅ DONE | 1 | 2 ✅ |
| 74 | QDII `qdii`(集思录 JSON,新增顶层领域) | ✅ DONE | 2 | 2 ✅ |
| 75 | 空气质量 `air`(百度 AQICN,新增顶层领域) | DEFERRED | 0 | 0 |
| 76 | 奇货可查 `qhkc`(qhkch.com,新增顶层领域) | DEFERRED | 0 | 0 |
| 77 | 银行 `bank`(银保监会,新增顶层领域) | DEFERRED | 0 | 0 |
| 78 | 迁徙 `event`(百度迁徙,新增顶层领域) | DEFERRED | 0 | 0 |
| 79 | 视频 `video`(艺恩,新增顶层领域) | DEFERRED | 0 | 0 |

| 80 | 中国宏观扩展2 `economic::macro_china_more`(东财 datacenter:财金/航运/农产品/油价等) | ✅ DONE | 7 | 10 ✅ |
| 81 | 美国宏观扩展 `economic::macro_usa_more`(Jin10 `datacenter-api` 需 `x-csrf-token`) | DEFERRED | 0 | 0 |
| 82 | 宏观杂项 `economic::macro_misc`(华尔街见闻日历 / 外汇情绪) | ✅ DONE | 2 | 2 ✅ |
| 83 | 国证指数 `index::cons`(国证系列现货 / 历史) | ✅ DONE | 3 | 4 ✅ |
| 84 | 指数扩展 `index::extra`(柯桥时尚 / 郑商所 / 央行动力 / 义乌指数等) | ✅ DONE | 11 | 11 ✅ |
| 85 | 港股美股中资 `index::stock_hk_us_zh`(恒生 / 道琼斯 / 中资股) | ✅ DONE | 5 | 5 ✅ |
| 86 | A股个股信息 `stock::info`(概况 / 退市 / 限售) | ✅ DONE | 6 | 6 ✅ |
| 87 | 港股行情 `stock::hk`(东财 / 新浪 现货 / 概况 / 财务) | ✅ DONE | 10 | 10 ✅ |
| 88 | 美股行情 `stock::us`(东财 目标价 / 粉单) | ✅ DONE | 3 | 2 ✅ |
| 89 | A股资金流 `stock::fund_flow`(东财 主力 / 板块 / 行业) | ✅ DONE | 6 | 11 ✅ |
| 90 | 板块行情 `stock::board`(东财 行业 / 概念 spot / hist / kline) | ✅ DONE | 16 | 11 ✅ |
| 91 | ESG 评论热度 `stock::esg_comment_hot`(新浪 / 华证 / MSCI / 中证 / 商道融绿) | ✅ DONE | 9 | 10 ✅ |
| 92 | 已实现波动率 `cal`(Yang-Zhang RV 纯计算 + 东财分钟包装)、`stock_feature::indicators_a`(东财 datacenter/push2ex/Futu 纯 JSON 指标) | ✅ DONE | 11 | 16 ✅ |
| 93 | 既有域新增叶子模块:`futures::sina`(新浪期货 spot/minute)、`option::exchange`(CZCE 年线 / SSE 期权板与标的)、`bond::zh`(中债/NAFMII)、`economic::macro_china3`(金十 CDN 宏观 14 个)、`index::index_more`(指数历史/实时/全球)、`cal::rv_from_futures_zh_minute_sina` | ✅ DONE | 29 | 33 ✅ |
| 94 | 既有域补叶子模块(纯 HTTP 长尾):`stock::more2`(沪港通/科创板报告/回购/新股/管股/股东大会/机构调研/千股跌停/龙虎榜/股东户数等 15 函数)、`fund::more2`(ETF/LOF 分钟/排名/新发/公告/SSE 规模/拆分折算/累计分红/规模/THS ETF 等 16 函数)、`stock::fundamental::more`(新浪三大报表/关键指标/东财股本结构/基金持仓 5 函数) | ✅ DONE | 36 | 39 ✅ |
| 95 | 东方财富个股人气榜 `stock::hot_rank`(emappdata JSON-body POST:`stock_hot_rank_em`/`stock_hot_up_em` 含 push2 实时价、`stock_hot_rank_detail_em`/`stock_hot_rank_detail_realtime_em`/`stock_hot_keyword_em`/`stock_hot_rank_latest_em`/`stock_hot_rank_relate_em` 共 7 函数);同步为 `core::client` 新增 `post_json`(JSON 请求体 POST)方法 | ✅ DONE | 7 | 7 ✅ |
| 96 | 东方财富个股/大盘资金流 `stock::fund_flow` 补齐 3 个函数:`stock_individual_fund_flow`(push2his daykline `secid=sh→1./sz·bj→0.`)、`stock_market_fund_flow`(push2his `secid=1.000001` + `secid2=0.399001`,新增 `MarketFundFlowRow` 解析 `f62-f65` 沪深指数)、`stock_individual_fund_flow_rank`(push2 clist,`今日/3日/5日/10日` 通过 `rank_indicator_fields` 映射字段,新增 `IndividualRankRow` 含 10 列净流/占比) | ✅ DONE | 3 | 4 ✅ |
| 97 | 港股个股人气榜 `stock::hot_rank` 补齐 4 个函数(东财 emappdata JSON-body POST,`marketType=000003`):`stock_hk_hot_rank_em`(POST `getAllCurrHkUsList` + push2 `116.` 前缀实时价)、`stock_hk_hot_rank_detail_em`/`stock_hk_hot_rank_detail_realtime_em`/`stock_hk_hot_rank_latest_em`(POST `getHisHkUsList`/`getCurrentHkUsList`/`getCurrentHkUsLatest`) | ✅ DONE | 4 | 4 ✅ |
| 98 | 涨停板池 siblings + 两融账户 `stock_feature::board_zt` + `stock_feature::margin_research`(东财 push2ex 5 个涨停/跌停股池 + datacenter `RPTA_WEB_MARGIN_DAILYTRADE` 两融账户统计;补齐 wave-92 预置的空叶子模块) | ✅ DONE | 6 | 6 ✅ |

**累计(2026-08-16 复核 + html_gaps 核对)**:1102 个 akshare 对外公开 API 中,**797 个已实现** Rust `pub fn`(其中 791 个为功能性 DONE、6 个为返回 `Err` 的 JS 解密桩函数,归入 DEFERRED);**289 个 DEFERRED/PARTIAL**;**16 个 INTERNAL**(akshare 内部辅助,非对外数据端点);**6 个未跟踪**(异常类 `APIError`/`AkshareException` 等,对应本库 `core::error::Error`,无需移植)。`cargo build` / `cargo test`(1002 passed, 19 ignored) / `cargo clippy` 全绿。

> 复核方法:以本地 `akshare/__init__.py` 的 `__all__`(1102 个对外名)为权威口径,与 `docs/MAPPING.md` 逐行对账,并用 `grep` 校验 `src/` 中每个 `pub fn` 是否真实存在。发现 MAPPING 原 950 个 DONE 标记中有 169 个为**虚标**(无对应 `pub fn`,仅出现在 `deferred_more.rs` 等注释/清单中),已统一更正为 `DEFERRED` 并标注 `NOT IMPLEMENTED`。更正后 MAPPING 与 `src/` 实现完全一致(781 DONE 全部有实现,0 虚标)。

> 第 80-91 行新增 12 个叶子模块(在既有 `economic` / `index` / `stock` 顶层域下),共 78 个公开函数、88 个离线解析测试(由 lead 预置模块骨架后 dispatch 12 个并行 worker 落地)。其中 `economic::macro_usa_more` 整模块因 Jin10 `datacenter-api` 需 `x-csrf-token` 会话鉴权,全部 40 个美国宏观函数 DEFERRED;`stock::fund_flow` 与 `stock::board` 的解析器被多个对外函数复用,故测试数高于函数数。

> 第 92 行新增 6 个顶层领域(`stock_feature` / `energy` / `registry` / `datasets` / `cal` / `pro`)与既有 `stock_feature` 拆分(4 个叶子模块)。其中 `stock_feature::indicators_a`(9 函数 / 11 测试,东财 datacenter/push2ex/Futu 纯 JSON 指标)与 `cal`(2 函数 / 5 测试:Yang-Zhang 已实现波动率纯计算 + 东财分钟行情包装 `rv_from_stock_zh_a_hist_min_em`)为 DONE;其余 `stock_feature::indicators_b`(乐股 token / THS JS / HTML 共 21 函数)、`energy`(6 碳交易 HTML/demjson)、`registry`(3 akshare 内部 registry JSON)、`datasets`(3 akshare 内部资源加载器)、`pro`(1 Tushare token 会话)、`cal::rv_from_futures_zh_minute_sina`(依赖未移植的 `futures_zh_minute_sina`)全部 DEFERRED(原因见 `docs/MAPPING.md`);`stock_feature::margin_research` 与 `stock_feature::board_zt`(融资融券 / 研报 / 涨停板池)暂未移植,归入后续 `stock` 专注波次。`cal` 的 `volatility_yz_rv` 用样本方差(ddof=1)对齐 pandas `.var()`,日期分组按时间戳前导 `YYYY-MM-DD` 键,单根 K 线的交易日因无方差被丢弃(对齐 akshare `.dropna()`)。

> 第 93 行(非新增顶层域,为既有域补叶子模块)由 lead 预置 5 个空叶子文件 + `pub mod` 声明后 dispatch 5 个并行 worker 落地:`futures::sina`(新浪期货 `futures_zh_spot_sina` + `futures_zh_minute_sina` 纯文本 CSV 解析,`futures_main_sina`/`futures_zh_daily_sina` 已在兄弟模块故跳过,`futures_display_main_sina` 因 `demjson` DEFERRED);`option::exchange`(CZCE 年线 `|` 文本 + SSE 期权板/标的 JSON,其余 Excel/JSON-POST DEFERRED);`bond::zh`(中债/NAFMII 纯 JSON,HTML/Excel DEFERRED);`economic::macro_china3`(14 个金十 `cdn.jin10.com` 免 token CDN 宏观,直接复用 akshare 静态 JS URL;Sina `MacPage`/Mofcom POST 等 DEFERRED);`index::index_more`(东财 push2 指数历史/实时/全球 6 函数,HTML/Excel/JS 类 DEFERRED);`cal` 顺带把 `rv_from_futures_zh_minute_sina` 从 DEFERRED 升级为 DONE(依赖 `futures::sina::futures_zh_minute_sina` 现已落地)。每个 worker 落地前均 `grep` 既有实现以避免重复。

> 第 94 行(既有域补叶子模块,纯 HTTP 长尾):`stock::more2` 与 `fund::more2` 为新增叶子文件(lead 预置 `pub mod` 后 dispatch 3 个并行 worker,其中 `fund::more2` 的 worker 触发 499 取消但已落盘、经 salvage 修复 3 处编译错误:临时值借用、日期字符串拼接改用 `format!`);`stock::fundamental::more` 新增 5 个纯 JSON 函数(新浪三大报表/关键指标、东财股本结构/基金持仓)。`stock::fundamental::more` 曾误带 4 个与 `stock::restricted` 重复的 `stock_restricted_release_*` 实现(含覆盖 3 个共享 fixtures),已 `sed` 删除重复块并 `git checkout --` 还原被覆盖的 fixtures,仅保留 5 个非重复函数。测试对齐:新浪报表/关键指标按 `report_list` 的 BTreeMap 字典序断言(改用 `find()` 按 `(报告期,指标)` 定位,而非下标),`fund_cf_em`/`fund_fh_rank_em` 的 Eastmoney `var xxx=[[[row],[row]]]` 三层包裹需多解一层取行列表(公开函数与测试均已修正)。

> 第 95 行(东方财富个股人气榜,纯 HTTP 长尾):`stock::hot_rank` 落地 `stock_hot_rank_em.py`(+ `stock_hot_up_em.py`)全部 7 个函数,全部走 `emappdata.eastmoney.com/stockrank` 的 JSON 请求体 POST(固定 `appId`/`globalId`);其中 `stock_hot_rank_em`/`stock_hot_up_em` 在 POST 排名后额外用 Eastmoney `push2` `ulist.np/get`(`secids` 由 `sc` 转 `0./1.` 前缀,`fields=f14,f3,f12,f2`)补全实时价,`涨跌额` 按 `最新价*涨跌幅/100` 推算,与 akshare 一致;`stock_hot_rank_detail_em` 合并两次 POST(排名历史 + 粉丝画像,`newUidRate`/`oldUidRate` 去 `%` 转小数);`stock_hot_rank_latest_em` 解析 `data` 字典(`from_dict orient=index`)为 `(item,value)` 行。为支撑该批,`core::client` 新增 `post_json`(JSON 请求体 POST,复用既有 retry/限流/信号量),并将内部 `fetch_with_retry` 增加可选的 `body: Option<&Value>` 参数(既有 4 个调用点补 `None`);为契合 akshare 列名,上游 key 名(如 `date`/`rk`/`newUidRate`/`name`/`code`/`hot`/`relateSc`/`pct`)为按 akshare 列重命名推断的代表值,联网联调时若与真实响应字段不符需微调。

> 第 96 行(东方财富个股/大盘资金流,纯 HTTP 长尾,补齐 `stock::fund_flow` 的 `stock_fund_em.py` 个股与大盘两条线):`stock_individual_fund_flow` 复用 `emh_klines`+`parse_fflow_klines`(返回 `Vec<FundFlowHistRow>`),`market` 入参 `sh→1.<code>`、其余 `sz`/`bj→0.<code>`(对齐 akshare `get_hist_data_em` 的 `code_id` 拼接);`stock_market_fund_flow` 新增 `MarketFundFlowRow` 与 `parse_market_fund_flow`,在 15 字段 kline 末尾 `cells[11..14]` 取上证/深证 收盘价与涨跌幅(`f62-f65`),其余净流/占比列布局与 `parse_fflow_klines` 一致;`stock_individual_fund_flow_rank` 新增 `IndividualRankRow`(含 主力/超大单/大单/中单/小单 净额与占比 共 10 列)+ `parse_individual_rank`,`indicator∈{今日,3日,5日,10日}` 经 `rank_indicator_fields` 映射到 push2 的 `f62/f267/f164/f174` 系列净流字段与 `f3/f127/f109/f160` 涨跌幅字段、`fs` 复用 akshare 全市场 `m:0+t:6+f:!2,...` 串、`pz=100`;该批未改动既有共享 `FundFlowPeriod` 枚举(个股排名额外支持 3日,故在 `rank_indicator_fields` 本地处理)。fixtures 为 `stock_individual_fund_flow.json` / `stock_market_fund_flow.json`(push2his daykline)与 `stock_individual_fund_flow_rank.json`(push2 clist diff,今日字段集)。

> 第 97 行(港股个股人气榜,纯 HTTP 长尾,补齐 `stock::hot_rank`):与 A股人气榜同源 emappdata `stockrank`,但 `marketType="000003"` 且端点为 `get*HkUs*` 系列;`stock_hk_hot_rank_em` 复用既有 `post_json`/`base_payload`,`@secid` 前缀为 `116.`(`hk_to_secid` 将 `HK|00700`→`116.00700`,区别于 A股 `0./1.`),POST 排名后再次用 `push2` `ulist.np/get`(`fields=f14,f3,f12,f2`)补全实时价,`代码` 取 `sc` 按 `|` 切分第二段(`00700`),与 akshare 一致;detail/realtime/latest 三个端点分别 POST `getHisHkUsList`/`getCurrentHkUsList`/`getCurrentHkUsLatest`(`srcSecurityCode="HK|{symbol}"`),其中 latest 解析 `data` 字典为 `(item,value)` 行(同 A股 `stock_hot_rank_latest_em`)。新增 `HkHotRankRow`/`HkHotDetailRow`/`HkHotRealtimeRow`/`HkHotLatestRow` 四个结构体与 `parse_hk_hot_rank`;fixtures 为 `stock_hk_hot_rank_em.json` / `stock_hk_hot_rank_em_diff.json`(push2 diff)与 `stock_hk_hot_rank_detail_em.json` / `stock_hk_hot_rank_detail_realtime_em.json` / `stock_hk_hot_rank_latest_em.json`。注:同目录 `stock_hk_spot` 已在 `stock::hk::spot` 以同名前缀无关形式落地,本批不重复。

> 第 68-79 行新增 12 个顶层领域、22 个公开函数、24 个离线解析测试(由 lead 直接 dispatch 10 个并行 worker 落地,覆盖纯 HTTP 长尾):`currency`(`currency_*` 走 currencyscoop 纯 JSON API、`fx_c_swap_cm` 走外汇交易中心 POST;`currency_boc_sina`/`currency_boc_safe`/`currency_pair_map`/`fx_quote_baidu` 为 HTML/Excel/反爬 → DEFERRED);`reits`(东财 push2/push2his,无 DEFERRED);`futures_derivative`(大商所/广期所/上期能源/上期所 合约信息为公开 JSON、新浪主力 JSONP、东财 hog datacenter 全落地;`cffex`/`czce` 为 XML、`futures_hold_pos_sina`/`futures_spot_sys` 为 `pd.read_html`、`futures_display_main_sina` 依赖 `demjson` → DEFERRED);`article`(`fred_md`/`fred_qd` 走 FRED S3 CSV;EPU/FF/RV 为 Excel/HTML/JS → DEFERRED);`hf`(`hf_sp_500` 走 GitHub 公开 CSV,无 DEFERRED);`fortune`(仅 `xincaifu_rank` 新财富 JSONP 落地,Bloomberg/胡润/福布斯 为 HTML、`business`/`online_value_artist` 需 JS 解密 → DEFERRED);`qdii`(集思录 `qdii_a/e_index_jsl` 公开 JSON,无 DEFERRED);`air`/`qhkc`/`bank`/`event`/`video` 全 DEFERRED(XML/HTML/JS 签名/API 下线/反爬)。

> 第 58-67 行新增 77 个函数 / 65 个离线解析测试(由 lead 直接 dispatch 9 个并行 worker 落地,覆盖 research agent-11 3 批计划剩余的纯 HTTP 长尾):`stock::stock_hist_em`(沪深京 A/北交/港股主板/AB 比价/美股 实时 spot 与 分钟趋势,东财 push2,无 DEFERRED);`fund::em`(EM 申购/指数/开放/货币/理财/分级/ETF 净值与日列表,`fund_money_fund_daily_em` 与 `fund_etf_fund_daily_em` 为 gb2312 `pd.read_html` 抓取 → DEFERRED,`fund_open_fund_info_em` 已在 `fund::open_fund` 移植故跳过);`economic::macro_bank` 与 `economic::macro_nbs_euro` 走金十纯 JSON(`macro_china_nbs_nation`/`macro_china_nbs_region` 需 `curl_cffi` 会话预热多步导航、`macro_euro_lme_holding`/`macro_euro_lme_stock` 为嵌套元组字符串 `eval` → DEFERRED);`index::research_sw` 走申万研究 API(`index_realtime_sw` 的 大类风格/金创 子路径为 JSON-body POST,`Client` 无此能力 → 该 2 symbol DEFERRED,其余 GET 全落地);`option::commodity`(DCE 为 JSON-body POST、CZCE 为 `|` 分隔 `pd.read_table` HTML → 2 个 DEFERRED,SHFE/GFEX 4 个落地);`futures::cot`(CZCE/DCE/CFFEX 为 Excel/HTML/ZIP 抓取、聚合器依赖这些 → 7 个 DEFERRED,SHFE/GFEX 2 个落地);`spot::sge` 与 `other::car_cpca` 全落地(纯 JSON);`alt::movie_yien`(`decrypt` 需 `py_mini_racer` JS、`movie_boxoffice_weekly`/`movie_boxoffice_cinema_weekly` 上游权限错误 → 3 个 DEFERRED,其余已在 `alt::movie` 移植故跳过)。

> 注:第 25-30 行领域数与函数数合计为 96 + 23 = 119;`stock::holder` 与 `stock::margin` 均含 `stock_yjbb_em` 同名实现,已统一保留 `stock::margin` 版本,`stock::holder` 中移除重复实现。
> 第 31-36 行新增 33 个函数 / 33 个离线解析测试,其中 `coin` 为新增顶层领域(金属 + 内外盘期货历史 / 排名),统一复用东财 `push2his` kline 解析;`coin_foreign_hist` 与 `coin_futures_hist` 共用 `parse_kline`,kline 字段布局对齐 akshare(`change_pct`=p[8]、`change`=p[9]、`open_interest`=p[12]、`position_chg`=p[13])。
> 第 37-41 行新增 110 个函数 / 110 个离线解析测试:`macro_intl` 原由三个 worker 分别按 UK/CA/AU 与 JP/DE 与 新兴市场经济体 落地,因 akshare 本 checkout(1.18.89)缺失 `macro_india/singapore/korea/brazil/mexico/turkey/russia/france/italy/...` 等文件,三个实现相互重叠,已合并为单一 `macro_intl` 模块(英/加/澳/日/德/瑞士/香港),删除重复子集 `macro_ukca`/`macro_jpde`/`macro_eu`。`index::cx` 与既有 `stock::index::more::index_pmi_cx` 通过 `category` 参数存在表面重叠,但对外函数名不同,均保留。
> 第 42-46 行新增 19 个函数 / 19 个离线解析测试:`macro_usa` 走 Jin10 公开 `cdn.jin10.com` 明文 JSON(`datacenter-api.jin10.com` 需 `x-csrf-token`,其余宏观函数暂 DEFERRED);`cbond` 走中债登 POST(fixture 断言按 serde_json 默认 BTreeMap 的键字典序升序对齐);`option::sina` 仅落地 `option_cffex_daily`(统一 `option_cffex_*_daily_sina`),并修正 `option/mod.rs` 误重导出本 checkout 不存在的 `option_sina_spot`;`index/funddb.rs` 因上游源文件 ABSENT 未创建。
> 第 47-52 行新增 38 个函数 / 40 个离线解析测试(由 research agent-11 的调研确定剩余 ~805 未移植函数中 ~600 可行的纯 HTTP 子集,本波取东财 datacenter / 新浪 JSON 族):`stock::dzjy`(大宗交易)、`stock::financial`(三大财务报表,归一化为 `(证券,项目,报告期,值)` 行)、`stock::sy`(市盈率/市净率)、`stock::gpzy`(股权质押)、`bond::cov`(可转债现货/日线/分钟/资料,`daily` 因新浪需 JS 解密改用东财 push2his 同列结构)、`stock::fundamental::registration`(注册制各板块 + IPO 申报/审核/辅导 + 盈利预测)。其中 `stock_zcfz_*`/`stock_lrb_*`/`stock_xjll_*` 与既有 `stock::fundamental::eastmoney` 的 `*_by_report_em` 来自不同 akshare 文件,无重复;`bond_cov_comparison`/`bond_zh_cov`/`bond_zh_cov_value_analysis` 已在 `bond::eastmoney` 移植,本波跳过。
> 第 53-57 行新增 72 个函数 / 56 个离线解析测试(续 research agent-11 的 3 批计划):`economic::macro_china2` 取 `macro_china.py` 中 25 个东财 datacenter `RPT_*` 函数(Jin10 `reportType` 令牌门控与 Sina `MacPage` 共 ~25 个 DEFERRED);`index::qvix` 取 optbbs.com 纯 CSV 的 18 个 ETF/指数波动率(日/分钟);`option::sse` 取新浪 JSONP 的 10 个上交所期权函数(CFFEX 列表 HTML 抓取与已统一的 spot/daily 跳过);`stock::financial_three` 取 `stock_three_report_em.py` 的 8 个年/季/退市报表(emweb HTML helper DEFERRED);`stock::fundamental::finance_more` 取 10jqka `stock_finance_ths` 7 个 + 港股/美股东财 datacenter 各 2 个(共 11,THS 三个 HTML `.phtml` 抓取 DEFERRED)。`qvix` 仅 2 个测试(日/分钟共用解析器),其余按函数数一一对应。

### 已推迟 / 部分(DEFERRED / PARTIAL)
- 各领域中需 HTML 表解析 / JS 引擎 / 第三方鉴权的长尾端点:`stock_dividend`(cninfo `Accept-Enckey`)、`air`/`weather`/`epidemic`/`food`/`fortune`(纯页面抓取)、`futures_spot`(JS 签名)、`index_stock_info`(HTML 抓取)等,已在 `docs/MAPPING.md` 对应条目下标注跳过原因。
- `stock_dividend` 与 `stock_rank_em` 已实现联网路径但未纳入离线 fixtures 比对,待补 fixtures。
- 部分 akshare 函数在本 checkout 中已更名 / 重构(如 `fund_name`→`fund_open_fund_name_em`、`futures_zh_daily` 已有 Eastmoney 版),已就近对齐实现。

### 已推迟(DEFERRED)
- 各领域中需 HTML 表解析 / JS 引擎 / 第三方鉴权的长尾端点(已在 `docs/MAPPING.md` 对应条目下标注跳过原因),如 `air`(JS 签名)、`epidemic`、`food`、`weather`、`fortune` 等纯页面抓取类。
- `stock_dividend`(cninfo,需 `Accept-Enckey` JS 鉴权)与 `stock_rank_em`(JSON-POST)已实现联网路径但未纳入离线 fixtures 比对,待补 fixtures。
- `stock_individual_spot_xq` 及雪球(`stock_*_xq`)族:需 `xq_a_token` 登录态 cookie(第三方会话令牌),按令牌 DEFERRED 政策跳过。
- `stock_hk_hot_rank_*` 的港币/其他源与 `stock_hk_daily`/`stock_us_daily`/`stock_us_spot`/`stock_hk_index_daily_sina`:新浪源需 `py_mini_racer` 执行 JS 解密(`hk_js_decode`/`zh_js_decode`),DEFERRED。
- `stock_a_*_lg` / `stock_index_*_lg` / `stock_market_*_lg` / `stock_hk_gxl_lg` / `stock_hk_indicator_eniu` 等:`lg`(乐股)/`eniu` 第三方令牌源,DEFERRED。
- `stock_board_concept_*_ths` / `stock_board_industry_*_ths` / `stock_hk_fhpx_detail_ths` / `stock_*_ths`(同花顺):需 `hexin-v` JS 签名(`ths.js`),DEFERRED。
- `stock_fund_flow_big_deal` / `stock_fund_flow_concept` / `stock_fund_flow_individual` / `stock_fund_flow_industry`(同花顺):需 `hexin-v` JS 签名,DEFERRED。
- `stock_hk_daily` / `stock_us_daily` / `stock_us_spot`(新浪):`py_mini_racer` JS 解密,DEFERRED。

### 下一步候选
- 沪深港通 `stock_hsgt_*` 全部 9 个端点已落地于 `stock::hsgt`(见 MAPPING「Stock 沪深港通 (hsgt)」):`stock_hsgt_fund_flow_summary_em`、`stock_hsgt_hist_em`、`stock_hsgt_hold_stock_em`、`stock_hsgt_stock_statistics_em`、`stock_hsgt_institution_statistics_em`、`stock_hsgt_board_rank_em`、`stock_hsgt_individual_em`、`stock_hsgt_individual_detail_em`、`stock_hsgt_fund_min_em`。其中 `stock_hsgt_hold_stock_em` / `stock_hsgt_stock_statistics_em`(`RPT_MUTUAL_STOCK_NORTHSTA`,本机构出口 IP 限流返回 9701)与 `stock_hsgt_fund_min_em`(push2 地域封锁 HTTP 302)三端 fixtures 为**合成数据**,已在 fixture 文件 `_note` 与模块头注释标注,需在首次联网运行时以真实响应校准字段键(解析器对缺失键做了容错,真实数据不会硬失败)。
- 继续补齐长尾包:`news` / `nlp` / `event` / `lpr` / `stock_fundamental`(财务)等。
- 为 `rate_interbank` 等增加多源 fallback(目前为单源)。
- 实现 `scripts/sync-akshare`(ADR-0012 的对标更新机制)。

---

## 完整度审计(2026-08-16)

目标:按设计(ADR-0008 范围对齐、ADR-0005 令牌/JS 策略)完全重构 akshare 的所有对外 API。口径以本地 `akshare` checkout 的 `__all__`(1102 个公开函数)为权威。

### 口径与现状

| 类别 | 数量 | 说明 |
|---|---|---|
| akshare 对外公开 API 总数 | 1102 | `akshare/__init__.py` `__all__` |
| 已实现(有 `pub fn`) | 787 | 781 功能性 DONE + 6 个 JS 解密桩(返回 `Err`,记 DEFERRED) |
| DEFERRED / PARTIAL | 299 | 见下「推迟原因分布」 |
| INTERNAL | 16 | akshare 内部辅助函数,非对外数据端点,无需移植 |
| 未跟踪(异常类) | 6 | `APIError`/`AkshareException`/`DataParsingError`/`InvalidParameterError`/`NetworkError`/`RateLimitError`,对应本库 `core::error::Error` |

功能性 DONE 占公开 API 的 **71.8%**(791 / 1102)。

### 推迟原因分布(299 个 DEFERRED)

| 原因 | 数量 | 是否设计内推迟(ADR) |
|---|---|---|
| JS 引擎 / 签名解密(`py_mini_racer` / `hexin-v` / `jm.js` / CYQ) | ~106 | 是(ADR-0005):需 JS 引擎或逐端点纯 Rust 逆向 |
| 第三方令牌 / 会话(`xq_a_token` / Jin10 `x-csrf-token` / `lg`·`eniu` / 艺恩权限) | ~12 | 是(ADR-0005):令牌门控 |
| HTML 表抓取(`pd.read_html` / BeautifulSoup,无 JSON 端点) | ~9 | 部分可行:可用 `scraper` 补(见 `air_html_gaps` 等先例) |
| 反爬令牌(`acs-token` / `_pcc` / huiyan) | 少量 | 是:anti-bot |
| NBS 目录动态解析 / ZIP·Excel·`demjson` | 少量 | 部分可行 |

> 注:上述 ~106 个 JS 引擎类推迟是「完全重构」的主要剩余障碍。按当前设计(ADR-0005)**保持推迟**,除非接受嵌入 JS 引擎(`rquickjs` / `boa`)。其中少数(如新浪日线 `hk_js_decode` / `zh_js_decode`、CYQ `CYQCalculator`)若逆为纯 Rust,可单独解锁,无需整引擎——属增量工作。

### 领域缺口(DEFERRED 按前缀)

`stock`(68)、`fund`(14)、`macro`(11)、`bond`(8)、`futures`(2)、`air`(2)、`migration`(2)、`movie`(2)、`video`(2)、`energy`(1)、`index`(1)、`business`(1)、`online`(1)、`news`(1)、`option`(1)、`pro`(1)、`qhkc`(1)、`tool`(1)。(含本次复核更正的 169 个虚标 DONE。`spot` 全部 6 个 soozhu 端点已在 `spot_html_gaps` 落地,本轮核对由 DEFERRED 更正为 DONE;`futures` 的 4 个 HTML 抓取端点(futures_dce_position_rank_other / pandas_read_html_link / futures_hold_pos_sina / futures_spot_sys)同批更正为 DONE,仅余令牌/JS 门控( zh_subscribe_exchange_symbol / futures_contract_info_cffex / futures_contract_info_czce )。)

### 下一步(按设计收敛)

1. **HTML 抓取类(~9 + 部分虚标)**:用 `scraper` 逐端点补实现,复用既有 `*_html_gaps` 模式(需真实 fixture,联网环境补齐)。
2. **JS 签名类**:要么接受 `rquickjs` 引擎统一解锁,要么挑高频端点(新浪日线、CYQ)做纯 Rust 逆向。
3. **令牌类**:仅当用户提供令牌/会话策略时解锁。
4. 每补一批即回填 fixture + 解析测试,并更新 `docs/MAPPING.md`(ADR-0012)。
