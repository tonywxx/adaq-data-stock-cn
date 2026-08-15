# AdaQ 数据层(adaq-data-stock-cn)

本 crate 是 akshare 的 Rust 重写,作为用户量化平台 AdaQ 的市场数据层。

## Language

**AdaQ**:
用户的量化交易/分析平台;本 crate 为其提供市场数据。
_Avoid_: 平台、系统、app

**端点 (Endpoint)**:
一个具体的数据获取函数,对应 akshare 的一个公开接口(如 `stock_zh_a_spot_em`)。
_Avoid_: 接口、函数、API、方法

**源 / 上游 (Source)**:
akshare 实际请求的外站,如东方财富、新浪、同花顺、各交易所、中国货币网。
_Avoid_: 网站、host、站点

**签名参数 (Signed Param)**:
部分源(如东方财富)要求用 JS 计算的请求参数(`ut` / `hexin-v`),用于鉴权 / 防爬。
_Avoid_: token、密钥、签名

**源链 (Source Chain)**:
一个逻辑端点背后按优先级组成的多个源实现序列(如 东财 → 新浪 → 腾讯);主源失败时自动降级到下一源。
_Avoid_: fallback 链、备选源

**对标表 (Benchmark Map)**:
记录本库端点 ↔ akshare 函数名 ↔ akshare 源文件/行 的对照表(`docs/MAPPING.md`),既是覆盖率追踪器也是上游同步锚点。
_Avoid_: 映射表、对照表
