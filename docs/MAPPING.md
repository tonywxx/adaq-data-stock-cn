# 对标表(Benchmark Map)

本表是本库端点与 akshare 公开接口的对照,兼作**覆盖率追踪器**与**上游同步锚点**(见 ADR-0012)。

- `本库路径`:实现该端点的 Rust 路径(实现后填写)。
- `akshare 源文件:行`:akshare 中对应该函数的源位置(用于 `scripts/sync-akshare` 比对)。
- `状态`:`TODO` / `WIP` / `DONE` / `DEFERRED`(需签名逆向)。

## Milestone 1

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `stock_zh_a_spot_em` | `src/stock/spot/eastmoney.rs::spot` | 东财 | `stock_feature/stock_hist_em.py` | DONE |
| `stock_zh_a_spot` | `src/stock/spot/sina.rs::spot` | 新浪 | `stock/stock_zh_a_sina.py` | DONE |
| `stock_zh_a_spot_tx` | `src/stock/spot/tencent.rs::spot` | 腾讯 | `stock/stock_zh_a_tx.py` | DONE |
| `stock_zh_a_hist` | `src/stock/hist/eastmoney.rs::daily` | 东财 | `stock_feature/stock_hist_em.py` | DONE |
| `stock_zh_a_hist_tx` | `src/stock/hist/tencent.rs::daily` | 腾讯 | `stock_feature/stock_hist_tx.py` | DONE |
| `stock_intraday_em` | `src/stock/intraday/eastmoney.rs::em` | 东财 | `stock/stock_intraday_em.py` | DONE |
| `stock_intraday_sina` | `src/stock/intraday/sina.rs::sina` | 新浪 | `stock/stock_intraday_sina.py` | DONE |
| `stock_zh_index_spot_em` | `src/stock/index/eastmoney.rs::spot` | 东财 | `index/index_stock_zh.py` | DONE |
| `stock_zh_index_spot_sina` | `src/stock/index/sina.rs::spot` | 新浪 | `index/index_stock_zh.py` | DONE |
| `index_zh_a_hist` | `src/stock/index/eastmoney.rs::daily` | 东财 | `index/index_zh_em.py` | DONE |
| `stock_zh_a_daily` | _(待填)_ | 新浪 | `stock/stock_zh_a_sina.py` | DEFERRED |

> 行号锚点为规划值,`scripts/sync-akshare` 运行时会刷新并与 registry 比对,产出"新增/变更/移除"报告。

## Forex(外汇)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `forex_spot_em` | `src/forex/eastmoney.rs::spot` | 东财 | `akshare/forex/forex_em.py` | DONE |
| `forex_hist_em` | `src/forex/eastmoney.rs::hist` | 东财 | `akshare/forex/forex_em.py` | DONE |

## Rate(利率)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `rate_interbank` | `src/rate/eastmoney.rs::rate_interbank` | 东财 | `akshare/interest_rate/interbank_rate_em.py` | DONE |
| `rate.repo_rate_hist` | `src/rate/chinamoney.rs::repo_rate_hist` | 外汇交易中心 | `akshare/rate/rate_china.py` | DONE |
| `rate.repo_rate_query` | `src/rate/chinamoney.rs::repo_rate_query` | 外汇交易中心 | `akshare/rate/rate_china.py` | DONE |

## Bond(债券)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `bond_zh_us_rate` | `src/bond/eastmoney.rs::bond_zh_us_rate` | 东财 | `akshare/bond/bond_em.py` | DONE |
| `bond_cov_comparison` | `src/bond/eastmoney.rs::bond_cov_comparison` | 东财 | `akshare/bond/bond_zh_cov.py` | DONE |
| `bond_spot_quote` | `src/bond/chinamoney.rs::bond_spot_quote` | 外汇交易中心 | `akshare/bond/bond_china.py` | DONE |
| `bond_spot_deal` | `src/bond/chinamoney.rs::bond_spot_deal` | 外汇交易中心 | `akshare/bond/bond_china.py` | DONE |

## Crypto(数字货币)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `crypto_js_spot` | `src/crypto/js_spot.rs::crypto_js_spot` | 金十 | `akshare/economic/macro_other.py` | DONE |
| `crypto_bitcoin_cme` | `src/crypto/bitcoin_cme.rs::crypto_bitcoin_cme` | 金十 | `akshare/crypto/crypto_bitcoin_cme.py` | DONE |
| `crypto_bitcoin_hold_report` | `src/crypto/bitcoin_hold.rs::crypto_bitcoin_hold_report` | 金十 | `akshare/crypto/crypto_hold.py` | DONE |

## Economic / Macro(宏观)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `macro_china_gdp` | `src/economic/china.rs::macro_china_gdp` | 东财 | `akshare/economic/macro_china.py` | DONE |
| `macro_china_cpi` | `src/economic/china.rs::macro_china_cpi` | 东财 | `akshare/economic/macro_china.py` | DONE |
| `macro_china_ppi` | `src/economic/china.rs::macro_china_ppi` | 东财 | `akshare/economic/macro_china.py` | DONE |
| `macro_china_money_supply` | `src/economic/china.rs::macro_china_money_supply` | 东财 | `akshare/economic/macro_china.py` | DONE |

## Futures(期货)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `futures_zh_daily` | `src/futures/daily.rs::daily` | 东财 | `akshare/futures/futures_hist_em.py` | DONE |
| `futures_zh_spot` | `src/futures/spot.rs::spot` | 东财 | `akshare/futures/futures_hf_em.py` | DONE |
| `futures_inventory` | `src/futures/inventory.rs::inventory` | 东财 | `akshare/futures/futures_inventory_em.py` | DONE |

## Option(期权)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `option_daily` | `src/option/eastmoney.rs::option_daily` | 东财 | `akshare/option/option_em.py` | DONE |
| `option_minute` | `src/option/eastmoney.rs::option_minute` | 东财 | `akshare/option/option_em.py` | DONE |
| `option_sina_spot` | `src/option/sina.rs::option_sina_spot` | 新浪 | `akshare/option/option_finance_sina.py` | DONE |
| `option_cffex_daily` | `src/option/sina.rs::option_cffex_daily` | 新浪 | `akshare/option/option_finance_sina.py` | DONE |

## Fund(基金)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `fund_etf_spot_em` | `src/fund/etf.rs::fund_etf_spot_em` | 东财 | `akshare/fund/fund_etf_em.py` | DONE |
| `fund_etf_hist_em` | `src/fund/etf.rs::fund_etf_hist_em` | 东财 | `akshare/fund/fund_etf_em.py` | DONE |
| `fund_lof_spot_em` | `src/fund/lof.rs::fund_lof_spot_em` | 东财 | `akshare/fund/fund_lof_em.py` | DONE |
| `fund_open_fund_info` | `src/fund/open_fund.rs::fund_open_fund_info` | 东财 | `akshare/fund/fund_em.py` | DONE |

## Stock 扩展(基本面 / 跨市场)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `stock_profit_sheet_by_report_em` | `src/stock/fundamental/eastmoney.rs` | 东财 | `akshare/stock_feature/stock_three_report_em.py` | DONE |
| `stock_balance_sheet_by_report_em` | `src/stock/fundamental/eastmoney.rs` | 东财 | `akshare/stock_feature/stock_three_report_em.py` | DONE |
| `stock_cash_flow_sheet_by_report_em` | `src/stock/fundamental/eastmoney.rs` | 东财 | `akshare/stock_feature/stock_three_report_em.py` | DONE |
| `stock_financial_analysis_indicator_em` | `src/stock/fundamental/eastmoney.rs` | 东财 | `akshare/stock_fundamental/stock_finance_sina.py` | DONE |
| `stock_hk_spot_em` | `src/stock/cross/hk.rs::stock_hk_spot_em` | 东财 | `akshare/stock_feature/stock_hist_em.py` | DONE |
| `stock_hk_hist` | `src/stock/cross/hk.rs::stock_hk_hist` | 东财 | `akshare/stock_feature/stock_hist_em.py` | DONE |
| `stock_us_spot_em` | `src/stock/cross/us.rs::stock_us_spot_em` | 东财 | `akshare/stock_feature/stock_hist_em.py` | DONE |
| `stock_us_hist` | `src/stock/cross/us.rs::stock_us_hist` | 东财 | `akshare/stock_feature/stock_hist_em.py` | DONE |
