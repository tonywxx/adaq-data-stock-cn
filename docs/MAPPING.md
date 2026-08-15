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

## Stock 扩展:新浪日线 / 杂项(长尾补全)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `stock_zh_a_daily` | `src/stock/daily_sina.rs::stock_zh_a_daily` | 新浪 | `akshare/stock/stock_zh_a_sina.py` | DONE |
| `stock_zh_a_gdhs` | `src/stock/extra.rs::stock_zh_a_gdhs` | 东财 | `akshare/stock_feature/stock_holder_num.py` | DONE |
| `stock_dividend` | `src/stock/extra.rs::stock_dividend` | 巨潮/cninfo | `akshare/stock/stock_dividend.py` | DONE |
| `stock_rank_em` | `src/stock/extra.rs::stock_rank_em` | 东财 app | `akshare/stock/stock_rank_em.py` | DONE |

## Board(板块)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `stock_board_industry_name_em` | `src/board/industry.rs::stock_board_industry_name_em` | 东财 | `akshare/stock/board.py` | DONE |
| `stock_board_industry_cons_em` | `src/board/industry.rs::stock_board_industry_cons_em` | 东财 | `akshare/stock/board.py` | DONE |
| `stock_board_concept_name_em` | `src/board/concept.rs::stock_board_concept_name_em` | 东财 | `akshare/stock/board.py` | DONE |
| `stock_board_concept_cons_em` | `src/board/concept.rs::stock_board_concept_cons_em` | 东财 | `akshare/stock/board.py` | DONE |

## Calendar(交易日历)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `tool_trade_date_hist_sina` | `src/calendar/mod.rs::tool_trade_date` | 新浪 | `akshare/tool/trade_date_hist.py` | DONE |

## LPR

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `rate_zh_1y_lpr` / `rate_zh_5y_lpr`(聚合为 `lpr`) | `src/lpr/mod.rs::lpr` | 全国银行间同业拆借中心 | `akshare/rate/rate_lpr.py` | DONE |

## News(资讯 / NLP)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `news_economic_baidu` | `src/news/baidu_calendar.rs::news_economic_baidu` | 百度财经 | `akshare/news/news_baidu.py` | DONE |
| `news_trade_notify_suspend_baidu` | `src/news/baidu_calendar.rs::news_trade_notify_suspend_baidu` | 百度财经 | `akshare/news/news_baidu.py` | DONE |
| `news_trade_notify_dividend_baidu` | `src/news/baidu_calendar.rs::news_trade_notify_dividend_baidu` | 百度财经 | `akshare/news/news_baidu.py` | DONE |
| `news_report_time_baidu` | `src/news/baidu_calendar.rs::news_report_time_baidu` | 百度财经 | `akshare/news/news_baidu.py` | DONE |
| `stock_news_em` | `src/news/stock_news.rs::stock_news_em` | 东财 | `akshare/news/news_stock_em.py` | DONE |
| `nlp_ownthink` | `src/news/nlp_ownthink.rs::nlp_ownthink` | OwnThink | `akshare/nlp/nlp_ownthink.py` | DONE |
| `nlp_answer` | `src/news/nlp_ownthink.rs::nlp_answer` | OwnThink | `akshare/nlp/nlp_ownthink.py` | DONE |

## Alt(另类数据)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `energy_oil_hist` | `src/alt/energy.rs::energy_oil_hist` | 金十/上期所 | `akshare/energy/energy_oil.py` | DONE |
| `energy_oil_detail` | `src/alt/energy.rs::energy_oil_detail` | 金十/上期所 | `akshare/energy/energy_oil.py` | DONE |
| `movie_boxoffice_daily` | `src/alt/movie.rs::movie_boxoffice_daily` | 艺恩 | `akshare/movie/movie_boxoffice_em.py` | DONE |
| `movie_boxoffice_realtime` | `src/alt/movie.rs::movie_boxoffice_realtime` | 艺恩 | `akshare/movie/movie_boxoffice_em.py` | DONE |
| `movie_boxoffice_monthly` | `src/alt/movie.rs::movie_boxoffice_monthly` | 艺恩 | `akshare/movie/movie_boxoffice_em.py` | DONE |
| `movie_boxoffice_yearly` | `src/alt/movie.rs::movie_boxoffice_yearly` | 艺恩 | `akshare/movie/movie_boxoffice_em.py` | DONE |
| `fx_spot_quote` | `src/alt/fx.rs::fx_spot_quote` | 外汇交易中心 | `akshare/alt_fx.py` | DONE |
| `fx_pair_quote` | `src/alt/fx.rs::fx_pair_quote` | 外汇交易中心 | `akshare/alt_fx.py` | DONE |
| `bank_fx_spot`(别名) | `src/alt/fx.rs::bank_fx_spot` | 外汇交易中心 | `akshare/alt_fx.py` | DONE |

## Stock 杂项(misc)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `stock_zh_a_hist_min_em` | `src/stock/misc.rs::stock_zh_a_hist_min_em` | 东财 | `akshare/stock_feature/stock_hist_em.py` | DONE |
| `stock_zh_a_minute` | `src/stock/misc.rs::stock_zh_a_minute` | 新浪 | `akshare/stock/stock_zh_a_sina.py` | DONE |
| `stock_zh_a_new` | `src/stock/misc.rs::stock_zh_a_new` | 新浪/东财 | `akshare/stock/stock_zh_a_special.py` | DONE |
| `stock_zh_a_stop` | `src/stock/misc.rs::stock_zh_a_stop` | 东财 | `akshare/stock/stock_zh_a_special.py` | DONE |
| `stock_summary` | `src/stock/misc.rs::stock_summary` | 东财 | (东财 clist 概况) | DONE |

## Stock 指数扩展

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `index_zh_a_spot` | `src/stock/index/extra.rs::index_zh_a_spot` | 东财 | `akshare/index/index_stock_zh.py` | DONE |
| `index_zh_a_daily` | `src/stock/index/extra.rs::index_zh_a_daily` | 东财 | `akshare/index/index_zh_a_hist.py` | DONE |
| `stock_zh_index_daily` | `src/stock/index/extra.rs::stock_zh_index_daily` | 腾讯 | `akshare/stock/stock_zh_index_daily.py` | DONE |
| `index_stock_cons` | `src/stock/index/extra.rs::index_stock_cons` | 东财 | `akshare/index/index_stock_cons.py` | DONE |

## Futures 扩展

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `futures_zh_daily_sina` | `src/futures/extra.rs::futures_zh_daily_sina` | 新浪 | `akshare/futures/futures_zh_sina.py` | DONE |
| `futures_foreign` | `src/futures/extra.rs::futures_foreign` | 新浪 | `akshare/futures/futures_foreign.py` | DONE |
| `futures_inventory_em` | `src/futures/extra.rs::futures_inventory_em` | 东财 | `akshare/futures/futures_inventory_em.py` | DONE |
| `futures_comex_inventory` | `src/futures/extra.rs::futures_comex_inventory` | 东财 | `akshare/futures/futures_comex_em.py` | DONE |

## Fund 扩展

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `fund_name`(→ `fund_open_fund_name_em`) | `src/fund/extra.rs::fund_open_fund_name_em` | 东财 | `akshare/fund/fund_em.py` | DONE |
| `fund_value_em` | `src/fund/extra.rs::fund_value_em` | 东财 | `akshare/fund/fund_em.py` | DONE |
| `fund_hist_em` | `src/fund/extra.rs::fund_hist_em` | 东财 | `akshare/fund/fund_em.py` | DONE |
| `fund_money_meta` | `src/fund/extra.rs::fund_money_meta` | 东财 | `akshare/fund/fund_em.py` | DONE |
| `fund_etf_category_sina` | `src/fund/extra.rs::fund_etf_category_sina` | 新浪 | `akshare/fund/fund_etf_sina.py` | DONE |

## Forex 扩展(中行 / 人民币掉期)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `currency_boc` | `src/forex/extra.rs::currency_boc` | 中国银行 | `akshare/currency/currency_boc.py` | DONE |
| `currency_hist` | `src/forex/extra.rs::currency_hist` | 中行(新浪) | `akshare/currency/currency_china_bank_sina.py` | DONE |
| `fx_swap_quote` | `src/forex/extra.rs::fx_swap_quote` | 外汇交易中心 | `akshare/fx/fx_quote.py` | DONE |

## Crypto 扩展(Binance / OKX)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `crypto_hist`(Binance/OKX) | `src/crypto/extra.rs::crypto_hist` | Binance/OKX | (公开 REST) | DONE |
| `crypto_spot`(Binance/OKX) | `src/crypto/extra.rs::crypto_spot` | Binance/OKX | (公开 REST) | DONE |
| `crypto_info` | `src/crypto/extra.rs::crypto_info` | Binance | (公开 REST) | DONE |
| `crypto_name_map` | `src/crypto/extra.rs::crypto_name_map` | Binance | (公开 REST) | DONE |

## Stock 资金流 / 沪深港通(flow)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `stock_individual_fund_flow` | `src/stock/flow.rs::individual_fund_flow` | 东财 | `akshare/stock/stock_fund_em.py` | DONE |
| `stock_market_fund_flow` | `src/stock/flow.rs::market_fund_flow` | 东财 | `akshare/stock/stock_fund_em.py` | DONE |
| `stock_individual_fund_flow_rank` | `src/stock/flow.rs::individual_fund_flow_rank` | 东财 | `akshare/stock/stock_fund_em.py` | DONE |
| `stock_hsgt_fund_flow_summary_em` | `src/stock/flow.rs::hsgt_fund_flow_summary_em` | 东财 | `akshare/stock_feature/stock_hsgt_em.py` | DONE |
| `stock_hsgt_hist_em` | `src/stock/flow.rs::hsgt_hist_em` | 东财 | `akshare/stock_feature/stock_hsgt_em.py` | DONE |

## Stock 个股信息 / 主营构成 / 板块(holder)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `stock_individual_info_em` | `src/stock/holder.rs::stock_individual_info_em` | 东财 | `akshare/stock/stock_info_em.py` | DONE |
| `stock_zygc_em`(主营构成) | `src/stock/holder.rs::stock_zygc_em` | 东财 | `akshare/stock_fundamental/stock_zygc.py` | DONE |
| `stock_sector_spot` | `src/stock/holder.rs::stock_sector_spot` | 新浪 | `akshare/stock/stock_industry.py` | DONE |

## Stock 融资融券 / 业绩报表(margin)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `stock_margin_sse`(→ `stock_margin_sh`) | `src/stock/margin.rs::stock_margin_sh` | 上交所 | `akshare/stock_feature/stock_margin_sse.py` | DONE |
| `stock_margin_szse`(→ `stock_margin_sz`) | `src/stock/margin.rs::stock_margin_sz` | 深交所 | `akshare/stock_feature/stock_margin_szse.py` | DONE |
| `stock_yjbb_em` | `src/stock/margin.rs::stock_yjbb_em` | 东财 | `akshare/stock_feature/stock_yjbb_em.py` | DONE |

## Bond 扩展

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `bond_zh_cov` | `src/bond/extra.rs::bond_zh_cov` | 东财 | `akshare/bond/bond_zh_cov.py` | DONE |
| `bond_zh_cov_value_analysis` | `src/bond/extra.rs::bond_zh_cov_value_analysis` | 东财 | `akshare/bond/bond_zh_cov.py` | DONE |
| `bond_sh_buy_back_em` | `src/bond/extra.rs::bond_sh_buy_back_em` | 东财 | `akshare/bond/bond_buy_back_em.py` | DONE |
| `bond_sz_buy_back_em` | `src/bond/extra.rs::bond_sz_buy_back_em` | 东财 | `akshare/bond/bond_buy_back_em.py` | DONE |

## Economic 宏观扩展

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `macro_china_new_house_price` | `src/economic/extra.rs::macro_china_new_house_price` | 东财 | `akshare/economic/macro_china.py` | DONE |
| `macro_china_lpr` | `src/economic/extra.rs::macro_china_lpr` | 东财 | `akshare/economic/macro_china.py` | DONE |
| `macro_china_enterprise_boom_index` | `src/economic/extra.rs::macro_china_enterprise_boom_index` | 东财 | `akshare/economic/macro_china.py` | DONE |
| `macro_china_national_tax_receipts` | `src/economic/extra.rs::macro_china_national_tax_receipts` | 东财 | `akshare/economic/macro_china.py` | DONE |
| `macro_china_qyspjg` | `src/economic/extra.rs::macro_china_qyspjg` | 东财 | `akshare/economic/macro_china.py` | DONE |
| `macro_china_fdi` | `src/economic/extra.rs::macro_china_fdi` | 东财 | `akshare/economic/macro_china.py` | DONE |

## Futures 主力 / 合约列表(main, 新浪)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `futures_main_sina`(→ `futures_main`) | `src/futures/main.rs::futures_main` | 新浪 | `akshare/futures/futures_main.py` | DONE |
| `futures_display_main_sina`(→ `futures_display`) | `src/futures/main.rs::futures_display` | 新浪 | `akshare/futures/futures_display.py` | DONE |

## Option 扩展(option::extra)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `option_current_em` | `src/option/extra.rs::option_current_em` | 东财 | `akshare/option/option_em.py:14` | DONE |
| `option_current_cffex_em` | `src/option/extra.rs::option_current_cffex_em` | 东财 | `akshare/option/option_em.py:112` | DONE |
| `option_risk_indicator_sse` | `src/option/extra.rs::option_risk_indicator_sse` | 上交所 | `akshare/option/option_risk_indicator_sse.py:12` | DONE |
| `option_current_day_sse` | `src/option/extra.rs::option_current_day_sse` | 上交所 | `akshare/option/option_current_sse.py:13` | DONE |
| `option_daily_stats_sse` | `src/option/extra.rs::option_daily_stats_sse` | 深交所 | `akshare/option/option_daily_stats_sse_szse.py:15` | DONE |
| `option_cffex_spot_sina`(统一 sz50/hs300/zz1000 三只标的) | `src/option/extra.rs::option_cffex_spot_sina` | 新浪 | `akshare/option/option_finance_sina.py:77/150/223` | DONE |

## Fund 扩展2(fund::more)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `fund_aum_trend_em` | `src/fund/more.rs::fund_aum_trend_em` | 东财 | `akshare/fund/fund_aum_em.py:45` | DONE |
| `fund_name_em` | `src/fund/more.rs::fund_name_em` | 东财 | `akshare/fund/fund_em.py:218` | DONE |
| `fund_fh_em` | `src/fund/more.rs::fund_fh_em` | 东财 | `akshare/fund/fund_fhsp_em.py:15` | DONE |
| `fund_scale_change_em` | `src/fund/more.rs::fund_scale_change_em` | 东财 | `akshare/fund/fund_scale_em.py:15` | DONE |
| `fund_hold_structure_em` | `src/fund/more.rs::fund_hold_structure_em` | 东财 | `akshare/fund/fund_scale_em.py:71` | DONE |
| `fund_manager_em` | `src/fund/more.rs::fund_manager_em` | 东财 | `akshare/fund/fund_manager.py:16` | DONE |

## Stock 更多(stock::more)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `stock_zh_a_st_em` | `src/stock/more.rs::stock_zh_a_st_em` | 东财 | `akshare/stock/stock_zh_a_special.py:20` | DONE |
| `stock_a_high_low_statistics` | `src/stock/more.rs::stock_a_high_low_statistics` | 乐咕 | `akshare/stock_feature/stock_a_high_low.py:15` | DONE |
| `stock_a_below_net_asset_statistics` | `src/stock/more.rs::stock_a_below_net_asset_statistics` | 乐咕 | `akshare/stock_feature/stock_a_below_net_asset_statistics.py:15` | DONE |
| `stock_account_statistics_em` | `src/stock/more.rs::stock_account_statistics_em` | 东财 | `akshare/stock_feature/stock_account_em.py:14` | DONE |
| `stock_zt_pool_em` | `src/stock/more.rs::stock_zt_pool_em` | 东财 | `akshare/stock_feature/stock_ztb_em.py:24` | DONE |

## Index 更多(stock::index::more)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `index_global_spot_em` | `src/stock/index/more.rs::index_global_spot_em` | 东财 | `akshare/index/index_global_em.py:15` | DONE |
| `index_global_hist_em` | `src/stock/index/more.rs::index_global_hist_em` | 东财 | `akshare/index/index_global_em.py:95` | DONE |
| `index_zh_a_hist_min_em` | `src/stock/index/more.rs::index_zh_a_hist_min_em` | 东财 | `akshare/index/index_zh_em.py:178` | DONE |
| `stock_zh_index_hist_csindex` | `src/stock/index/more.rs::stock_zh_index_hist_csindex` | 中证 | `akshare/index/index_stock_zh_csindex.py:13` | DONE |
| `index_pmi_cx`(统一 com/man/ser 三口径) | `src/stock/index/more.rs::index_pmi_cx` | 财新 | `akshare/index/index_cx.py:13/41/69` | DONE |

## Economic 宏观第二批(economic::macro2)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `macro_china_pmi_yearly`(→ `macro_china_pmi`) | `src/economic/macro2.rs::macro_china_pmi` | 东财 | `akshare/economic/macro_china.py:544` | DONE |
| `macro_china_gdzctz` | `src/economic/macro2.rs::macro_china_gdzctz` | 东财 | `akshare/economic/macro_china.py:2674` | DONE |
| `macro_china_gyzjz` | `src/economic/macro2.rs::macro_china_gyzjz` | 东财 | `akshare/economic/macro_china.py:3051` | DONE |
| `macro_china_consumer_goods_retail` | `src/economic/macro2.rs::macro_china_consumer_goods_retail` | 东财 | `akshare/economic/macro_china.py:3180` | DONE |
| `macro_usa_cpi_yoy` | `src/economic/macro2.rs::macro_usa_cpi_yoy` | 东财 | `akshare/economic/macro_usa.py:129` | DONE |
| `macro_usa_phs` | `src/economic/macro2.rs::macro_usa_phs` | 东财 | `akshare/economic/macro_usa.py:79` | DONE |

## Coin 金属 / 外盘(coin,新增顶层域)

| akshare 函数 | 本库路径 | 源 | akshare 源文件:行 | 状态 |
|---|---|---|---|---|
| `futures_hq_sina`(LME 实时,统一到 `coin_lme_realtime`) | `src/coin/mod.rs::coin_lme_realtime` | 新浪 | `akshare/futures/futures_hq_sina.py:72` | DONE |
| `futures_shfe_position_rank`(→ `coin_shfe_rank`) | `src/coin/mod.rs::coin_shfe_rank` | 上期所 | `akshare/futures/cot.py:275`(`get_shfe_rank_table`) | DONE |
| `futures_foreign_hist`(→ `coin_foreign_hist`,本库走东财 push2his) | `src/coin/mod.rs::coin_foreign_hist` | 东财 | `akshare/futures/futures_foreign.py:20` | DONE |
| `futures_hist_em`(→ `coin_futures_hist`) | `src/coin/mod.rs::coin_futures_hist` | 东财 | `akshare/futures/futures_hist_em.py:91` | DONE |
| `futures_rule_em`(→ `coin_futures_symbol_map`) | `src/coin/mod.rs::coin_futures_symbol_map` | 东财 | `akshare/futures/futures_rule_em.py:14` | DONE |
