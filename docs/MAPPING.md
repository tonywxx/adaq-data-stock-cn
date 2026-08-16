# 对标表 (Benchmark Map)

本表是本库端点与 akshare 公开接口的完整对照,兼作**覆盖率追踪器**与**上游同步锚点**(见 ADR-0012)。

> 本文件由 `scripts/sync-akshare` 思路自动生成:逐一对齐 akshare 顶层 `def`(1172 个公开函数)与本库 `src/` 实现。
> 状态:`DONE`=已移植;`DEFERRED`=需签名/令牌/JS/HTML,按 ADR-0008 推迟;`INTERNAL`=akshare 内部辅助函数,非对外数据端点。

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|

## air

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `air_city_table` | `src/air/air_gaps.rs::air_city_table` | `air/air_zhenqi.py:64` | DONE |  |
| `air_quality_hebei` | — | `air/air_hebei.py:23` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `air_quality_hist` | — | `air/air_zhenqi.py:142` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `air_quality_rank` | `src/air/air_gaps.rs::air_quality_rank` | `air/air_zhenqi.py:219` | DONE |  |
| `air_quality_watch_point` | — | `air/air_zhenqi.py:99` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `has_month_data` | — | `air/air_zhenqi.py:53` | INTERNAL | akshare internal helper, not a data endpoint |
| `sunrise_city_list` | — | `air/sunrise_tad.py:15` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `sunrise_daily` | — | `air/sunrise_tad.py:40` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `sunrise_monthly` | — | `air/sunrise_tad.py:73` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |

## article

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `article_epu_index` | `src/article/excel_gaps.rs::article_epu_index` | `article/epu_index.py:12` | DONE |  |
| `article_ff_crr` | `src/article/article_gaps.rs::article_ff_crr` | `article/ff_factor.py:17` | DONE |  |
| `article_oman_rv` | — | `article/risk_rv.py:18` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `article_oman_rv_short` | — | `article/risk_rv.py:78` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `article_rlab_rv` | `src/article/article_gaps.rs::article_rlab_rv` | `article/risk_rv.py:117` | DONE |  |
| `fred_md` | `article/fred.rs::fred_md` | `article/fred_md.py:13` | DONE |  |
| `fred_qd` | `article/fred.rs::fred_qd` | `article/fred_md.py:28` | DONE |  |

## bank

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `bank_fjcf_page_url` | — | `bank/bank_cbirc_2020.py:76` | DEFERRED | token/JS/HTML-gated |
| `bank_fjcf_table_detail` | `src/bank/bank_gaps.rs::bank_fjcf_table_detail` | `bank/bank_cbirc_2020.py:111` | DONE |  |
| `bank_fjcf_total_num` | — | `bank/bank_cbirc_2020.py:22` | DEFERRED | token/JS/HTML-gated |
| `bank_fjcf_total_page` | — | `bank/bank_cbirc_2020.py:47` | DEFERRED | token/JS/HTML-gated |

## bond

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `bond_available_index_cbond` | `bond/wv_bond_misc.rs::bond_available_index_cbond` | `bond/bond_cbond.py:14` | DONE |  |
| `bond_buy_back_hist_em` | `bond/wv_bond_misc.rs::bond_buy_back_hist_em` | `bond/bond_buy_back_em.py:158` | DONE |  |
| `bond_cash_summary_sse` | `src/bond/excel_gaps.rs::bond_cash_summary_sse` | `bond/bond_summary.py:15` | DONE |  |
| `bond_cb_adj_logs_jsl` | — | `bond/bond_convert.py:297` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `bond_cb_index_jsl` | — | `bond/bond_convert.py:17` | DEFERRED | token/JS/HTML-gated |
| `bond_cb_jsl` | `bond/jisilu.rs::bond_cb_jsl` | `bond/bond_convert.py:31` | DONE |  |
| `bond_cb_profile_sina` | — | `bond/bond_cb_sina.py:15` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `bond_cb_redeem_jsl` | `bond/jisilu.rs::bond_cb_redeem_jsl` | `bond/bond_convert.py:165` | DONE |  |
| `bond_cb_summary_sina` | — | `bond/bond_cb_sina.py:31` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `bond_china_close_return` | `src (present)` | `bond/bond_china_money.py:127` | DONE |  |
| `bond_china_close_return_map` | `bond/chinamoney_pub.rs::bond_china_close_return_map` | `bond/bond_china_money.py:93` | DONE |  |
| `bond_china_yield` | — | `bond/bond_china.py:142` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `bond_composite_index_cbond` | `bond/cbond.rs::bond_composite_index_cbond` | `bond/bond_cbond.py:214` | DONE |  |
| `bond_corporate_issue_cninfo` | — | `bond/bond_issue_cninfo.py:222` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `bond_cov_comparison` | `bond/eastmoney.rs::bond_cov_comparison` | `bond/bond_zh_cov.py:465` | DONE |  |
| `bond_cov_issue_cninfo` | — | `bond/bond_issue_cninfo.py:322` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `bond_cov_stock_issue_cninfo` | — | `bond/bond_issue_cninfo.py:481` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `bond_deal_summary_sse` | `src/bond/excel_gaps.rs::bond_deal_summary_sse` | `bond/bond_summary.py:50` | DONE |  |
| `bond_debt_nafmii` | `bond/zh.rs::bond_debt_nafmii` | `bond/bond_nafmii.py:13` | DONE |  |
| `bond_gb_us_sina` | `bond/zh.rs::bond_gb_us_sina` | `bond/bond_gb_sina.py:54` | DONE |  |
| `bond_gb_zh_sina` | `bond/zh.rs::bond_gb_zh_sina` | `bond/bond_gb_sina.py:13` | DONE |  |
| `bond_index_general_cbond` | `bond/cbond.rs::bond_index_general_cbond` | `bond/bond_cbond.py:28` | DONE |  |
| `bond_info_cm` | `bond/wv_bond_misc.rs::bond_info_cm` | `bond/bond_info_cm.py:65` | DONE |  |
| `bond_info_cm_query` | `bond/wv_bond_misc.rs::bond_info_cm_query` | `bond/bond_info_cm.py:19` | DONE |  |
| `bond_info_detail_cm` | `bond/wv_bond_misc.rs::bond_info_detail_cm` | `bond/bond_info_cm.py:183` | DONE |  |
| `bond_local_government_issue_cninfo` | — | `bond/bond_issue_cninfo.py:126` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `bond_new_composite_index_cbond` | `bond/cbond.rs::bond_new_composite_index_cbond` | `bond/bond_cbond.py:130` | DONE |  |
| `bond_sh_buy_back_em` | `bond/extra.rs::bond_sh_buy_back_em` | `bond/bond_buy_back_em.py:14` | DONE |  |
| `bond_spot_deal` | `bond/chinamoney.rs::bond_spot_deal` | `bond/bond_china.py:84` | DONE |  |
| `bond_spot_quote` | `bond/chinamoney.rs::bond_spot_quote` | `bond/bond_china.py:20` | DONE |  |
| `bond_sz_buy_back_em` | `bond/extra.rs::bond_sz_buy_back_em` | `bond/bond_buy_back_em.py:86` | DONE |  |
| `bond_treasure_issue_cninfo` | — | `bond/bond_issue_cninfo.py:30` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `bond_treasury_index_cbond` | `bond/cbond.rs::bond_treasury_index_cbond` | `bond/bond_cbond.py:72` | DONE |  |
| `bond_zh_cov` | `bond/extra.rs::bond_zh_cov` | `bond/bond_zh_cov.py:309` | DONE |  |
| `bond_zh_cov_info` | `bond/cov.rs::bond_zh_cov_info` | `bond/bond_zh_cov.py:542` | DONE |  |
| `bond_zh_cov_info_ths` | `bond/wv_bond_misc.rs::bond_zh_cov_info_ths` | `bond/bond_cb_ths.py:13` | DONE |  |
| `bond_zh_cov_value_analysis` | `bond/extra.rs::bond_zh_cov_value_analysis` | `bond/bond_zh_cov.py:627` | DONE |  |
| `bond_zh_hs_cov_daily` | `bond/cov.rs::bond_zh_hs_cov_daily` | `bond/bond_zh_cov.py:65` | DONE |  |
| `bond_zh_hs_cov_min` | `bond/cov.rs::bond_zh_hs_cov_min` | `bond/bond_zh_cov.py:131` | DONE |  |
| `bond_zh_hs_cov_pre_min` | `bond/cov.rs::bond_zh_hs_cov_pre_min` | `bond/bond_zh_cov.py:264` | DONE |  |
| `bond_zh_hs_cov_spot` | `bond/cov.rs::bond_zh_hs_cov_spot` | `bond/bond_zh_cov.py:46` | DONE |  |
| `bond_zh_hs_daily` | — | `bond/bond_zh_sina.py:118` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `bond_zh_hs_spot` | — | `bond/bond_zh_sina.py:45` | DEFERRED | token/JS/HTML-gated |
| `bond_zh_us_rate` | `bond/eastmoney.rs::bond_zh_us_rate` | `bond/bond_em.py:14` | DONE |  |
| `get_zh_bond_hs_page_count` | — | `bond/bond_zh_sina.py:27` | INTERNAL | akshare internal helper, not a data endpoint |
| `macro_china_bond_public` | `bond/chinamoney.rs::macro_china_bond_public` | `bond/bond_china_money.py:313` | DONE |  |
| `macro_china_swap_rate` | `src/economic/macro_gaps.rs::macro_china_swap_rate` | `bond/bond_china_money.py:192` | DONE |  |

## cal

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `rv_from_futures_zh_minute_sina` | `cal/mod.rs::rv_from_futures_zh_minute_sina` | `cal/rv.py:61` | DONE |  |
| `rv_from_stock_zh_a_hist_min_em` | `cal/mod.rs::rv_from_stock_zh_a_hist_min_em` | `cal/rv.py:13` | DONE |  |
| `volatility_yz_rv` | `cal/mod.rs::volatility_yz_rv` | `cal/rv.py:92` | DONE |  |

## crypto

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `crypto_bitcoin_cme` | `crypto/bitcoin_cme.rs::crypto_bitcoin_cme` | `crypto/crypto_bitcoin_cme.py:13` | DONE |  |
| `crypto_bitcoin_hold_report` | `crypto/bitcoin_hold.rs::crypto_bitcoin_hold_report` | `crypto/crypto_hold.py:13` | DONE |  |

## currency

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `currency_boc_safe` | — | `currency/currency_safe.py:18` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `currency_boc_sina` | — | `currency/currency_china_bank_sina.py:57` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `currency_convert` | `currency/api.rs::currency_convert` | `currency/currency.py:126` | DONE |  |
| `currency_currencies` | `currency/api.rs::currency_currencies` | `currency/currency.py:107` | DONE |  |
| `currency_history` | `currency/api.rs::currency_history` | `currency/currency.py:39` | DONE |  |
| `currency_latest` | `currency/api.rs::currency_latest` | `currency/currency.py:14` | DONE |  |
| `currency_time_series` | `currency/api.rs::currency_time_series` | `currency/currency.py:66` | DONE |  |

## datasets.py

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `get_crypto_info_csv` | — | `datasets.py:23` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_registry_json` | — | `datasets.py:34` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_ths_js` | — | `datasets.py:12` | INTERNAL | akshare internal helper, not a data endpoint |

## economic

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `crypto_js_spot` | `crypto/js_spot.rs::crypto_js_spot` | `economic/macro_other.py:14` | DONE |  |
| `macro_australia_bank_rate` | `economic/macro_intl.rs::macro_australia_bank_rate` | `economic/macro_australia.py:320` | DONE |  |
| `macro_australia_cpi_quarterly` | `economic/macro_intl.rs::macro_australia_cpi_quarterly` | `economic/macro_australia.py:218` | DONE |  |
| `macro_australia_cpi_yearly` | `economic/macro_intl.rs::macro_australia_cpi_yearly` | `economic/macro_australia.py:269` | DONE |  |
| `macro_australia_ppi_quarterly` | `economic/macro_intl.rs::macro_australia_ppi_quarterly` | `economic/macro_australia.py:167` | DONE |  |
| `macro_australia_retail_rate_monthly` | `economic/macro_intl.rs::macro_australia_retail_rate_monthly` | `economic/macro_australia.py:14` | DONE |  |
| `macro_australia_trade` | `economic/macro_intl.rs::macro_australia_trade` | `economic/macro_australia.py:65` | DONE |  |
| `macro_australia_unemployment_rate` | `economic/macro_intl.rs::macro_australia_unemployment_rate` | `economic/macro_australia.py:116` | DONE |  |
| `macro_bank_australia_interest_rate` | `src (present)` | `economic/macro_bank.py:172` | DONE |  |
| `macro_bank_brazil_interest_rate` | `src (present)` | `economic/macro_bank.py:220` | DONE |  |
| `macro_bank_china_interest_rate` | `src (present)` | `economic/macro_bank.py:136` | DONE |  |
| `macro_bank_english_interest_rate` | `src (present)` | `economic/macro_bank.py:160` | DONE |  |
| `macro_bank_euro_interest_rate` | `src (present)` | `economic/macro_bank.py:112` | DONE |  |
| `macro_bank_india_interest_rate` | `src (present)` | `economic/macro_bank.py:208` | DONE |  |
| `macro_bank_japan_interest_rate` | `src (present)` | `economic/macro_bank.py:184` | DONE |  |
| `macro_bank_newzealand_interest_rate` | `src (present)` | `economic/macro_bank.py:124` | DONE |  |
| `macro_bank_russia_interest_rate` | `src (present)` | `economic/macro_bank.py:196` | DONE |  |
| `macro_bank_switzerland_interest_rate` | `src (present)` | `economic/macro_bank.py:148` | DONE |  |
| `macro_bank_usa_interest_rate` | `src (present)` | `economic/macro_bank.py:101` | DONE |  |
| `macro_canada_bank_rate` | `economic/macro_intl.rs::macro_canada_bank_rate` | `economic/macro_canada.py:218` | DONE |  |
| `macro_canada_core_cpi_monthly` | `economic/macro_intl.rs::macro_canada_core_cpi_monthly` | `economic/macro_canada.py:320` | DONE |  |
| `macro_canada_core_cpi_yearly` | `economic/macro_intl.rs::macro_canada_core_cpi_yearly` | `economic/macro_canada.py:269` | DONE |  |
| `macro_canada_cpi_monthly` | `economic/macro_intl.rs::macro_canada_cpi_monthly` | `economic/macro_canada.py:422` | DONE |  |
| `macro_canada_cpi_yearly` | `economic/macro_intl.rs::macro_canada_cpi_yearly` | `economic/macro_canada.py:371` | DONE |  |
| `macro_canada_gdp_monthly` | `economic/macro_intl.rs::macro_canada_gdp_monthly` | `economic/macro_canada.py:473` | DONE |  |
| `macro_canada_new_house_rate` | `economic/macro_intl.rs::macro_canada_new_house_rate` | `economic/macro_canada.py:14` | DONE |  |
| `macro_canada_retail_rate_monthly` | `economic/macro_intl.rs::macro_canada_retail_rate_monthly` | `economic/macro_canada.py:167` | DONE |  |
| `macro_canada_trade` | `economic/macro_intl.rs::macro_canada_trade` | `economic/macro_canada.py:116` | DONE |  |
| `macro_canada_unemployment_rate` | `economic/macro_intl.rs::macro_canada_unemployment_rate` | `economic/macro_canada.py:65` | DONE |  |
| `macro_china_agricultural_index` | `src (present)` | `economic/macro_china.py:1490` | DONE |  |
| `macro_china_agricultural_product` | `src (present)` | `economic/macro_china.py:1435` | DONE |  |
| `macro_china_au_report` | `economic/macro_china_more.rs::macro_china_au_report` | `economic/macro_china.py:953` | DONE |  |
| `macro_china_bank_financing` | `src (present)` | `economic/macro_china.py:1241` | DONE |  |
| `macro_china_bdti_index` | `src (present)` | `economic/macro_china.py:1933` | DONE |  |
| `macro_china_bsi_index` | `src (present)` | `economic/macro_china.py:1988` | DONE |  |
| `macro_china_central_bank_balance` | — | `economic/macro_china.py:3526` | DEFERRED | token/JS/HTML-gated |
| `macro_china_commodity_price_index` | `src (present)` | `economic/macro_china.py:1600` | DONE |  |
| `macro_china_construction_index` | `src (present)` | `economic/macro_china.py:1765` | DONE |  |
| `macro_china_construction_price_index` | `src (present)` | `economic/macro_china.py:1823` | DONE |  |
| `macro_china_consumer_goods_retail` | `economic/macro2.rs::macro_china_consumer_goods_retail` | `economic/macro_china.py:3180` | DONE |  |
| `macro_china_cpi` | `economic/china.rs::macro_china_cpi` | `economic/macro_china.py:2425` | DONE |  |
| `macro_china_cpi_monthly` | `src (present)` | `economic/macro_china.py:421` | DONE |  |
| `macro_china_cpi_yearly` | `src (present)` | `economic/macro_china.py:402` | DONE |  |
| `macro_china_cx_pmi_yearly` | `src (present)` | `economic/macro_china.py:563` | DONE |  |
| `macro_china_cx_services_pmi_yearly` | `src (present)` | `economic/macro_china.py:582` | DONE |  |
| `macro_china_czsr` | `economic/macro_china2.rs::macro_china_czsr` | `economic/macro_china.py:2814` | DONE |  |
| `macro_china_daily_energy` | `src/economic/macro_gaps.rs::macro_china_daily_energy` | `economic/macro_china.py:750` | DONE |  |
| `macro_china_energy_index` | `src (present)` | `economic/macro_china.py:1545` | DONE |  |
| `macro_china_enterprise_boom_index` | `economic/extra.rs::macro_china_enterprise_boom_index` | `economic/macro_china.py:1138` | DONE |  |
| `macro_china_exports_yoy` | `src (present)` | `economic/macro_china.py:459` | DONE |  |
| `macro_china_fdi` | `economic/extra.rs::macro_china_fdi` | `economic/macro_china.py:203` | DONE |  |
| `macro_china_foreign_exchange_gold` | — | `economic/macro_china.py:3628` | DEFERRED | token/JS/HTML-gated |
| `macro_china_freight_index` | `src/economic/macro_gaps.rs::macro_china_freight_index` | `economic/macro_china.py:3481` | DONE |  |
| `macro_china_fx_gold` | `economic/macro_china2.rs::macro_china_fx_gold` | `economic/macro_china.py:2190` | DONE |  |
| `macro_china_fx_reserves_yearly` | `src (present)` | `economic/macro_china.py:620` | DONE |  |
| `macro_china_gdp` | `economic/china.rs::macro_china_gdp` | `economic/macro_china.py:2500` | DONE |  |
| `macro_china_gdp_yearly` | `src (present)` | `economic/macro_china.py:383` | DONE |  |
| `macro_china_gdzctz` | `economic/macro2.rs::macro_china_gdzctz` | `economic/macro_china.py:2674` | DONE |  |
| `macro_china_gyzjz` | `economic/macro2.rs::macro_china_gyzjz` | `economic/macro_china.py:3051` | DONE |  |
| `macro_china_hgjck` | `economic/macro_china2.rs::macro_china_hgjck` | `economic/macro_china.py:2723` | DONE |  |
| `macro_china_hk_building_amount` | `economic/macro_intl.rs::macro_china_hk_building_amount` | `economic/macro_china_hk.py:135` | DONE |  |
| `macro_china_hk_building_volume` | `economic/macro_intl.rs::macro_china_hk_building_volume` | `economic/macro_china_hk.py:124` | DONE |  |
| `macro_china_hk_core` | `economic/wv_macro_core.rs::macro_china_hk_core` | `economic/macro_china_hk.py:13` | DONE |  |
| `macro_china_hk_cpi` | `economic/macro_intl.rs::macro_china_hk_cpi` | `economic/macro_china_hk.py:69` | DONE |  |
| `macro_china_hk_cpi_ratio` | `economic/macro_intl.rs::macro_china_hk_cpi_ratio` | `economic/macro_china_hk.py:80` | DONE |  |
| `macro_china_hk_gbp` | `economic/macro_intl.rs::macro_china_hk_gbp` | `economic/macro_china_hk.py:102` | DONE |  |
| `macro_china_hk_gbp_ratio` | `economic/macro_intl.rs::macro_china_hk_gbp_ratio` | `economic/macro_china_hk.py:113` | DONE |  |
| `macro_china_hk_market_info` | `economic/macro_china_more.rs::macro_china_hk_market_info` | `economic/macro_china.py:704` | DONE |  |
| `macro_china_hk_ppi` | `economic/macro_intl.rs::macro_china_hk_ppi` | `economic/macro_china_hk.py:157` | DONE |  |
| `macro_china_hk_rate_of_unemployment` | `economic/macro_intl.rs::macro_china_hk_rate_of_unemployment` | `economic/macro_china_hk.py:91` | DONE |  |
| `macro_china_hk_trade_diff_ratio` | `economic/macro_intl.rs::macro_china_hk_trade_diff_ratio` | `economic/macro_china_hk.py:146` | DONE |  |
| `macro_china_imports_yoy` | `src (present)` | `economic/macro_china.py:480` | DONE |  |
| `macro_china_industrial_production_yoy` | `src (present)` | `economic/macro_china.py:522` | DONE |  |
| `macro_china_insurance` | `src (present)` | `economic/macro_china.py:3560` | DONE |  |
| `macro_china_insurance_income` | `src (present)` | `economic/macro_china.py:1287` | DONE |  |
| `macro_china_international_tourism_fx` | — | `economic/macro_china.py:3381` | DEFERRED | token/JS/HTML-gated |
| `macro_china_lpi_index` | `src (present)` | `economic/macro_china.py:1878` | DONE |  |
| `macro_china_lpr` | `economic/extra.rs::macro_china_lpr` | `economic/macro_china.py:1012` | DONE |  |
| `macro_china_m2_yearly` | `src (present)` | `economic/macro_china.py:639` | DONE |  |
| `macro_china_market_margin_sh` | `economic/macro_china_more.rs::macro_china_market_margin_sh` | `economic/macro_china.py:919` | DONE |  |
| `macro_china_market_margin_sz` | `economic/macro_china_more.rs::macro_china_market_margin_sz` | `economic/macro_china.py:888` | DONE |  |
| `macro_china_mobile_number` | `src (present)` | `economic/macro_china.py:1333` | DONE |  |
| `macro_china_money_supply` | `economic/china.rs::macro_china_money_supply` | `economic/macro_china.py:2342` | DONE |  |
| `macro_china_national_tax_receipts` | `economic/extra.rs::macro_china_national_tax_receipts` | `economic/macro_china.py:1206` | DONE |  |
| `macro_china_nbs_nation` | — | | `economic/macro_china_nbs.py:517` | DEFERRED | needs NBS catalog resolution (dynamic cid/root_id/route from path) |
| `macro_china_nbs_region` | — | | `economic/macro_china_nbs.py:566` | DEFERRED | needs NBS catalog resolution (dynamic cid/root_id/route from path) |
| `macro_china_new_financial_credit` | `economic/macro_china2.rs::macro_china_new_financial_credit` | `economic/macro_china.py:2142` | DONE |  |
| `macro_china_new_house_price` | `economic/extra.rs::macro_china_new_house_price` | `economic/macro_china.py:1059` | DONE |  |
| `macro_china_non_man_pmi` | `src (present)` | `economic/macro_china.py:601` | DONE |  |
| `macro_china_passenger_load_factor` | — | `economic/macro_china.py:3415` | DEFERRED | token/JS/HTML-gated |
| `macro_china_pmi` | `economic/macro2.rs::macro_china_pmi` | `economic/macro_china.py:2622` | DONE |  |
| `macro_china_pmi_yearly` | `src (present)` | `economic/macro_china.py:544` | DONE |  |
| `macro_china_postal_telecommunicational` | — | `economic/macro_china.py:3347` | DEFERRED | token/JS/HTML-gated |
| `macro_china_ppi` | `economic/china.rs::macro_china_ppi` | `economic/macro_china.py:2577` | DONE |  |
| `macro_china_ppi_yearly` | `src (present)` | `economic/macro_china.py:440` | DONE |  |
| `macro_china_qyspjg` | `economic/extra.rs::macro_china_qyspjg` | `economic/macro_china.py:108` | DONE |  |
| `macro_china_real_estate` | `src (present)` | `economic/macro_china.py:3699` | DONE |  |
| `macro_china_reserve_requirement_ratio` | `economic/macro_china2.rs::macro_china_reserve_requirement_ratio` | `economic/macro_china.py:3096` | DONE |  |
| `macro_china_retail_price_index` | — | `economic/macro_china.py:3663` | DEFERRED | token/JS/HTML-gated |
| `macro_china_rmb` | `economic/macro_china_more.rs::macro_china_rmb` | `economic/macro_china.py:780` | DONE |  |
| `macro_china_shibor_all` | `economic/macro_china_more.rs::macro_china_shibor_all` | `economic/macro_china.py:658` | DONE |  |
| `macro_china_shrzgm` | `src/economic/macro_more3.rs::macro_china_shrzgm` | `economic/macro_china.py:258` | DONE |  |
| `macro_china_society_electricity` | — | `economic/macro_china.py:3236` | DEFERRED | token/JS/HTML-gated |
| `macro_china_society_traffic_volume` | — | `economic/macro_china.py:3289` | DEFERRED | token/JS/HTML-gated |
| `macro_china_stock_market_cap` | `economic/macro_china2.rs::macro_china_stock_market_cap` | `economic/macro_china.py:2256` | DONE |  |
| `macro_china_supply_of_money` | — | `economic/macro_china.py:3594` | DEFERRED | token/JS/HTML-gated |
| `macro_china_trade_balance` | `src (present)` | `economic/macro_china.py:502` | DONE |  |
| `macro_china_urban_unemployment` | `src/economic/macro_more3.rs::macro_china_urban_unemployment` | `economic/macro_china.py:318` | DONE |  |
| `macro_china_vegetable_basket` | `src (present)` | `economic/macro_china.py:1380` | DONE |  |
| `macro_china_wbck` | `economic/macro_china2.rs::macro_china_wbck` | `economic/macro_china.py:2917` | DONE |  |
| `macro_china_whxd` | `economic/macro_china2.rs::macro_china_whxd` | `economic/macro_china.py:2867` | DONE |  |
| `macro_china_xfzxx` | `economic/macro_china2.rs::macro_china_xfzxx` | `economic/macro_china.py:2966` | DONE |  |
| `macro_china_yw_electronic_index` | `src (present)` | `economic/macro_china.py:1710` | DONE |  |
| `macro_cnbs` | `src/economic/excel_gaps.rs::macro_cnbs` | `economic/marco_cnbs.py:12` | DONE |  |
| `macro_cons_gold` | `src/economic/macro_more3.rs::macro_cons_gold` | `economic/macro_constitute.py:17` | DONE |  |
| `macro_cons_opec_month` | `src/economic/macro_more3.rs::macro_cons_opec_month` | `economic/macro_constitute.py:147` | DONE |  |
| `macro_cons_silver` | `src/economic/macro_more3.rs::macro_cons_silver` | `economic/macro_constitute.py:82` | DONE |  |
| `macro_euro_cpi_mom` | `src (present)` | `economic/macro_euro.py:81` | DONE |  |
| `macro_euro_cpi_yoy` | `src (present)` | `economic/macro_euro.py:137` | DONE |  |
| `macro_euro_current_account_mom` | `src (present)` | `economic/macro_euro.py:487` | DONE |  |
| `macro_euro_employment_change_qoq` | `src (present)` | `economic/macro_euro.py:313` | DONE |  |
| `macro_euro_gdp_yoy` | `src (present)` | `economic/macro_euro.py:24` | DONE |  |
| `macro_euro_industrial_production_mom` | `src (present)` | `economic/macro_euro.py:546` | DONE |  |
| `macro_euro_lme_holding` | `src/economic/macro_gaps.rs::macro_euro_lme_holding` | `economic/macro_euro.py:839` | DONE |  |
| `macro_euro_lme_stock` | `src/economic/macro_gaps.rs::macro_euro_lme_stock` | `economic/macro_euro.py:870` | DONE |  |
| `macro_euro_manufacturing_pmi` | `src (present)` | `economic/macro_euro.py:605` | DONE |  |
| `macro_euro_ppi_mom` | `src (present)` | `economic/macro_euro.py:196` | DONE |  |
| `macro_euro_retail_sales_mom` | `src (present)` | `economic/macro_euro.py:254` | DONE |  |
| `macro_euro_sentix_investor_confidence` | `src (present)` | `economic/macro_euro.py:781` | DONE |  |
| `macro_euro_services_pmi` | `src (present)` | `economic/macro_euro.py:664` | DONE |  |
| `macro_euro_trade_balance` | `src (present)` | `economic/macro_euro.py:428` | DONE |  |
| `macro_euro_unemployment_rate_mom` | `src (present)` | `economic/macro_euro.py:369` | DONE |  |
| `macro_euro_zew_economic_sentiment` | `src (present)` | `economic/macro_euro.py:723` | DONE |  |
| `macro_fx_sentiment` | `economic/macro_misc.rs::macro_fx_sentiment` | `economic/macro_other.py:53` | DONE |  |
| `macro_germany_core` | `economic/wv_macro_core.rs::macro_germany_core` | `economic/macro_germany.py:12` | DONE |  |
| `macro_germany_cpi_monthly` | `economic/macro_intl.rs::macro_germany_cpi_monthly` | `economic/macro_germany.py:81` | DONE |  |
| `macro_germany_cpi_yearly` | `economic/macro_intl.rs::macro_germany_cpi_yearly` | `economic/macro_germany.py:93` | DONE |  |
| `macro_germany_gdp` | `economic/macro_intl.rs::macro_germany_gdp` | `economic/macro_germany.py:117` | DONE |  |
| `macro_germany_ifo` | `economic/macro_intl.rs::macro_germany_ifo` | `economic/macro_germany.py:69` | DONE |  |
| `macro_germany_retail_sale_monthly` | `economic/macro_intl.rs::macro_germany_retail_sale_monthly` | `economic/macro_germany.py:129` | DONE |  |
| `macro_germany_retail_sale_yearly` | `economic/macro_intl.rs::macro_germany_retail_sale_yearly` | `economic/macro_germany.py:141` | DONE |  |
| `macro_germany_trade_adjusted` | `economic/macro_intl.rs::macro_germany_trade_adjusted` | `economic/macro_germany.py:105` | DONE |  |
| `macro_germany_zew` | `economic/macro_intl.rs::macro_germany_zew` | `economic/macro_germany.py:153` | DONE |  |
| `macro_global_sox_index` | `src (present)` | `economic/macro_china.py:1655` | DONE |  |
| `macro_info_ws` | `economic/macro_misc.rs::macro_info_ws` | `economic/macro_info_ws.py:38` | DONE |  |
| `macro_japan_bank_rate` | `economic/macro_intl.rs::macro_japan_bank_rate` | `economic/macro_japan.py:70` | DONE |  |
| `macro_japan_core` | `economic/wv_macro_core.rs::macro_japan_core` | `economic/macro_japan.py:13` | DONE |  |
| `macro_japan_core_cpi_yearly` | `economic/macro_intl.rs::macro_japan_core_cpi_yearly` | `economic/macro_japan.py:94` | DONE |  |
| `macro_japan_cpi_yearly` | `economic/macro_intl.rs::macro_japan_cpi_yearly` | `economic/macro_japan.py:82` | DONE |  |
| `macro_japan_head_indicator` | `economic/macro_intl.rs::macro_japan_head_indicator` | `economic/macro_japan.py:118` | DONE |  |
| `macro_japan_unemployment_rate` | `economic/macro_intl.rs::macro_japan_unemployment_rate` | `economic/macro_japan.py:106` | DONE |  |
| `macro_rmb_deposit` | — | `economic/macro_finance_ths.py:82` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `macro_rmb_loan` | — | `economic/macro_finance_ths.py:50` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `macro_shipping_bci` | `economic/macro_econ.rs::macro_shipping_bci` | `economic/macro_china.py:2098` | DONE |  |
| `macro_shipping_bcti` | `economic/macro_econ.rs::macro_shipping_bcti` | `economic/macro_china.py:2131` | DONE |  |
| `macro_shipping_bdi` | `economic/macro_econ.rs::macro_shipping_bdi` | `economic/macro_china.py:2109` | DONE |  |
| `macro_shipping_bpi` | `economic/macro_econ.rs::macro_shipping_bpi` | `economic/macro_china.py:2120` | DONE |  |
| `macro_stock_finance` | — | `economic/macro_finance_ths.py:15` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `macro_swiss_core` | `economic/wv_macro_core.rs::macro_swiss_core` | `economic/macro_swiss.py:13` | DONE |  |
| `macro_swiss_cpi_yearly` | `economic/macro_intl.rs::macro_swiss_cpi_yearly` | `economic/macro_swiss.py:94` | DONE |  |
| `macro_swiss_gbd_bank_rate` | `economic/macro_intl.rs::macro_swiss_gbd_bank_rate` | `economic/macro_swiss.py:130` | DONE |  |
| `macro_swiss_gbd_yearly` | `economic/macro_intl.rs::macro_swiss_gbd_yearly` | `economic/macro_swiss.py:118` | DONE |  |
| `macro_swiss_gdp_quarterly` | `economic/macro_intl.rs::macro_swiss_gdp_quarterly` | `economic/macro_swiss.py:106` | DONE |  |
| `macro_swiss_svme` | `economic/macro_intl.rs::macro_swiss_svme` | `economic/macro_swiss.py:70` | DONE |  |
| `macro_swiss_trade` | `economic/macro_intl.rs::macro_swiss_trade` | `economic/macro_swiss.py:82` | DONE |  |
| `macro_uk_bank_rate` | `economic/macro_intl.rs::macro_uk_bank_rate` | `economic/macro_uk.py:106` | DONE |  |
| `macro_uk_core` | `economic/wv_macro_core.rs::macro_uk_core` | `economic/macro_uk.py:13` | DONE |  |
| `macro_uk_core_cpi_monthly` | `economic/macro_intl.rs::macro_uk_core_cpi_monthly` | `economic/macro_uk.py:130` | DONE |  |
| `macro_uk_core_cpi_yearly` | `economic/macro_intl.rs::macro_uk_core_cpi_yearly` | `economic/macro_uk.py:118` | DONE |  |
| `macro_uk_cpi_monthly` | `economic/macro_intl.rs::macro_uk_cpi_monthly` | `economic/macro_uk.py:154` | DONE |  |
| `macro_uk_cpi_yearly` | `economic/macro_intl.rs::macro_uk_cpi_yearly` | `economic/macro_uk.py:142` | DONE |  |
| `macro_uk_gdp_quarterly` | `economic/macro_intl.rs::macro_uk_gdp_quarterly` | `economic/macro_uk.py:214` | DONE |  |
| `macro_uk_gdp_yearly` | `economic/macro_intl.rs::macro_uk_gdp_yearly` | `economic/macro_uk.py:226` | DONE |  |
| `macro_uk_halifax_monthly` | `economic/macro_intl.rs::macro_uk_halifax_monthly` | `economic/macro_uk.py:70` | DONE |  |
| `macro_uk_halifax_yearly` | `economic/macro_intl.rs::macro_uk_halifax_yearly` | `economic/macro_uk.py:82` | DONE |  |
| `macro_uk_retail_monthly` | `economic/macro_intl.rs::macro_uk_retail_monthly` | `economic/macro_uk.py:166` | DONE |  |
| `macro_uk_retail_yearly` | `economic/macro_intl.rs::macro_uk_retail_yearly` | `economic/macro_uk.py:178` | DONE |  |
| `macro_uk_rightmove_monthly` | `economic/macro_intl.rs::macro_uk_rightmove_monthly` | `economic/macro_uk.py:202` | DONE |  |
| `macro_uk_rightmove_yearly` | `economic/macro_intl.rs::macro_uk_rightmove_yearly` | `economic/macro_uk.py:190` | DONE |  |
| `macro_uk_trade` | `economic/macro_intl.rs::macro_uk_trade` | `economic/macro_uk.py:94` | DONE |  |
| `macro_uk_unemployment_rate` | `economic/macro_intl.rs::macro_uk_unemployment_rate` | `economic/macro_uk.py:238` | DONE |  |
| `macro_usa_adp_employment` | `economic/macro_usa.rs::macro_usa_adp_employment` | | `economic/macro_usa.py:374` | DONE | |
| `macro_usa_api_crude_stock` | `economic/macro_usa.rs::macro_usa_api_crude_stock` | | `economic/macro_usa.py:534` | DONE | |
| `macro_usa_building_permits` | `economic/macro_usa.rs::macro_usa_building_permits` | | `economic/macro_usa.py:763` | DONE | |
| `macro_usa_business_inventories` | `economic/macro_usa.rs::macro_usa_business_inventories` | | `economic/macro_usa.py:668` | DONE | |
| `macro_usa_cb_consumer_confidence` | `economic/macro_usa.rs::macro_usa_cb_consumer_confidence` | | `economic/macro_usa.py:862` | DONE | |
| `macro_usa_cftc_c_holding` | `economic/macro_usa.rs::macro_usa_cftc_c_holding` | `economic/macro_usa.py:1026` | DONE |  |
| `macro_usa_cftc_merchant_currency_holding` | `economic/macro_usa.rs::macro_usa_cftc_merchant_currency_holding` | `economic/macro_usa.py:1055` | DONE |  |
| `macro_usa_cftc_merchant_goods_holding` | `economic/macro_usa.rs::macro_usa_cftc_merchant_goods_holding` | `economic/macro_usa.py:1084` | DONE |  |
| `macro_usa_cftc_nc_holding` | `economic/macro_usa.rs::macro_usa_cftc_nc_holding` | `economic/macro_usa.py:997` | DONE |  |
| `macro_usa_cme_merchant_goods_holding` | `economic/macro_usa.rs::macro_usa_cme_merchant_goods_holding` | `economic/macro_usa.py:1113` | DONE |  |
| `macro_usa_core_cpi_monthly` | `economic/macro_usa.rs::macro_usa_core_cpi_monthly` | | `economic/macro_usa.py:205` | DONE | |
| `macro_usa_core_pce_price` | `economic/macro_usa.rs::macro_usa_core_pce_price` | | `economic/macro_usa.py:392` | DONE | |
| `macro_usa_core_ppi` | `economic/macro_usa.rs::macro_usa_core_ppi` | | `economic/macro_usa.py:515` | DONE | |
| `macro_usa_cpi_monthly` | `economic/macro_usa.rs::macro_usa_cpi_monthly` | | `economic/macro_usa.py:186` | DONE | |
| `macro_usa_cpi_yoy` | `economic/macro2.rs::macro_usa_cpi_yoy` | `economic/macro_usa.py:129` | DONE |  |
| `macro_usa_crude_inner` | `economic/macro_usa.rs::macro_usa_crude_inner` | `economic/macro_usa.py:961` | DONE |  |
| `macro_usa_current_account` | `economic/macro_usa.rs::macro_usa_current_account` | | `economic/macro_usa.py:448` | DONE | |
| `macro_usa_durable_goods_orders` | `economic/macro_usa.rs::macro_usa_durable_goods_orders` | | `economic/macro_usa.py:611` | DONE | |
| `macro_usa_eia_crude_rate` | `economic/macro_usa.rs::macro_usa_eia_crude_rate` | | `economic/macro_usa.py:923` | DONE | |
| `macro_usa_exist_home_sales` | `economic/macro_usa.rs::macro_usa_exist_home_sales` | | `economic/macro_usa.py:782` | DONE | |
| `macro_usa_export_price` | `economic/macro_usa.rs::macro_usa_export_price` | | `economic/macro_usa.py:281` | DONE | |
| `macro_usa_factory_orders` | `economic/macro_usa.rs::macro_usa_factory_orders` | | `economic/macro_usa.py:630` | DONE | |
| `macro_usa_gdp_monthly` | `economic/macro_usa.rs::macro_usa_gdp_monthly` | | `economic/macro_usa.py:167` | DONE | |
| `macro_usa_house_price_index` | `economic/macro_usa.rs::macro_usa_house_price_index` | | `economic/macro_usa.py:801` | DONE | |
| `macro_usa_house_starts` | `economic/macro_usa.rs::macro_usa_house_starts` | | `economic/macro_usa.py:725` | DONE | |
| `macro_usa_import_price` | `economic/macro_usa.rs::macro_usa_import_price` | | `economic/macro_usa.py:262` | DONE | |
| `macro_usa_industrial_production` | `economic/macro_usa.rs::macro_usa_industrial_production` | | `economic/macro_usa.py:592` | DONE | |
| `macro_usa_initial_jobless` | `economic/macro_usa.rs::macro_usa_initial_jobless` | | `economic/macro_usa.py:942` | DONE | |
| `macro_usa_ism_non_pmi` | `economic/macro_usa.rs::macro_usa_ism_non_pmi` | | `economic/macro_usa.py:687` | DONE | |
| `macro_usa_ism_pmi` | `economic/macro_usa.rs::macro_usa_ism_pmi` | | `economic/macro_usa.py:573` | DONE | |
| `macro_usa_job_cuts` | `economic/macro_usa.rs::macro_usa_job_cuts` | | `economic/macro_usa.py:338` | DONE | |
| `macro_usa_lmci` | `economic/macro_usa.rs::macro_usa_lmci` | | `economic/macro_usa.py:301` | DONE | |
| `macro_usa_michigan_consumer_sentiment` | `economic/macro_usa.rs::macro_usa_michigan_consumer_sentiment` | | `economic/macro_usa.py:902` | DONE | |
| `macro_usa_nahb_house_market_index` | `economic/macro_usa.rs::macro_usa_nahb_house_market_index` | | `economic/macro_usa.py:706` | DONE | |
| `macro_usa_new_home_sales` | `economic/macro_usa.rs::macro_usa_new_home_sales` | | `economic/macro_usa.py:744` | DONE | |
| `macro_usa_nfib_small_business` | `economic/macro_usa.rs::macro_usa_nfib_small_business` | | `economic/macro_usa.py:881` | DONE | |
| `macro_usa_non_farm` | `economic/macro_usa.rs::macro_usa_non_farm` | | `economic/macro_usa.py:356` | DONE | |
| `macro_usa_pending_home_sales` | `economic/macro_usa.rs::macro_usa_pending_home_sales` | | `economic/macro_usa.py:841` | DONE | |
| `macro_usa_personal_spending` | `economic/macro_usa.rs::macro_usa_personal_spending` | | `economic/macro_usa.py:224` | DONE | |
| `macro_usa_phs` | `economic/macro2.rs::macro_usa_phs` | `economic/macro_usa.py:79` | DONE |  |
| `macro_usa_pmi` | `economic/macro_usa.rs::macro_usa_pmi` | | `economic/macro_usa.py:554` | DONE | |
| `macro_usa_ppi` | `economic/macro_usa.rs::macro_usa_ppi` | | `economic/macro_usa.py:496` | DONE | |
| `macro_usa_real_consumer_spending` | `economic/macro_usa.rs::macro_usa_real_consumer_spending` | | `economic/macro_usa.py:410` | DONE | |
| `macro_usa_retail_sales` | `economic/macro_usa.rs::macro_usa_retail_sales` | | `economic/macro_usa.py:243` | DONE | |
| `macro_usa_rig_count` | `economic/macro_usa.rs::macro_usa_rig_count` | `economic/macro_usa.py:466` | DONE |  |
| `macro_usa_services_pmi` | `economic/macro_usa.rs::macro_usa_services_pmi` | | `economic/macro_usa.py:649` | DONE | |
| `macro_usa_spcs20` | `economic/macro_usa.rs::macro_usa_spcs20` | | `economic/macro_usa.py:820` | DONE | |
| `macro_usa_trade_balance` | `economic/macro_usa.rs::macro_usa_trade_balance` | | `economic/macro_usa.py:430` | DONE | |
| `macro_usa_unemployment_rate` | `economic/macro_usa.rs::macro_usa_unemployment_rate` | | `economic/macro_usa.py:320` | DONE | |

## energy

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `energy_carbon_bj` | — | `energy/energy_carbon.py:76` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `energy_carbon_domestic` | — | `energy/energy_carbon.py:33` | DEFERRED | token/JS/HTML-gated |
| `energy_carbon_eu` | — | `energy/energy_carbon.py:166` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `energy_carbon_gz` | — | `energy/energy_carbon.py:242` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `energy_carbon_hb` | — | `energy/energy_carbon.py:198` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `energy_carbon_sz` | — | `energy/energy_carbon.py:134` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `energy_oil_detail` | `alt/energy.rs::energy_oil_detail` | `energy/energy_oil_em.py:48` | DONE |  |
| `energy_oil_hist` | `alt/energy.rs::energy_oil_hist` | `energy/energy_oil_em.py:13` | DONE |  |

## event

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `migration_area_baidu` | — | | `event/migration.py:16` | DEFERRED | token/JS/HTML-gated (baidu migration) |
| `migration_scale_baidu` | — | | `event/migration.py:56` | DEFERRED | token/JS/HTML-gated (baidu migration) |

## forex

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `forex_hist_em` | `src (present)` | `forex/forex_em.py:77` | DONE |  |
| `forex_spot_em` | `src (present)` | `forex/forex_em.py:16` | DONE |  |

## fortune

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `forbes_rank` | — | `fortune/fortune_forbes_500.py:14` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `fortune_rank` | — | `fortune/fortune_500.py:40` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `hurun_rank` | — | `fortune/fortune_hurun.py:16` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `index_bloomberg_billionaires` | — | `fortune/fortune_bloomberg.py:65` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `index_bloomberg_billionaires_hist` | — | `fortune/fortune_bloomberg.py:14` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `xincaifu_rank` | `fortune/xincaifu.rs::xincaifu_rank` | `fortune/fortune_xincaifu_500.py:15` | DONE |  |

## fund

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `amac_aoin_info` | `fund/amac.rs::amac_aoin_info` | `fund/fund_amac.py:530` | DONE |  |
| `amac_fund_abs` | `fund/wv_fund_misc.rs::amac_fund_abs` | `fund/fund_amac.py:678` | DONE |  |
| `amac_fund_account_info` | `fund/amac.rs::amac_fund_account_info` | `fund/fund_amac.py:629` | DONE |  |
| `amac_fund_info` | `fund/amac.rs::amac_fund_info` | `fund/fund_amac.py:415` | DONE |  |
| `amac_fund_sub_info` | `fund/amac.rs::amac_fund_sub_info` | `fund/fund_amac.py:577` | DONE |  |
| `amac_futures_info` | `fund/amac.rs::amac_futures_info` | `fund/fund_amac.py:737` | DONE |  |
| `amac_manager_cancelled_info` | `fund/amac.rs::amac_manager_cancelled_info` | `fund/fund_amac.py:792` | DONE |  |
| `amac_manager_classify_info` | `fund/amac.rs::amac_manager_classify_info` | `fund/fund_amac.py:294` | DONE |  |
| `amac_manager_info` | `fund/amac.rs::amac_manager_info` | `fund/fund_amac.py:240` | DONE |  |
| `amac_member_info` | `fund/amac.rs::amac_member_info` | `fund/fund_amac.py:44` | DONE |  |
| `amac_member_sub_info` | `fund/amac.rs::amac_member_sub_info` | `fund/fund_amac.py:365` | DONE |  |
| `amac_person_bond_org_list` | `fund/wv_fund_misc.rs::amac_person_bond_org_list` | `fund/fund_amac.py:198` | DONE |  |
| `amac_person_fund_org_list` | `fund/amac.rs::amac_person_fund_org_list` | `fund/fund_amac.py:96` | DONE |  |
| `amac_securities_info` | `fund/amac.rs::amac_securities_info` | `fund/fund_amac.py:476` | DONE |  |
| `fund_announcement_dividend_em` | `fund/more2.rs::fund_announcement_dividend_em` | `fund/fund_announcement_em.py:15` | DONE |  |
| `fund_announcement_personnel_em` | `fund/more2.rs::fund_announcement_personnel_em` | `fund/fund_announcement_em.py:97` | DONE |  |
| `fund_announcement_report_em` | `fund/more2.rs::fund_announcement_report_em` | `fund/fund_announcement_em.py:56` | DONE |  |
| `fund_aum_em` | — | `fund/fund_aum_em.py:14` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `fund_aum_hist_em` | — | `fund/fund_aum_em.py:64` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `fund_aum_trend_em` | `fund/more.rs::fund_aum_trend_em` | `fund/fund_aum_em.py:45` | DONE |  |
| `fund_balance_position_lg` | — | `fund/fund_position_lg.py:51` | DEFERRED | token/JS/HTML-gated |
| `fund_cf_em` | `fund/more2.rs::fund_cf_em` | `fund/fund_fhsp_em.py:104` | DONE |  |
| `fund_etf_category_sina` | `fund/extra.rs::fund_etf_category_sina` | `fund/fund_etf_sina.py:17` | DONE |  |
| `fund_etf_category_ths` | `fund/more2.rs::fund_etf_category_ths` | `fund/fund_etf_ths.py:15` | DONE |  |
| `fund_etf_dividend_sina` | — | | `fund/fund_etf_sina.py:152` | DEFERRED | sina dividend calendar (leave for follow-up wave) |
| `fund_etf_fund_daily_em` | — | `fund/fund_em.py:1064` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `fund_etf_fund_info_em` | `fund/em.rs::fund_etf_fund_info_em` | `fund/fund_em.py:1097` | DONE |  |
| `fund_etf_hist_em` | `fund/etf.rs::fund_etf_hist_em` | `fund/fund_etf_em.py:237` | DONE |  |
| `fund_etf_hist_min_em` | `fund/more2.rs::fund_etf_hist_min_em` | `fund/fund_etf_em.py:320` | DONE |  |
| `fund_etf_hist_sina` | — | `fund/fund_etf_sina.py:116` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `fund_etf_scale_sse` | `fund/more2.rs::fund_etf_scale_sse` | `fund/fund_etf_sse.py:13` | DONE |  |
| `fund_etf_scale_szse` | `src/fund/excel_gaps.rs::fund_etf_scale_szse` | `fund/fund_etf_szse.py:15` | DONE |  |
| `fund_etf_spot_em` | `fund/etf.rs::fund_etf_spot_em` | `fund/fund_etf_em.py:44` | DONE |  |
| `fund_etf_spot_ths` | `fund/more2.rs::fund_etf_spot_ths` | `fund/fund_etf_ths.py:110` | DONE |  |
| `fund_exchange_rank_em` | `fund/wv_fund_misc.rs::fund_exchange_rank_em` | `fund/fund_rank_em.py:151` | DONE |  |
| `fund_fee_em` | — | `fund/fund_fee_em.py:17` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `fund_fh_em` | `fund/more.rs::fund_fh_em` | `fund/fund_fhsp_em.py:15` | DONE |  |
| `fund_fh_rank_em` | `fund/more2.rs::fund_fh_rank_em` | `fund/fund_fhsp_em.py:191` | DONE |  |
| `fund_financial_fund_daily_em` | `fund/em.rs::fund_financial_fund_daily_em` | `fund/fund_em.py:800` | DONE |  |
| `fund_financial_fund_info_em` | `fund/em.rs::fund_financial_fund_info_em` | `fund/fund_em.py:873` | DONE |  |
| `fund_graded_fund_daily_em` | `fund/em.rs::fund_graded_fund_daily_em` | `fund/fund_em.py:938` | DONE |  |
| `fund_graded_fund_info_em` | `fund/em.rs::fund_graded_fund_info_em` | `fund/fund_em.py:1008` | DONE |  |
| `fund_hk_fund_hist_em` | `fund/em.rs::fund_hk_fund_hist_em` | `fund/fund_em.py:1260` | DONE |  |
| `fund_hk_rank_em` | `fund/wv_fund_misc.rs::fund_hk_rank_em` | `fund/fund_rank_em.py:427` | DONE |  |
| `fund_hold_structure_em` | `fund/more.rs::fund_hold_structure_em` | `fund/fund_scale_em.py:71` | DONE |  |
| `fund_individual_achievement_xq` | — | | `fund/fund_xq.py:78` | DEFERRED | session/token gated (xq_a_token) |
| `fund_individual_analysis_xq` | — | | `fund/fund_xq.py:132` | DEFERRED | session/token gated (xq_a_token) |
| `fund_individual_basic_info_xq` | — | | `fund/fund_xq.py:13` | DEFERRED | session/token gated (xq_a_token) |
| `fund_individual_detail_hold_xq` | — | | `fund/fund_xq.py:270` | DEFERRED | session/token gated (xq_a_token) |
| `fund_individual_detail_info_xq` | — | | `fund/fund_xq.py:224` | DEFERRED | session/token gated (xq_a_token) |
| `fund_individual_profit_probability_xq` | — | | `fund/fund_xq.py:185` | DEFERRED | session/token gated (xq_a_token) |
| `fund_info_index_em` | `fund/em.rs::fund_info_index_em` | `fund/fund_em.py:234` | DONE |  |
| `fund_info_ths` | — | `fund/fund_info_ths.py:16` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `fund_lcx_rank_em` | `fund/more2.rs::fund_lcx_rank_em` | `fund/fund_rank_em.py:346` | DONE |  |
| `fund_linghuo_position_lg` | — | `fund/fund_position_lg.py:89` | DEFERRED | token/JS/HTML-gated |
| `fund_lof_hist_em` | `fund/wv_fund_misc.rs::fund_lof_hist_em` | `fund/fund_lof_em.py:120` | DONE |  |
| `fund_lof_hist_min_em` | `fund/more2.rs::fund_lof_hist_min_em` | `fund/fund_lof_em.py:190` | DONE |  |
| `fund_lof_spot_em` | `fund/lof.rs::fund_lof_spot_em` | `fund/fund_lof_em.py:45` | DONE |  |
| `fund_manager_em` | `fund/more.rs::fund_manager_em` | `fund/fund_manager.py:16` | DONE |  |
| `fund_money_fund_daily_em` | — | `fund/fund_em.py:707` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `fund_money_fund_info_em` | `fund/em.rs::fund_money_fund_info_em` | `fund/fund_em.py:741` | DONE |  |
| `fund_money_rank_em` | `fund/more2.rs::fund_money_rank_em` | `fund/fund_rank_em.py:246` | DONE |  |
| `fund_name_em` | `fund/more.rs::fund_name_em` | `fund/fund_em.py:218` | DONE |  |
| `fund_new_found_em` | `fund/more2.rs::fund_new_found_em` | `fund/fund_init_em.py:15` | DONE |  |
| `fund_new_found_ths` | `fund/wv_fund_misc.rs::fund_new_found_ths` | `fund/fund_init_ths.py:15` | DONE |  |
| `fund_open_fund_daily_em` | `fund/em.rs::fund_open_fund_daily_em` | `fund/fund_em.py:386` | DONE |  |
| `fund_open_fund_info_em` | — | `fund/fund_em.py:452` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `fund_open_fund_rank_em` | `fund/wv_fund_misc.rs::fund_open_fund_rank_em` | `fund/fund_rank_em.py:33` | DONE |  |
| `fund_overview_em` | — | `fund/fund_overview_em.py:15` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `fund_portfolio_bond_hold_em` | — | `fund/fund_portfolio_em.py:166` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `fund_portfolio_change_em` | — | `fund/fund_portfolio_em.py:290` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `fund_portfolio_hold_em` | — | `fund/fund_portfolio_em.py:84` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `fund_portfolio_industry_allocation_em` | — | `fund/fund_portfolio_em.py:217` | DEFERRED | token/JS/HTML-gated |
| `fund_purchase_em` | `fund/em.rs::fund_purchase_em` | `fund/fund_em.py:151` | DONE |  |
| `fund_rating_all` | — | `fund/fund_rating.py:14` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `fund_rating_ja` | — | `fund/fund_rating.py:276` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `fund_rating_sh` | — | `fund/fund_rating.py:91` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `fund_rating_zs` | — | `fund/fund_rating.py:189` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `fund_report_asset_allocation_cninfo` | — | `fund/fund_report_cninfo.py:161` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `fund_report_industry_allocation_cninfo` | — | `fund/fund_report_cninfo.py:97` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `fund_report_stock_cninfo` | — | `fund/fund_report_cninfo.py:30` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `fund_scale_change_em` | `fund/more.rs::fund_scale_change_em` | `fund/fund_scale_em.py:15` | DONE |  |
| `fund_scale_close_sina` | `fund/more2.rs::fund_scale_close_sina` | `fund/fund_scale_sina.py:95` | DONE |  |
| `fund_scale_daily_szse` | `src/fund/excel_gaps.rs::fund_scale_daily_szse` | `fund/fund_scale_szse.py:27` | DONE |  |
| `fund_scale_open_sina` | `fund/more2.rs::fund_scale_open_sina` | `fund/fund_scale_sina.py:15` | DONE |  |
| `fund_scale_structured_sina` | `fund/more2.rs::fund_scale_structured_sina` | `fund/fund_scale_sina.py:166` | DONE |  |
| `fund_stock_position_lg` | — | `fund/fund_position_lg.py:15` | DEFERRED | token/JS/HTML-gated |
| `fund_value_estimation_em` | `fund/em.rs::fund_value_estimation_em` | `fund/fund_em.py:1161` | DONE |  |
| `get_data` | — | `fund/fund_amac.py:32` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_market_id` | — | `fund/fund_etf_em.py:220` | INTERNAL | akshare internal helper, not a data endpoint |

## futures

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `chinese_to_english` | — | `futures/symbol_var.py:48` | INTERNAL | akshare internal helper, not a data endpoint |
| `convert_date` | — | | `futures/cons.py:522` | INTERNAL | akshare internal helper (utils/func.py), not a data endpoint |
| `find_chinese` | — | `futures/symbol_var.py:37` | INTERNAL | akshare internal helper, not a data endpoint |
| `futures_comex_inventory` | `futures/extra.rs::futures_comex_inventory` | `futures/futures_comex_em.py:15` | DONE |  |
| `futures_comm_info` | `src (present)` | `futures/futures_comm_qihuo.py:172` | DONE |  |
| `futures_comm_js` | `src (present)` | `futures/futures_comm_js.py:15` | DONE |  |
| `futures_contract_detail` | `src (present)` | `futures/futures_contract_detail.py:16` | DONE |  |
| `futures_contract_detail_em` | `src (present)` | `futures/futures_contract_detail.py:41` | DONE |  |
| `futures_dce_position_rank` | — | `futures/cot.py:818` | DEFERRED | endpoint returns a zip of TSV files; parsing needs a zip/deflate crate absent from Cargo.toml |
| `futures_dce_position_rank_other` | — | `futures/cot.py:1052` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `futures_delivery_czce` | `src (present)` | `futures/futures_to_spot.py:244` | DONE |  |
| `futures_delivery_dce` | `src (present)` | `futures/futures_to_spot.py:57` | DONE |  |
| `futures_delivery_match_czce` | `src (present)` | `futures/futures_to_spot.py:198` | DONE |  |
| `futures_delivery_match_dce` | `src (present)` | `futures/futures_to_spot.py:128` | DONE |  |
| `futures_delivery_shfe` | `futures/exchange_shfe.rs::futures_delivery_shfe` | `futures/futures_to_spot.py:269` | DONE |  |
| `futures_fees_info` | `src (present)` | `futures/futures_comm_ctp.py:17` | DONE |  |
| `futures_foreign_commodity_realtime` | `futures/sina_hq.rs::futures_foreign_commodity_realtime` | `futures/futures_hq_sina.py:103` | DONE |  |
| `futures_foreign_commodity_subscribe_exchange_symbol` | `src (present)` | `futures/futures_hq_sina.py:38` | DONE |  |
| `futures_foreign_detail` | `src (present)` | `futures/futures_foreign.py:45` | DONE |  |
| `futures_foreign_hist` | `src/futures/wv_futures_more.rs::futures_foreign_hist` | `futures/futures_foreign.py:20` | DONE |  |
| `futures_gfex_position_rank` | `futures/cot.rs::futures_gfex_position_rank` | `futures/cot.py:1292` | DONE |  |
| `futures_gfex_warehouse_receipt` | `futures/exchange_gfex.rs::futures_gfex_warehouse_receipt` | `futures/futures_warehouse_receipt.py:159` | DONE |  |
| `futures_global_hist_em` | `futures/global_em_hist.rs::futures_global_hist_em` | `futures/futures_hf_em.py:171` | DONE |  |
| `futures_global_spot_em` | `futures/global_spot_em.rs::futures_global_spot_em` | `futures/futures_hf_em.py:87` | DONE |  |
| `futures_hist_daily_cffex` | `futures/wv_futures_cffex.rs::futures_hist_daily_cffex` | `futures/futures_daily_bar.py:697` | DONE |  |
| `futures_hist_em` | `src/futures/wv_futures_more.rs::futures_hist_em` | `futures/futures_hist_em.py:91` | DONE |  |
| `futures_hist_table_em` | `futures/global_em_hist.rs::futures_hist_table_em` | `futures/futures_hist_em.py:77` | DONE |  |
| `futures_hq_subscribe_exchange_symbol` | `futures/sina_hq.rs::futures_hq_subscribe_exchange_symbol` | `futures/futures_hq_sina.py:58` | DONE |  |
| `futures_index_ccidx` | `futures/wv_futures_index.rs::futures_index_ccidx` | `futures/futures_index_ccidx.py:13` | DONE |  |
| `futures_inventory_99` | `src/futures/fut_gaps.rs::futures_inventory_99` | `futures/futures_inventory_99.py:47` | DONE |  |
| `futures_inventory_em` | `futures/extra.rs::futures_inventory_em` | `futures/futures_inventory_em.py:14` | DONE |  |
| `futures_news_shmet` | `futures/wv_futures_news.rs::futures_news_shmet` | `futures/futures_news_shmet.py:13` | DONE |  |
| `futures_rule` | `src (present)` | `futures/futures_rule.py:15` | DONE |  |
| `futures_rule_em` | `src/futures/wv_futures_more.rs::futures_rule_em` | `futures/futures_rule_em.py:14` | DONE |  |
| `futures_settle` | `futures/wv_futures_settle.rs::futures_settle` | `futures/futures_settle.py:481` | DONE |  |
| `futures_settle_cffex` | `futures/wv_futures_settle.rs::futures_settle_cffex` | `futures/futures_settle.py:175` | DONE |  |
| `futures_settle_czce` | `futures/wv_futures_settle.rs::futures_settle_czce` | `futures/futures_settle.py:227` | DONE |  |
| `futures_settle_gfex` | `futures/wv_futures_settle.rs::futures_settle_gfex` | `futures/futures_settle.py:288` | DONE |  |
| `futures_settle_ine` | `futures/wv_futures_settle.rs::futures_settle_ine` | `futures/futures_settle.py:420` | DONE |  |
| `futures_settle_shfe` | `futures/wv_futures_settle.rs::futures_settle_shfe` | `futures/futures_settle.py:359` | DONE |  |
| `futures_settlement_price_sgx` | `src (present)` | `futures/futures_settlement_price_sgx.py:63` | DONE |  |
| `futures_shfe_warehouse_receipt` | `futures/warehouse_receipt_shfe.rs::futures_shfe_warehouse_receipt` | `futures/futures_warehouse_receipt.py:104` | DONE |  |
| `futures_spot_price` | `src (present)` | `futures/futures_basis.py:79` | DONE |  |
| `futures_spot_price_daily` | `src (present)` | `futures/futures_basis.py:31` | DONE |  |
| `futures_spot_price_previous` | `src (present)` | `futures/futures_basis.py:300` | DONE |  |
| `futures_spot_stock` | `src (present)` | `futures/futures_spot_stock_em.py:15` | DONE |  |
| `futures_stock_shfe_js` | `src (present)` | `futures/futures_stock_js.py:14` | DONE |  |
| `futures_symbol_mark` | `src (present)` | `futures/futures_zh_sina.py:28` | DONE |  |
| `futures_to_spot_czce` | `src (present)` | `futures/futures_to_spot.py:155` | DONE |  |
| `futures_to_spot_dce` | `src (present)` | `futures/futures_to_spot.py:97` | DONE |  |
| `futures_to_spot_shfe` | `futures/exchange_shfe.rs::futures_to_spot_shfe` | `futures/futures_to_spot.py:14` | DONE |  |
| `futures_trading_hours_em` | `futures/wv_futures_rule.rs::futures_trading_hours_em` | `futures/futures_rule_em.py:28` | DONE |  |
| `futures_warehouse_receipt_czce` | `src (present)` | `futures/futures_warehouse_receipt.py:23` | DONE |  |
| `futures_warehouse_receipt_dce` | `futures/exchange_dce.rs::futures_warehouse_receipt_dce` | `futures/futures_warehouse_receipt.py:61` | DONE |  |
| `futures_zh_daily_sina` | `futures/extra.rs::futures_zh_daily_sina` | `futures/futures_zh_sina.py:651` | DONE |  |
| `futures_zh_minute_sina` | `futures/sina.rs::futures_zh_minute_sina` | `futures/futures_zh_sina.py:615` | DONE |  |
| `futures_zh_realtime` | `src (present)` | `futures/futures_zh_sina.py:91` | DONE |  |
| `futures_zh_spot` | `futures/spot.rs::futures_zh_spot` | `futures/futures_zh_sina.py:205` | DONE |  |
| `get_calendar` | — | `futures/cons.py:577` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_cffex_daily` | `src (present)` | `futures/futures_daily_bar.py:108` | DONE |  |
| `get_cffex_rank_table` | — | `futures/cot.py:716` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_czce_daily` | `futures/exchange_czce.rs::get_czce_daily` | `futures/futures_daily_bar.py:341` | DONE |  |
| `get_czce_receipt_1` | — | `futures/receipt.py:269` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_czce_receipt_2` | — | `futures/receipt.py:328` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_czce_receipt_3` | — | `futures/receipt.py:386` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_dce_daily` | `futures/exchange_dce.rs::get_dce_daily` | `futures/futures_daily_bar.py:527` | DONE |  |
| `get_dce_rank_table` | — | `futures/cot.py:566` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_dce_receipt` | — | `futures/receipt.py:37` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_futures_daily` | `src (present)` | `futures/futures_daily_bar.py:637` | DONE |  |
| `get_gfex_daily` | `futures/exchange_gfex.rs::get_gfex_daily` | `futures/futures_daily_bar.py:199` | DONE |  |
| `get_gfex_receipt` | — | `futures/receipt.py:502` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_ine_daily` | `futures/exchange_ine.rs::get_ine_daily` | `futures/futures_daily_bar.py:275` | DONE |  |
| `get_json_path` | — | `futures/cons.py:543` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_latest_data_date` | — | `futures/cons.py:617` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_pk_data` | — | `futures/cons.py:567` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_pk_path` | — | `futures/cons.py:555` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_rank_sum` | — | `futures/cot.py:110` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_rank_sum_daily` | — | `futures/cot.py:56` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_rank_table_czce` | — | `futures/cot.py:408` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_receipt` | `src (present)` | `futures/receipt.py:571` | DONE |  |
| `get_roll_yield` | `src (present)` | `futures/futures_roll_yield.py:23` | DONE |  |
| `get_roll_yield_bar` | `src (present)` | `futures/futures_roll_yield.py:74` | DONE |  |
| `get_shfe_daily` | `futures/exchange_shfe.rs::get_shfe_daily` | `futures/futures_daily_bar.py:453` | DONE |  |
| `get_shfe_rank_table` | `futures/cot.rs::get_shfe_rank_table` | `futures/cot.py:275` | DONE |  |
| `get_shfe_receipt_1` | — | `futures/receipt.py:82` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_shfe_receipt_2` | — | `futures/receipt.py:156` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_shfe_receipt_3` | — | `futures/receipt.py:218` | INTERNAL | akshare internal helper, not a data endpoint |
| `last_trading_day` | — | `futures/cons.py:590` | INTERNAL | akshare internal helper, not a data endpoint |
| `match_main_contract` | `src (present)` | `futures/futures_zh_sina.py:171` | DONE |  |
| `pandas_read_html_link` | — | `futures/requests_fun.py:53` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `requests_link` | — | `futures/requests_fun.py:16` | INTERNAL | akshare internal helper, not a data endpoint |
| `symbol_market` | `src (present)` | `futures/symbol_var.py:25` | DONE |  |
| `symbol_varieties` | — | `futures/symbol_var.py:13` | INTERNAL | akshare internal helper, not a data endpoint |
| `zh_subscribe_exchange_symbol` | — | `futures/futures_zh_sina.py:139` | DEFERRED | token/JS/HTML-gated |

## futures_derivative

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `futures_contract_info_cffex` | — | `futures_derivative/futures_contract_info_cffex.py:15` | DEFERRED | token/JS/HTML-gated |
| `futures_contract_info_czce` | — | `futures_derivative/futures_contract_info_czce.py:15` | DEFERRED | token/JS/HTML-gated |
| `futures_contract_info_dce` | `futures_derivative/contract_info.rs::futures_contract_info_dce` | `futures_derivative/futures_contract_info_dce.py:13` | DONE |  |
| `futures_contract_info_gfex` | `futures_derivative/contract_info.rs::futures_contract_info_gfex` | `futures_derivative/futures_contract_info_gfex.py:13` | DONE |  |
| `futures_contract_info_ine` | `futures_derivative/contract_info.rs::futures_contract_info_ine` | `futures_derivative/futures_contract_info_ine.py:13` | DONE |  |
| `futures_contract_info_shfe` | `futures_derivative/contract_info.rs::futures_contract_info_shfe` | `futures_derivative/futures_contract_info_shfe.py:13` | DONE |  |
| `futures_display_main_sina` | `src/futures/fut_gaps.rs::futures_display_main_sina` | `futures_derivative/futures_index_sina.py:89` | DONE |  |
| `futures_hog_core` | `futures_derivative/hog.rs::futures_hog_core` | `futures_derivative/futures_hog.py:13` | DONE |  |
| `futures_hog_cost` | `futures_derivative/hog.rs::futures_hog_cost` | `futures_derivative/futures_hog.py:57` | DONE |  |
| `futures_hog_supply` | `futures_derivative/hog.rs::futures_hog_supply` | `futures_derivative/futures_hog.py:116` | DONE |  |
| `futures_hold_pos_sina` | — | `futures_derivative/futures_cot_sina.py:15` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `futures_main_sina` | `futures_derivative/sina.rs::futures_main_sina` | `futures_derivative/futures_index_sina.py:103` | DONE |  |
| `futures_spot_sys` | — | `futures_derivative/futures_spot_sys.py:36` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |

## fx

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `currency_pair_map` | — | `fx/currency_investing.py:16` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `fx_c_swap_cm` | `currency/api.rs::fx_c_swap_cm` | `fx/fx_c_swap_cm.py:25` | DONE |  |
| `fx_pair_quote` | `alt/fx.rs::fx_pair_quote` | `fx/fx_quote.py:81` | DONE |  |
| `fx_quote_baidu` | `src/alt/wv_fx_more.rs::fx_quote_baidu` | `fx/fx_quote_baidu.py:13` | DONE |  |
| `fx_spot_quote` | `alt/fx.rs::fx_spot_quote` | `fx/fx_quote.py:24` | DONE |  |
| `fx_swap_quote` | `forex/extra.rs::fx_swap_quote` | `fx/fx_quote.py:48` | DONE |  |

## hf

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `hf_sp_500` | `hf/sp500.rs::hf_sp_500` | `hf/hf_sp500.py:14` | DONE |  |

## index

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `drewry_wci_index` | — | `index/index_drewry.py:17` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `get_hk_index_page_count` | — | `index/index_stock_hk.py:37` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_tx_start_year` | — | `index/index_stock_zh.py:319` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_zh_index_page_count` | — | `index/index_stock_zh.py:43` | INTERNAL | akshare internal helper, not a data endpoint |
| `index_ai_cx` | `index/cx.rs::index_ai_cx` | `index/index_cx.py:469` | DONE |  |
| `index_all_cni` | `index/cons.rs::index_all_cni` | `index/index_cni.py:16` | DONE |  |
| `index_analysis_daily_sw` | `index/research_sw.rs::index_analysis_daily_sw` | `index/index_research_sw.py:319` | DONE |  |
| `index_analysis_monthly_sw` | `index/research_sw.rs::index_analysis_monthly_sw` | `index/index_research_sw.py:498` | DONE |  |
| `index_analysis_week_month_sw` | `index/research_sw.rs::index_analysis_week_month_sw` | `index/index_research_sw.py:397` | DONE |  |
| `index_analysis_weekly_sw` | `index/research_sw.rs::index_analysis_weekly_sw` | `index/index_research_sw.py:423` | DONE |  |
| `index_awpr_cx` | `index/cx.rs::index_awpr_cx` | `index/index_cx.py:377` | DONE |  |
| `index_bei_cx` | `index/cx.rs::index_bei_cx` | `index/index_cx.py:501` | DONE |  |
| `index_bi_cx` | `index/cx.rs::index_bi_cx` | `index/index_cx.py:209` | DONE |  |
| `index_cci_cx` | `index/cx.rs::index_cci_cx` | `index/index_cx.py:405` | DONE |  |
| `index_ci_cx` | `index/cx.rs::index_ci_cx` | `index/index_cx.py:293` | DONE |  |
| `index_code_id_map_em` | `src (present)` | `index/index_zh_em.py:17` | DONE |  |
| `index_component_sw` | `index/research_sw.rs::index_component_sw` | `index/index_research_sw.py:139` | DONE |  |
| `index_csindex_all` | `src/index/excel_gaps.rs::index_csindex_all` | `index/index_csindex.py:16` | DONE |  |
| `index_dei_cx` | `index/cx.rs::index_dei_cx` | `index/index_cx.py:97` | DONE |  |
| `index_detail_cni` | `src/index/excel_gaps.rs::index_detail_cni` | `index/index_cni.py:134` | DONE |  |
| `index_detail_hist_adjust_cni` | — | `index/index_cni.py:191` | DEFERRED | Excel parsing (pd.read_excel) |
| `index_detail_hist_cni` | `src/index/excel_gaps.rs::index_detail_hist_cni` | `index/index_cni.py:164` | DONE |  |
| `index_eri` | `index/extra.rs::index_eri` | `index/index_eri.py:13` | DONE |  |
| `index_fi_cx` | `index/cx.rs::index_fi_cx` | `index/index_cx.py:181` | DONE |  |
| `index_global_hist_em` | `index/index_more.rs::index_global_hist_em` | `index/index_global_em.py:95` | DONE |  |
| `index_global_hist_sina` | `index/extra.rs::index_global_hist_sina` | `index/index_global_sina.py:30` | DONE |  |
| `index_global_name_table` | `index/extra.rs::index_global_name_table` | `index/index_global_sina.py:15` | DONE |  |
| `index_global_spot_em` | `index/index_more.rs::index_global_spot_em` | `index/index_global_em.py:15` | DONE |  |
| `index_hist_cni` | `index/cons.rs::index_hist_cni` | `index/index_cni.py:67` | DONE |  |
| `index_hist_fund_sw` | `index/wv_index_fund_sw.rs::index_hist_fund_sw` | `index/index_research_fund_sw.py:61` | DONE |  |
| `index_hist_sw` | `index/research_sw.rs::index_hist_sw` | `index/index_research_sw.py:29` | DONE |  |
| `index_hog_spot_price` | `index/wv_index_misc.rs::index_hog_spot_price` | `index/index_hog.py:13` | DONE |  |
| `index_ii_cx` | `index/cx.rs::index_ii_cx` | `index/index_cx.py:125` | DONE |  |
| `index_inner_quote_sugar_msweet` | `index/extra.rs::index_inner_quote_sugar_msweet` | `index/index_sugar.py:39` | DONE |  |
| `index_kq_fashion` | `index/extra.rs::index_kq_fashion` | `index/index_kq_ss.py:13` | DONE |  |
| `index_kq_fz` | `index/extra.rs::index_kq_fz` | `index/index_kq_fz.py:14` | DONE |  |
| `index_li_cx` | `index/cx.rs::index_li_cx` | `index/index_cx.py:265` | DONE |  |
| `index_min_sw` | `index/research_sw.rs::index_min_sw` | `index/index_research_sw.py:93` | DONE |  |
| `index_neaw_cx` | `index/cx.rs::index_neaw_cx` | `index/index_cx.py:349` | DONE |  |
| `index_neei_cx` | `index/cx.rs::index_neei_cx` | `index/index_cx.py:533` | DONE |  |
| `index_nei_cx` | `index/cx.rs::index_nei_cx` | `index/index_cx.py:237` | DONE |  |
| `index_news_sentiment_scope` | `index/wv_index_misc.rs::index_news_sentiment_scope` | `index/index_zh_a_scope.py:13` | DONE |  |
| `index_option_1000index_min_qvix` | `src (present)` | `index/index_option_qvix.py:331` | DONE |  |
| `index_option_1000index_qvix` | `src (present)` | `index/index_option_qvix.py:308` | DONE |  |
| `index_option_100etf_min_qvix` | `src (present)` | `index/index_option_qvix.py:251` | DONE |  |
| `index_option_100etf_qvix` | `src (present)` | `index/index_option_qvix.py:228` | DONE |  |
| `index_option_300etf_min_qvix` | `src (present)` | `index/index_option_qvix.py:91` | DONE |  |
| `index_option_300etf_qvix` | `src (present)` | `index/index_option_qvix.py:68` | DONE |  |
| `index_option_300index_min_qvix` | `src (present)` | `index/index_option_qvix.py:291` | DONE |  |
| `index_option_300index_qvix` | `src (present)` | `index/index_option_qvix.py:268` | DONE |  |
| `index_option_500etf_min_qvix` | `src (present)` | `index/index_option_qvix.py:131` | DONE |  |
| `index_option_500etf_qvix` | `src (present)` | `index/index_option_qvix.py:108` | DONE |  |
| `index_option_50etf_min_qvix` | `src (present)` | `index/index_option_qvix.py:51` | DONE |  |
| `index_option_50etf_qvix` | `src (present)` | `index/index_option_qvix.py:28` | DONE |  |
| `index_option_50index_min_qvix` | `src (present)` | `index/index_option_qvix.py:371` | DONE |  |
| `index_option_50index_qvix` | `src (present)` | `index/index_option_qvix.py:348` | DONE |  |
| `index_option_cyb_min_qvix` | `src (present)` | `index/index_option_qvix.py:171` | DONE |  |
| `index_option_cyb_qvix` | `src (present)` | `index/index_option_qvix.py:148` | DONE |  |
| `index_option_kcb_min_qvix` | `src (present)` | `index/index_option_qvix.py:211` | DONE |  |
| `index_option_kcb_qvix` | `src (present)` | `index/index_option_qvix.py:188` | DONE |  |
| `index_outer_quote_sugar_msweet` | `index/extra.rs::index_outer_quote_sugar_msweet` | `index/index_sugar.py:84` | DONE |  |
| `index_pmi_com_cx` | `index/cx_pmi.rs::index_pmi_com_cx` | `index/index_cx.py:13` | DONE |  |
| `index_pmi_man_cx` | `index/cx_pmi.rs::index_pmi_man_cx` | `index/index_cx.py:41` | DONE |  |
| `index_pmi_ser_cx` | `index/cx_pmi.rs::index_pmi_ser_cx` | `index/index_cx.py:69` | DONE |  |
| `index_price_cflp` | `index/extra.rs::index_price_cflp` | `index/index_cflp.py:13` | DONE |  |
| `index_qli_cx` | `index/cx.rs::index_qli_cx` | `index/index_cx.py:437` | DONE |  |
| `index_realtime_fund_sw` | `index/wv_index_fund_sw.rs::index_realtime_fund_sw` | `index/index_research_fund_sw.py:15` | DONE |  |
| `index_realtime_sw` | `index/research_sw.rs::index_realtime_sw` | `index/index_research_sw.py:241` | DONE |  |
| `index_si_cx` | `index/cx.rs::index_si_cx` | `index/index_cx.py:153` | DONE |  |
| `index_stock_cons` | `stock/index/extra.rs::index_stock_cons` | `index/index_cons.py:87` | DONE |  |
| `index_stock_cons_csindex` | `src/index/excel_gaps.rs::index_stock_cons_csindex` | `index/index_cons.py:126` | DONE |  |
| `index_stock_cons_sina` | `index/cons.rs::index_stock_cons_sina` | `index/index_cons.py:20` | DONE |  |
| `index_stock_cons_weight_csindex` | `src/index/excel_gaps.rs::index_stock_cons_weight_csindex` | `index/index_cons.py:160` | DONE |  |
| `index_stock_info` | — | `index/index_cons.py:70` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `index_sugar_msweet` | `index/extra.rs::index_sugar_msweet` | `index/index_sugar.py:13` | DONE |  |
| `index_ti_cx` | `index/cx.rs::index_ti_cx` | `index/index_cx.py:321` | DONE |  |
| `index_us_stock_sina` | — | `index/index_stock_us_sina.py:18` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `index_volume_cflp` | `index/extra.rs::index_volume_cflp` | `index/index_cflp.py:63` | DONE |  |
| `index_yw` | `index/extra.rs::index_yw` | `index/index_yw.py:18` | DONE |  |
| `index_zh_a_hist` | `index/index_more.rs::index_zh_a_hist` | `index/index_zh_em.py:42` | DONE |  |
| `index_zh_a_hist_min_em` | `index/index_more.rs::index_zh_a_hist_min_em` | `index/index_zh_em.py:178` | DONE |  |
| `spot_goods` | `index/wv_index_misc.rs::spot_goods` | `index/index_spot.py:13` | DONE |  |
| `stock_a_code_to_symbol` | `index/cons.rs::stock_a_code_to_symbol` | `index/index_cons.py:196` | DONE |  |
| `stock_hk_index_daily_em` | `index/stock_hk_us_zh.rs::stock_hk_index_daily_em` | `index/index_stock_hk.py:235` | DONE |  |
| `stock_hk_index_daily_sina` | — | `index/index_stock_hk.py:121` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_hk_index_spot_em` | `index/stock_hk_us_zh.rs::stock_hk_index_spot_em` | `index/index_stock_hk.py:148` | DONE |  |
| `stock_hk_index_spot_sina` | `index/stock_hk_us_zh.rs::stock_hk_index_spot_sina` | `index/index_stock_hk.py:54` | DONE |  |
| `stock_zh_index_daily` | `stock/index/extra.rs::stock_zh_index_daily` | `index/index_stock_zh.py:293` | DONE |  |
| `stock_zh_index_daily_em` | `index/stock_hk_us_zh.rs::stock_zh_index_daily_em` | `index/index_stock_zh.py:428` | DONE |  |
| `stock_zh_index_daily_tx` | `index/stock_hk_us_zh.rs::stock_zh_index_daily_tx` | `index/index_stock_zh.py:354` | DONE |  |
| `stock_zh_index_hist_csindex` | `stock/index/more.rs::stock_zh_index_hist_csindex` | `index/index_stock_zh_csindex.py:13` | DONE |  |
| `stock_zh_index_spot_em` | `src (present)` | `index/index_stock_zh.py:208` | DONE |  |
| `stock_zh_index_spot_sina` | `src (present)` | `index/index_stock_zh.py:58` | DONE |  |
| `stock_zh_index_value_csindex` | `src/index/excel_gaps.rs::stock_zh_index_value_csindex` | `index/index_stock_zh_csindex.py:72` | DONE |  |
| `sw_index_first_info` | — | `index/index_sw.py:38` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `sw_index_second_info` | — | `index/index_sw.py:96` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `sw_index_third_cons` | — | `index/index_sw.py:220` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `sw_index_third_info` | — | `index/index_sw.py:158` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |

## interest_rate

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `rate_interbank` | `rate/eastmoney.rs::rate_interbank` | `interest_rate/interbank_rate_em.py:14` | DONE |  |

## movie

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `business_value_artist` | — | `movie/artist_yien.py:65` | DEFERRED | response encrypted; akshare decrypt() runs jm.js via py_mini_racer (JS engine, ADR-0005) |
| `decrypt` | — | `movie/artist_yien.py:50` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `get_current_week` | `alt/movie_yien.rs::get_current_week` | `movie/movie_yien.py:50` | DONE |  |
| `movie_boxoffice_cinema_daily` | `alt/movie_yien.rs::movie_boxoffice_cinema_daily` | `movie/movie_yien.py:581` | DONE |  |
| `movie_boxoffice_cinema_weekly` | — | `movie/movie_yien.py:642` | DEFERRED | akshare raises week-permission error (endpoint gated) |
| `movie_boxoffice_daily` | `alt/movie.rs::movie_boxoffice_daily` | `movie/movie_yien.py:263` | DONE |  |
| `movie_boxoffice_monthly` | `alt/movie.rs::movie_boxoffice_monthly` | `movie/movie_yien.py:353` | DONE |  |
| `movie_boxoffice_realtime` | `alt/movie.rs::movie_boxoffice_realtime` | `movie/movie_yien.py:207` | DONE |  |
| `movie_boxoffice_weekly` | — | `movie/movie_yien.py:340` | DEFERRED | akshare raises week-permission error (endpoint gated) |
| `movie_boxoffice_yearly` | `alt/movie.rs::movie_boxoffice_yearly` | `movie/movie_yien.py:437` | DONE |  |
| `movie_boxoffice_yearly_first_week` | `alt/movie_yien.rs::movie_boxoffice_yearly_first_week` | `movie/movie_yien.py:502` | DONE |  |
| `online_value_artist` | — | `movie/artist_yien.py:103` | DEFERRED | response encrypted; akshare decrypt() runs jm.js via py_mini_racer (JS engine, ADR-0005) |
| `video_tv` | — | `movie/video_yien.py:65` | DEFERRED | response encrypted; akshare decrypt() runs jm.js via py_mini_racer (JS engine, ADR-0005) |
| `video_variety_show` | — | `movie/video_yien.py:96` | DEFERRED | response encrypted; akshare decrypt() runs jm.js via py_mini_racer (JS engine, ADR-0005) |

## news

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `news_cctv` | — | `news/news_cctv.py:17` | DEFERRED | token/JS/HTML-gated |
| `news_economic_baidu` | `news/baidu_calendar.rs::news_economic_baidu` | `news/news_baidu.py:265` | DONE |  |
| `news_report_time_baidu` | `news/baidu_calendar.rs::news_report_time_baidu` | `news/news_baidu.py:434` | DONE |  |
| `news_trade_notify_dividend_baidu` | `news/baidu_calendar.rs::news_trade_notify_dividend_baidu` | `news/news_baidu.py:355` | DONE |  |
| `news_trade_notify_suspend_baidu` | `news/baidu_calendar.rs::news_trade_notify_suspend_baidu` | `news/news_baidu.py:281` | DONE |  |
| `stock_news_em` | `news/stock_news.rs::stock_news_em` | `news/news_stock.py:15` | DONE |  |

## nlp

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `nlp_answer` | `news/nlp_ownthink.rs::nlp_answer` | `nlp/nlp_interface.py:43` | DONE |  |
| `nlp_ownthink` | `news/nlp_ownthink.rs::nlp_ownthink` | `nlp/nlp_interface.py:14` | DONE |  |

## option

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `option_cffex_hs300_daily_sina` | `src/option/wv_option_more.rs::option_cffex_hs300_daily_sina` | `option/option_finance_sina.py:337` | DONE |  |
| `option_cffex_hs300_list_sina` | — | `option/option_finance_sina.py:45` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `option_cffex_hs300_spot_sina` | `src/option/wv_option_more.rs::option_cffex_hs300_spot_sina` | `option/option_finance_sina.py:150` | DONE |  |
| `option_cffex_sz50_daily_sina` | `src/option/wv_option_more.rs::option_cffex_sz50_daily_sina` | `option/option_finance_sina.py:296` | DONE |  |
| `option_cffex_sz50_list_sina` | — | `option/option_finance_sina.py:28` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `option_cffex_sz50_spot_sina` | `src/option/wv_option_more.rs::option_cffex_sz50_spot_sina` | `option/option_finance_sina.py:77` | DONE |  |
| `option_cffex_zz1000_daily_sina` | `src/option/wv_option_more.rs::option_cffex_zz1000_daily_sina` | `option/option_finance_sina.py:378` | DONE |  |
| `option_cffex_zz1000_list_sina` | — | `option/option_finance_sina.py:61` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `option_cffex_zz1000_spot_sina` | `src/option/wv_option_more.rs::option_cffex_zz1000_spot_sina` | `option/option_finance_sina.py:223` | DONE |  |
| `option_comm_info` | — | `option/option_comm_qihuo.py:38` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `option_comm_symbol` | — | `option/option_comm_qihuo.py:18` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `option_commodity_contract_sina` | — | `option/option_commodity_sina.py:16` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `option_commodity_contract_table_sina` | — | `option/option_commodity_sina.py:55` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `option_commodity_hist_sina` | — | `option/option_commodity_sina.py:139` | DEFERRED | token/JS/HTML-gated |
| `option_contract_info_ctp` | `option/wv_option_misc.rs::option_contract_info_ctp` | `option/option_contract_info_ctp.py:13` | DONE |  |
| `option_current_cffex_em` | `option/extra.rs::option_current_cffex_em` | `option/option_em.py:112` | DONE |  |
| `option_current_day_sse` | `option/extra.rs::option_current_day_sse` | `option/option_current_sse.py:13` | DONE |  |
| `option_current_day_szse` | `src/option/excel_gaps.rs::option_current_day_szse` | `option/option_current_szse.py:14` | DONE |  |
| `option_current_em` | `option/extra.rs::option_current_em` | `option/option_em.py:14` | DONE |  |
| `option_daily_stats_sse` | `option/extra.rs::option_daily_stats_sse` | `option/option_daily_stats_sse_szse.py:15` | DONE |  |
| `option_daily_stats_szse` | `option/wv_option_misc.rs::option_daily_stats_szse` | `option/option_daily_stats_sse_szse.py:85` | DONE |  |
| `option_finance_board` | `option/wv_option_misc.rs::option_finance_board` | `option/option_finance.py:72` | DONE |  |
| `option_finance_minute_sina` | `option/sse.rs::option_finance_minute_sina` | `option/option_finance_sina.py:816` | DONE |  |
| `option_finance_sse_underlying` | `option/exchange.rs::option_finance_sse_underlying` | `option/option_finance.py:34` | DONE |  |
| `option_hist_czce` | `option/wv_option_misc.rs::option_hist_czce` | `option/option_commodity.py:187` | DONE |  |
| `option_hist_dce` | `option/commodity.rs::option_hist_dce` | `option/option_commodity.py:32` | DONE |  |
| `option_hist_gfex` | `option/commodity.rs::option_hist_gfex` | `option/option_commodity.py:504` | DONE |  |
| `option_hist_shfe` | `option/commodity.rs::option_hist_shfe` | `option/option_commodity.py:365` | DONE |  |
| `option_hist_yearly_czce` | `option/exchange.rs::option_hist_yearly_czce` | `option/option_czce.py:37` | DONE |  |
| `option_lhb_em` | `option/wv_option_misc.rs::option_lhb_em` | `option/option_lhb_em.py:13` | DONE |  |
| `option_margin` | — | `option/option_margin.py:38` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `option_margin_symbol` | — | `option/option_margin.py:18` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `option_minute_em` | `option/sse.rs::option_minute_em` | `option/option_finance_sina.py:865` | DONE |  |
| `option_premium_analysis_em` | `option/wv_option_misc.rs::option_premium_analysis_em` | `option/option_premium_analysis_em.py:14` | DONE |  |
| `option_risk_analysis_em` | `option/wv_option_misc.rs::option_risk_analysis_em` | `option/option_risk_analysis_em.py:14` | DONE |  |
| `option_risk_indicator_sse` | `option/extra.rs::option_risk_indicator_sse` | `option/option_risk_indicator_sse.py:12` | DONE |  |
| `option_sse_codes_sina` | `option/sse.rs::option_sse_codes_sina` | `option/option_finance_sina.py:477` | DONE |  |
| `option_sse_daily_sina` | `option/sse.rs::option_sse_daily_sina` | `option/option_finance_sina.py:776` | DONE |  |
| `option_sse_expire_day_sina` | `option/sse.rs::option_sse_expire_day_sina` | `option/option_finance_sina.py:441` | DONE |  |
| `option_sse_greeks_sina` | `option/sse.rs::option_sse_greeks_sina` | `option/option_finance_sina.py:686` | DONE |  |
| `option_sse_list_sina` | `option/sse.rs::option_sse_list_sina` | `option/option_finance_sina.py:422` | DONE |  |
| `option_sse_minute_sina` | `option/sse.rs::option_sse_minute_sina` | `option/option_finance_sina.py:732` | DONE |  |
| `option_sse_spot_price_sina` | `option/sse.rs::option_sse_spot_price_sina` | `option/option_finance_sina.py:542` | DONE |  |
| `option_sse_underlying_spot_price_sina` | `option/sse.rs::option_sse_underlying_spot_price_sina` | `option/option_finance_sina.py:621` | DONE |  |
| `option_value_analysis_em` | `option/wv_option_misc.rs::option_value_analysis_em` | `option/option_value_analysis_em.py:14` | DONE |  |
| `option_vol_gfex` | `option/commodity.rs::option_vol_gfex` | `option/option_commodity.py:593` | DONE |  |
| `option_vol_shfe` | `option/commodity.rs::option_vol_shfe` | `option/option_commodity.py:445` | DONE |  |

## other

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `car_market_cate_cpca` | `other/car_cpca.rs::car_market_cate_cpca` | `other/other_car_cpca.py:646` | DONE |  |
| `car_market_country_cpca` | `other/car_cpca.rs::car_market_country_cpca` | `other/other_car_cpca.py:665` | DONE |  |
| `car_market_fuel_cpca` | `other/car_cpca.rs::car_market_fuel_cpca` | `other/other_car_cpca.py:722` | DONE |  |
| `car_market_man_rank_cpca` | `other/car_cpca.rs::car_market_man_rank_cpca` | `other/other_car_cpca.py:391` | DONE |  |
| `car_market_segment_cpca` | `other/car_cpca.rs::car_market_segment_cpca` | `other/other_car_cpca.py:685` | DONE |  |
| `car_market_total_cpca` | `other/car_cpca.rs::car_market_total_cpca` | `other/other_car_cpca.py:13` | DONE |  |
| `car_sale_rank_gasgoo` | `other/wv_other_misc.rs::car_sale_rank_gasgoo` | `other/other_car_gasgoo.py:15` | DONE |  |
| `game_hot_rank_taptap` | `other/wv_other_misc.rs::game_hot_rank_taptap` | `other/other_taptap.py:72` | DONE |  |

## pro

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `pro_api` | — | `pro/data_pro.py:12` | DEFERRED | session/token gated (xq_a_token/hexin-v) |

## qdii

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `qdii_a_index_jsl` | `qdii/jsl.rs::qdii_a_index_jsl` | `qdii/qdii_jsl.py:160` | DONE |  |
| `qdii_e_comm_jsl` | `qdii/e_comm_jsl.rs::qdii_e_comm_jsl` | `qdii/qdii_jsl.py:88` | DONE |  |
| `qdii_e_index_jsl` | `qdii/jsl.rs::qdii_e_index_jsl` | `qdii/qdii_jsl.py:14` | DONE |  |

## qhkc_web

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `get_qhkc_fund_bs` | — | `qhkc_web/qhkc_fund.py:23` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_qhkc_fund_money_change` | — | `qhkc_web/qhkc_fund.py:319` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_qhkc_fund_position` | — | `qhkc_web/qhkc_fund.py:121` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_qhkc_fund_position_change` | — | `qhkc_web/qhkc_fund.py:220` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_qhkc_index` | — | `qhkc_web/qhkc_index.py:21` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_qhkc_index_profit_loss` | — | `qhkc_web/qhkc_index.py:149` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_qhkc_index_trend` | — | `qhkc_web/qhkc_index.py:77` | INTERNAL | akshare internal helper, not a data endpoint |
| `qhkc_tool_foreign` | — | `qhkc_web/qhkc_tool.py:17` | DEFERRED | endpoint https://qhkch.com/ajax/toolbox_foreign.php returns 404; public JSON auth withdrawn (probed 2026-08-15) |
| `qhkc_tool_gdp` | — | `qhkc_web/qhkc_tool.py:111` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `qhkc_tool_nebula` | — | `qhkc_web/qhkc_tool.py:65` | DEFERRED | endpoint returns 404; public JSON auth withdrawn (probed 2026-08-15) |

## rate

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `repo_rate_hist` | `rate/chinamoney.rs::repo_rate_hist` | `rate/repo_rate.py:45` | DONE |  |
| `repo_rate_query` | `rate/chinamoney.rs::repo_rate_query` | `rate/repo_rate.py:12` | DONE |  |

## registry.py

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `interface_info` | — | | `registry.py:215` | INTERNAL | akshare registry introspection, not a market-data endpoint |
| `list_categories` | — | | `registry.py:232` | INTERNAL | akshare registry introspection, not a market-data endpoint |
| `search` | `src (present)` | `registry.py:175` | DONE |  |

## reits

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `reits_hist_em` | `reits/em.rs::reits_hist_em` | `reits/reits_basic.py:116` | DONE |  |
| `reits_hist_min_em` | `reits/wv_reits_misc.rs::reits_hist_min_em` | `reits/reits_basic.py:173` | DONE |  |
| `reits_realtime_em` | `reits/em.rs::reits_realtime_em` | `reits/reits_basic.py:45` | DONE |  |

## request.py

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `make_request_with_retry_json` | — | `request.py:10` | INTERNAL | akshare internal helper, not a data endpoint |
| `make_request_with_retry_text` | — | `request.py:65` | INTERNAL | akshare internal helper, not a data endpoint |

## spot

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `spot_corn_price_soozhu` | — | `spot/spot_hog_soozhu.py:137` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `spot_golden_benchmark_sge` | `spot/sge.rs::spot_golden_benchmark_sge` | `spot/spot_sge.py:163` | DONE |  |
| `spot_hist_sge` | `spot/sge.rs::spot_hist_sge` | `spot/spot_sge.py:109` | DONE |  |
| `spot_hog_crossbred_soozhu` | — | `spot/spot_hog_soozhu.py:113` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `spot_hog_lean_price_soozhu` | — | `spot/spot_hog_soozhu.py:65` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `spot_hog_soozhu` | — | `spot/spot_hog_soozhu.py:14` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `spot_hog_three_way_soozhu` | — | `spot/spot_hog_soozhu.py:89` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `spot_hog_year_trend_soozhu` | — | `spot/spot_hog_soozhu.py:41` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `spot_mixed_feed_soozhu` | — | `spot/spot_hog_soozhu.py:185` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `spot_price_qh` | `src/spot/spot_gaps.rs::spot_price_qh` | `spot/spot_price_qh.py:79` | DONE |  |
| `spot_price_table_qh` | `spot/price_qh.rs::spot_price_table_qh` | `spot/spot_price_qh.py:55` | DONE |  |
| `spot_quotations_sge` | `spot/sge.rs::spot_quotations_sge` | `spot/spot_sge.py:50` | DONE |  |
| `spot_silver_benchmark_sge` | `spot/sge.rs::spot_silver_benchmark_sge` | `spot/spot_sge.py:194` | DONE |  |
| `spot_soybean_price_soozhu` | — | `spot/spot_hog_soozhu.py:161` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `spot_symbol_table_sge` | `spot/sge.rs::spot_symbol_table_sge` | `spot/spot_sge.py:17` | DONE |  |

## stock

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `get_us_stock_name` | — | `stock/stock_us_sina.py:55` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_zh_kcb_page_count` | — | `stock/stock_zh_kcb_sina.py:27` | INTERNAL | akshare internal helper, not a data endpoint |
| `stock_allotment_cninfo` | — | `stock/stock_allotment_cninfo.py:30` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_bid_ask_em` | `stock/bid_ask_em.rs::stock_bid_ask_em` | `stock/stock_ask_bid_em.py:13` | DONE |  |
| `stock_board_concept_cons_em` | `board/concept.rs::stock_board_concept_cons_em` | `stock/stock_board_concept_em.py:382` | DONE |  |
| `stock_board_concept_hist_em` | `stock/board.rs::stock_board_concept_hist_em` | `stock/stock_board_concept_em.py:181` | DONE |  |
| `stock_board_concept_hist_min_em` | `stock/board.rs::stock_board_concept_hist_min_em` | `stock/stock_board_concept_em.py:273` | DONE |  |
| `stock_board_concept_name_em` | `board/concept.rs::stock_board_concept_name_em` | `stock/stock_board_concept_em.py:121` | DONE |  |
| `stock_board_concept_spot_em` | `stock/board.rs::stock_board_concept_spot_em` | `stock/stock_board_concept_em.py:131` | DONE |  |
| `stock_board_industry_cons_em` | `board/industry.rs::stock_board_industry_cons_em` | `stock/stock_board_industry_em.py:461` | DONE |  |
| `stock_board_industry_hist_em` | `stock/board.rs::stock_board_industry_hist_em` | `stock/stock_board_industry_em.py:261` | DONE |  |
| `stock_board_industry_hist_min_em` | `stock/board.rs::stock_board_industry_hist_min_em` | `stock/stock_board_industry_em.py:351` | DONE |  |
| `stock_board_industry_name_em` | `board/industry.rs::stock_board_industry_name_em` | `stock/stock_board_industry_em.py:115` | DONE |  |
| `stock_board_industry_spot_em` | `stock/board.rs::stock_board_industry_spot_em` | `stock/stock_board_industry_em.py:211` | DONE |  |
| `stock_cg_equity_mortgage_cninfo` | — | `stock/stock_cg_equity_mortgage.py:30` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_cg_guarantee_cninfo` | — | `stock/stock_cg_guarantee.py:30` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_cg_lawsuit_cninfo` | — | `stock/stock_cg_lawsuit.py:31` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_concept_fund_flow_hist` | `stock/fund_flow.rs::stock_concept_fund_flow_hist` | `stock/stock_fund_em.py:1136` | DONE |  |
| `stock_dividend_cninfo` | — | `stock/stock_dividend_cninfo.py:30` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_dzjy_hygtj` | `stock/dzjy.rs::stock_dzjy_hygtj` | `stock/stock_dzjy_em.py:295` | DONE |  |
| `stock_dzjy_hyyybtj` | `stock/dzjy.rs::stock_dzjy_hyyybtj` | `stock/stock_dzjy_em.py:402` | DONE |  |
| `stock_dzjy_mrmx` | `stock/dzjy.rs::stock_dzjy_mrmx` | `stock/stock_dzjy_em.py:72` | DONE |  |
| `stock_dzjy_mrtj` | `stock/dzjy.rs::stock_dzjy_mrtj` | `stock/stock_dzjy_em.py:213` | DONE |  |
| `stock_dzjy_sctj` | `stock/dzjy.rs::stock_dzjy_sctj` | `stock/stock_dzjy_em.py:13` | DONE |  |
| `stock_dzjy_yybph` | `stock/dzjy.rs::stock_dzjy_yybph` | `stock/stock_dzjy_em.py:484` | DONE |  |
| `stock_gsrl_gsdt_em` | `stock/more2.rs::stock_gsrl_gsdt_em` | `stock/stock_gsrl_em.py:13` | DONE |  |
| `stock_hk_company_profile_em` | `stock/hk_profile_em.rs::stock_hk_company_profile_em` | `stock/stock_profile_em.py:79` | DONE |  |
| `stock_hk_daily` | — | `stock/stock_hk_sina.py:109` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_hk_dividend_payout_em` | `stock/hk_profile_em.rs::stock_hk_dividend_payout_em` | `stock/stock_profile_em.py:237` | DONE |  |
| `stock_hk_famous_spot_em` | `src (present)` | `stock/stock_hk_famous.py:13` | DONE |  |
| `stock_hk_fhpx_detail_ths` | — | `stock/stock_hk_fhpx_ths.py:15` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_hk_financial_indicator_em` | `stock/hk_profile_em.rs::stock_hk_financial_indicator_em` | `stock/stock_profile_em.py:153` | DONE |  |
| `stock_hk_growth_comparison_em` | `stock/hk_comparison_em.rs::stock_hk_growth_comparison_em` | `stock/stock_hk_comparison_em.py:13` | DONE |  |
| `stock_hk_hot_rank_detail_em` | `stock/hot_rank.rs::stock_hk_hot_rank_detail_em` | `stock/stock_hk_hot_rank_em.py:60` | DONE |  |
| `stock_hk_hot_rank_detail_realtime_em` | `stock/hot_rank.rs::stock_hk_hot_rank_detail_realtime_em` | `stock/stock_hk_hot_rank_em.py:85` | DONE |  |
| `stock_hk_hot_rank_em` | `stock/hot_rank.rs::stock_hk_hot_rank_em` | `stock/stock_hk_hot_rank_em.py:13` | DONE |  |
| `stock_hk_hot_rank_latest_em` | `stock/hot_rank.rs::stock_hk_hot_rank_latest_em` | `stock/stock_hk_hot_rank_em.py:108` | DONE |  |
| `stock_hk_scale_comparison_em` | `stock/hk_comparison_em.rs::stock_hk_scale_comparison_em` | `stock/stock_hk_comparison_em.py:118` | DONE |  |
| `stock_hk_security_profile_em` | `src (present)` | `stock/stock_profile_em.py:13` | DONE |  |
| `stock_hk_spot` | `src (present)` | `stock/stock_hk_sina.py:22` | DONE |  |
| `stock_hk_valuation_comparison_em` | `stock/hk_comparison_em.rs::stock_hk_valuation_comparison_em` | `stock/stock_hk_comparison_em.py:61` | DONE |  |
| `stock_hold_change_cninfo` | — | `stock/stock_hold_control_cninfo.py:198` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_hold_control_cninfo` | — | `stock/stock_hold_control_cninfo.py:35` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_hold_management_detail_cninfo` | — | `stock/stock_hold_control_cninfo.py:106` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_hold_management_detail_em` | `stock/more2.rs::stock_hold_management_detail_em` | `stock/stock_hold_control_em.py:14` | DONE |  |
| `stock_hold_management_person_em` | `stock/hold_management_em.rs::stock_hold_management_person_em` | `stock/stock_hold_control_em.py:111` | DONE |  |
| `stock_hold_num_cninfo` | — | `stock/stock_hold_num_cninfo.py:30` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_hot_keyword_em` | `stock/hot_rank.rs::stock_hot_keyword_em` | `stock/stock_hot_rank_em.py:127` | DONE |  |
| `stock_hot_rank_detail_em` | `stock/hot_rank.rs::stock_hot_rank_detail_em` | `stock/stock_hot_rank_em.py:67` | DONE |  |
| `stock_hot_rank_detail_realtime_em` | `stock/hot_rank.rs::stock_hot_rank_detail_realtime_em` | `stock/stock_hot_rank_em.py:104` | DONE |  |
| `stock_hot_rank_em` | `stock/hot_rank.rs::stock_hot_rank_em` | `stock/stock_hot_rank_em.py:13` | DONE |  |
| `stock_hot_rank_latest_em` | `stock/hot_rank.rs::stock_hot_rank_latest_em` | `stock/stock_hot_rank_em.py:150` | DONE |  |
| `stock_hot_rank_relate_em` | `stock/hot_rank.rs::stock_hot_rank_relate_em` | `stock/stock_hot_rank_em.py:174` | DONE |  |
| `stock_hot_search_baidu` | `stock/wv_stock_misc1.rs::stock_hot_search_baidu` | `stock/stock_hot_search_baidu.py:15` | DONE |  |
| `stock_hot_up_em` | `stock/hot_rank.rs::stock_hot_up_em` | `stock/stock_hot_up_em.py:13` | DONE |  |
| `stock_hsgt_sh_hk_spot_em` | `stock/more2.rs::stock_hsgt_sh_hk_spot_em` | `stock/stock_hsgt_em.py:76` | DONE |  |
| `stock_individual_fund_flow` | `stock/fund_flow.rs::stock_individual_fund_flow` | `stock/stock_fund_em.py:20` | DONE |  |
| `stock_individual_fund_flow_rank` | `stock/fund_flow.rs::stock_individual_fund_flow_rank` | `stock/stock_fund_em.py:122` | DONE |  |
| `stock_individual_info_em` | `stock/holder.rs::stock_individual_info_em` | `stock/stock_info_em.py:13` | DONE |  |
| `stock_individual_spot_xq` | — | `stock/stock_xq.py:81` | DEFERRED | session/token gated (xq_a_token/hexin-v) |
| `stock_industry_category_cninfo` | — | `stock/stock_industry_cninfo.py:32` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_industry_change_cninfo` | — | `stock/stock_industry_cninfo.py:105` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_industry_clf_hist_sw` | `src/stock/excel_gaps.rs::stock_industry_clf_hist_sw` | `stock/stock_industry_sw.py:17` | DONE |  |
| `stock_industry_pe_ratio_cninfo` | — | `stock/stock_industry_pe_cninfo.py:30` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_info_a_code_name` | `stock/info.rs::stock_info_a_code_name` | `stock/stock_info.py:440` | DONE |  |
| `stock_info_bj_name_code` | `stock/info.rs::stock_info_bj_name_code` | `stock/stock_info.py:185` | DONE |  |
| `stock_info_change_name` | — | `stock/stock_info.py:411` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_info_sh_delist` | `stock/info.rs::stock_info_sh_delist` | `stock/stock_info.py:286` | DONE |  |
| `stock_info_sh_name_code` | `stock/info.rs::stock_info_sh_name_code` | `stock/stock_info.py:122` | DONE |  |
| `stock_info_sz_change_name` | `src/stock/excel_gaps.rs::stock_info_sz_change_name` | `stock/stock_info.py:384` | DONE |  |
| `stock_info_sz_delist` | `stock/info.rs::stock_info_sz_delist` | `stock/stock_info.py:355` | DONE |  |
| `stock_info_sz_name_code` | `stock/info.rs::stock_info_sz_name_code` | `stock/stock_info.py:20` | DONE |  |
| `stock_intraday_em` | `src (present)` | `stock/stock_intraday_em.py:29` | DONE |  |
| `stock_intraday_sina` | `src (present)` | `stock/stock_intraday_sina.py:17` | DONE |  |
| `stock_ipo_summary_cninfo` | — | `stock/stock_ipo_summary_cninfo.py:30` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_js_weibo_nlp_time` | — | | `stock/stock_weibo_nlp.py:20` | DEFERRED | needs JS execution |
| `stock_js_weibo_report` | — | | `stock/stock_weibo_nlp.py:49` | DEFERRED | needs JS execution |
| `stock_main_fund_flow` | `stock/fund_flow.rs::stock_main_fund_flow` | `stock/stock_fund_em.py:1223` | DONE |  |
| `stock_market_fund_flow` | `stock/fund_flow.rs::stock_market_fund_flow` | `stock/stock_fund_em.py:347` | DONE |  |
| `stock_new_gh_cninfo` | — | `stock/stock_new_cninfo.py:30` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_new_ipo_cninfo` | — | `stock/stock_new_cninfo.py:76` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_news_main_cx` | `stock/news_cx.rs::stock_news_main_cx` | `stock/stock_news_cx.py:13` | DONE |  |
| `stock_price_js` | `stock/wv_stock_misc1.rs::stock_price_js` | `stock/stock_us_js.py:13` | DONE |  |
| `stock_profile_cninfo` | — | `stock/stock_profile_cninfo.py:30` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_rank_forecast_cninfo` | — | `stock/stock_rank_forecast.py:30` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_report_fund_hold` | `stock/fundamental/more.rs::stock_report_fund_hold` | `stock/stock_fund_hold.py:13` | DONE |  |
| `stock_report_fund_hold_detail` | `stock/fundamental/more.rs::stock_report_fund_hold_detail` | `stock/stock_fund_hold.py:110` | DONE |  |
| `stock_repurchase_em` | `stock/more2.rs::stock_repurchase_em` | `stock/stock_repurchase_em.py:14` | DONE |  |
| `stock_sector_detail` | `stock/fund_flow.rs::stock_sector_detail` | `stock/stock_industry.py:77` | DONE |  |
| `stock_sector_fund_flow_hist` | `stock/fund_flow.rs::stock_sector_fund_flow_hist` | `stock/stock_fund_em.py:1024` | DONE |  |
| `stock_sector_fund_flow_rank` | `stock/fund_flow.rs::stock_sector_fund_flow_rank` | `stock/stock_fund_em.py:447` | DONE |  |
| `stock_sector_fund_flow_summary` | `stock/fund_flow.rs::stock_sector_fund_flow_summary` | `stock/stock_fund_em.py:738` | DONE |  |
| `stock_sector_spot` | `stock/holder.rs::stock_sector_spot` | `stock/stock_industry.py:19` | DONE |  |
| `stock_share_change_cninfo` | — | `stock/stock_share_changes_cninfo.py:31` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_share_hold_change_bse` | `stock/share_hold_exchange.rs::stock_share_hold_change_bse` | `stock/stock_share_hold.py:196` | DONE |  |
| `stock_share_hold_change_sse` | `stock/share_hold_exchange.rs::stock_share_hold_change_sse` | `stock/stock_share_hold.py:21` | DONE |  |
| `stock_share_hold_change_szse` | `stock/share_hold_exchange.rs::stock_share_hold_change_szse` | `stock/stock_share_hold.py:118` | DONE |  |
| `stock_sse_deal_daily` | `stock/sse_summary.rs::stock_sse_deal_daily` | `stock/stock_summary.py:251` | DONE |  |
| `stock_sse_summary` | `stock/sse_summary.rs::stock_sse_summary` | `stock/stock_summary.py:207` | DONE |  |
| `stock_staq_net_stop` | `stock/staq_net_stop.rs::stock_staq_net_stop` | `stock/stock_stop.py:13` | DONE |  |
| `stock_szse_area_summary` | `src/stock/excel_gaps.rs::stock_szse_area_summary` | `stock/stock_summary.py:53` | DONE |  |
| `stock_szse_sector_summary` | — | `stock/stock_summary.py:110` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_szse_summary` | `src/stock/excel_gaps.rs::stock_szse_summary` | `stock/stock_summary.py:22` | DONE |  |
| `stock_us_daily` | — | `stock/stock_us_sina.py:117` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_us_famous_spot_em` | `stock/us.rs::stock_us_famous_spot_em` | `stock/stock_us_famous.py:13` | DONE |  |
| `stock_us_pink_spot_em` | `stock/us.rs::stock_us_pink_spot_em` | `stock/stock_us_pink.py:15` | DONE |  |
| `stock_us_spot` | `src (present)` | `stock/stock_us_sina.py:86` | DONE |  |
| `stock_zh_a_cdr_daily` | — | `stock/stock_zh_a_sina.py:307` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_zh_a_daily` | `stock/daily_sina.rs::stock_zh_a_daily` | `stock/stock_zh_a_sina.py:127` | DONE |  |
| `stock_zh_a_minute` | `stock/misc.rs::stock_zh_a_minute` | `stock/stock_zh_a_sina.py:344` | DONE |  |
| `stock_zh_a_new` | `stock/misc.rs::stock_zh_a_new` | `stock/stock_zh_a_special.py:290` | DONE |  |
| `stock_zh_a_new_em` | `stock/more2.rs::stock_zh_a_new_em` | `stock/stock_zh_a_special.py:110` | DONE |  |
| `stock_zh_a_spot` | `src (present)` | `stock/stock_zh_a_sina.py:45` | DONE |  |
| `stock_zh_a_spot_tx` | `src (present)` | `stock/stock_zh_a_tx.py:17` | DONE |  |
| `stock_zh_a_st_em` | `stock/more.rs::stock_zh_a_st_em` | `stock/stock_zh_a_special.py:20` | DONE |  |
| `stock_zh_a_stop_em` | `stock/zh_a_stop_em.rs::stock_zh_a_stop_em` | `stock/stock_zh_a_special.py:200` | DONE |  |
| `stock_zh_a_tick_tx_js` | `stock/wv_stock_misc1.rs::stock_zh_a_tick_tx_js` | `stock/stock_zh_a_tick_tx.py:16` | DONE |  |
| `stock_zh_ah_daily` | `stock/wv_stock_misc1.rs::stock_zh_ah_daily` | `stock/stock_zh_ah_tx.py:157` | DONE |  |
| `stock_zh_ah_name` | `stock/wv_stock_misc1.rs::stock_zh_ah_name` | `stock/stock_zh_ah_tx.py:110` | DONE |  |
| `stock_zh_ah_spot` | `stock/wv_stock_misc1.rs::stock_zh_ah_spot` | `stock/stock_zh_ah_tx.py:40` | DONE |  |
| `stock_zh_ah_spot_em` | `stock/more2.rs::stock_zh_ah_spot_em` | `stock/stock_hsgt_em.py:14` | DONE |  |
| `stock_zh_b_daily` | — | `stock/stock_zh_b_sina.py:124` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_zh_b_minute` | `stock/zh_b_minute.rs::stock_zh_b_minute` | `stock/stock_zh_b_sina.py:281` | DONE |  |
| `stock_zh_b_spot` | `src (present)` | `stock/stock_zh_b_sina.py:48` | DONE |  |
| `stock_zh_dupont_comparison_em` | `stock/zh_comparison_em.rs::stock_zh_dupont_comparison_em` | `stock/stock_zh_comparison_em.py:162` | DONE |  |
| `stock_zh_growth_comparison_em` | `stock/zh_comparison_em.rs::stock_zh_growth_comparison_em` | `stock/stock_zh_comparison_em.py:13` | DONE |  |
| `stock_zh_kcb_daily` | `stock/wv_stock_misc2.rs::stock_zh_kcb_daily` | `stock/stock_zh_kcb_sina.py:123` | DONE |  |
| `stock_zh_kcb_report_em` | `stock/more2.rs::stock_zh_kcb_report_em` | `stock/stock_zh_kcb_report.py:39` | DONE |  |
| `stock_zh_kcb_spot` | `stock/wv_stock_misc2.rs::stock_zh_kcb_spot` | `stock/stock_zh_kcb_sina.py:42` | DONE |  |
| `stock_zh_scale_comparison_em` | `stock/more2.rs::stock_zh_scale_comparison_em` | `stock/stock_zh_comparison_em.py:219` | DONE |  |
| `stock_zh_valuation_comparison_em` | `stock/indicator.rs::stock_zh_valuation_comparison_em` | `stock/stock_zh_comparison_em.py:72` | DONE |  |

## stock_feature

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `get_cookie_csrf` | — | `stock_feature/stock_a_indicator.py:20` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_token_lg` | — | `stock_feature/stock_a_indicator.py:40` | INTERNAL | akshare internal helper, not a data endpoint |
| `stock_a_all_pb` | — | `stock_feature/stock_all_pb.py:15` | DEFERRED | token/JS/HTML-gated |
| `stock_a_below_net_asset_statistics` | `stock/more.rs::stock_a_below_net_asset_statistics` | `stock_feature/stock_a_below_net_asset_statistics.py:15` | DONE |  |
| `stock_a_congestion_lg` | — | `stock_feature/stock_congestion_lg.py:15` | DEFERRED | token/JS/HTML-gated |
| `stock_a_gxl_lg` | — | `stock_feature/stock_gxl_lg.py:15` | DEFERRED | token/JS/HTML-gated |
| `stock_a_high_low_statistics` | `stock/more.rs::stock_a_high_low_statistics` | `stock_feature/stock_a_high_low.py:15` | DONE |  |
| `stock_a_ttm_lyr` | — | `stock_feature/stock_ttm_lyr.py:15` | DEFERRED | token/JS/HTML-gated |
| `stock_account_statistics_em` | `stock/more.rs::stock_account_statistics_em` | `stock_feature/stock_account_em.py:14` | DONE |  |
| `stock_analyst_detail_em` | `stock_feature/indicators_a.rs::stock_analyst_detail_em` | `stock_feature/stock_analyst_em.py:105` | DONE |  |
| `stock_analyst_rank_em` | `stock_feature/indicators_a.rs::stock_analyst_rank_em` | `stock_feature/stock_analyst_em.py:15` | DONE |  |
| `stock_balance_sheet_by_report_delisted_em` | `stock/financial_three.rs::stock_balance_sheet_by_report_delisted_em` | `stock_feature/stock_three_report_em.py:474` | DONE |  |
| `stock_balance_sheet_by_report_em` | `stock/fundamental/eastmoney.rs::stock_balance_sheet_by_report_em` | `stock_feature/stock_three_report_em.py:35` | DONE |  |
| `stock_balance_sheet_by_yearly_em` | `stock/financial_three.rs::stock_balance_sheet_by_yearly_em` | `stock_feature/stock_three_report_em.py:84` | DONE |  |
| `stock_bj_a_spot_em` | `stock/stock_hist_em.rs::stock_bj_a_spot_em` | `stock_feature/stock_hist_em.py:340` | DONE |  |
| `stock_board_change_em` | `stock_feature/indicators_a.rs::stock_board_change_em` | `stock_feature/stock_pankou_em.py:83` | DONE |  |
| `stock_board_concept_index_ths` | — | `stock_feature/stock_board_concept_ths.py:124` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_board_concept_info_ths` | — | `stock_feature/stock_board_concept_ths.py:91` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_board_concept_name_ths` | `src/stock_feature/sf_gaps.rs::stock_board_concept_name_ths` | `stock_feature/stock_board_concept_ths.py:71` | DONE |  |
| `stock_board_concept_summary_ths` | — | `stock_feature/stock_board_concept_ths.py:273` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_board_industry_index_ths` | — | `stock_feature/stock_board_industry_ths.py:121` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_board_industry_info_ths` | — | `stock_feature/stock_board_industry_ths.py:88` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_board_industry_name_ths` | `src/stock_feature/sf_gaps.rs::stock_board_industry_name_ths` | `stock_feature/stock_board_industry_ths.py:68` | DONE |  |
| `stock_board_industry_summary_ths` | — | `stock_feature/stock_board_industry_ths.py:331` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_buffett_index_lg` | — | `stock_feature/stock_buffett_index_lg.py:15` | DEFERRED | token/JS/HTML-gated |
| `stock_cash_flow_sheet_by_quarterly_em` | `stock/financial_three.rs::stock_cash_flow_sheet_by_quarterly_em` | `stock_feature/stock_three_report_em.py:393` | DONE |  |
| `stock_cash_flow_sheet_by_report_delisted_em` | `stock/financial_three.rs::stock_cash_flow_sheet_by_report_delisted_em` | `stock_feature/stock_three_report_em.py:540` | DONE |  |
| `stock_cash_flow_sheet_by_report_em` | `stock/fundamental/eastmoney.rs::stock_cash_flow_sheet_by_report_em` | `stock_feature/stock_three_report_em.py:291` | DONE |  |
| `stock_cash_flow_sheet_by_yearly_em` | `stock/financial_three.rs::stock_cash_flow_sheet_by_yearly_em` | `stock_feature/stock_three_report_em.py:342` | DONE |  |
| `stock_changes_em` | `stock_feature/indicators_a.rs::stock_changes_em` | `stock_feature/stock_pankou_em.py:13` | DONE |  |
| `stock_classify_board` | — | `stock_feature/stock_classify_sina.py:17` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_classify_sina` | `src/stock_feature/sf_gaps.rs::stock_classify_sina` | `stock_feature/stock_classify_sina.py:48` | DONE |  |
| `stock_comment_detail_scrd_desire_em` | `stock/esg_comment_hot.rs::stock_comment_detail_scrd_desire_em` | `stock_feature/stock_comment_em.py:226` | DONE |  |
| `stock_comment_detail_scrd_focus_em` | `stock/esg_comment_hot.rs::stock_comment_detail_scrd_focus_em` | `stock_feature/stock_comment_em.py:188` | DONE |  |
| `stock_comment_detail_zhpj_lspf_em` | `stock/esg_comment_hot.rs::stock_comment_detail_zhpj_lspf_em` | `stock_feature/stock_comment_em.py:151` | DONE |  |
| `stock_comment_detail_zlkp_jgcyd_em` | `stock_feature/indicators_a.rs::stock_comment_detail_zlkp_jgcyd_em` | `stock_feature/stock_comment_em.py:120` | DONE |  |
| `stock_comment_em` | `stock/esg_comment_hot.rs::stock_comment_em` | `stock_feature/stock_comment_em.py:19` | DONE |  |
| `stock_concept_cons_futu` | `stock_feature/indicators_a.rs::stock_concept_cons_futu` | `stock_feature/stock_concept_futu.py:103` | DONE |  |
| `stock_cy_a_spot_em` | `stock/stock_hist_em.rs::stock_cy_a_spot_em` | `stock_feature/stock_hist_em.py:561` | DONE |  |
| `stock_cyq_em` | — | | `stock_feature/stock_cyq_em.py:16` | DEFERRED | Eastmoney signed param (hexin-v) |
| `stock_dxsyl_em` | `stock_feature/indicators_a.rs::stock_dxsyl_em` | `stock_feature/stock_dxsyl_em.py:18` | DONE |  |
| `stock_ebs_lg` | — | `stock_feature/stock_ebs_lg.py:15` | DEFERRED | token/JS/HTML-gated |
| `stock_esg_hz_sina` | `stock/esg_comment_hot.rs::stock_esg_hz_sina` | `stock_feature/stock_esg_sina.py:267` | DONE |  |
| `stock_esg_msci_sina` | `stock/esg_comment_hot.rs::stock_esg_msci_sina` | `stock_feature/stock_esg_sina.py:16` | DONE |  |
| `stock_esg_rate_sina` | `stock/esg_comment_hot.rs::stock_esg_rate_sina` | `stock_feature/stock_esg_sina.py:167` | DONE |  |
| `stock_esg_rft_sina` | `stock/esg_comment_hot.rs::stock_esg_rft_sina` | `stock_feature/stock_esg_sina.py:103` | DONE |  |
| `stock_esg_zd_sina` | `stock/esg_comment_hot.rs::stock_esg_zd_sina` | `stock_feature/stock_esg_sina.py:221` | DONE |  |
| `stock_fhps_detail_em` | `stock_feature/wv_sf_misc1.rs::stock_fhps_detail_em` | `stock_feature/stock_fhps_em.py:141` | DONE |  |
| `stock_fhps_detail_ths` | — | `stock_feature/stock_fhps_ths.py:15` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_fhps_em` | `stock_feature/indicators_a.rs::stock_fhps_em` | `stock_feature/stock_fhps_em.py:15` | DONE |  |
| `stock_fund_flow_big_deal` | — | `stock_feature/stock_fund_flow.py:349` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_fund_flow_concept` | — | `stock_feature/stock_fund_flow.py:137` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_fund_flow_individual` | — | `stock_feature/stock_fund_flow.py:41` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_fund_flow_industry` | — | `stock_feature/stock_fund_flow.py:243` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_gddh_em` | `stock/more2.rs::stock_gddh_em` | `stock_feature/stock_gddh_em.py:14` | DONE |  |
| `stock_gdfx_free_holding_analyse_em` | `stock/gdfx.rs::stock_gdfx_free_holding_analyse_em` | `stock_feature/stock_gdfx_em.py:691` | DONE |  |
| `stock_gdfx_free_holding_change_em` | `stock/gdfx.rs::stock_gdfx_free_holding_change_em` | `stock_feature/stock_gdfx_em.py:222` | DONE |  |
| `stock_gdfx_free_holding_detail_em` | `stock/gdfx.rs::stock_gdfx_free_holding_detail_em` | `stock_feature/stock_gdfx_em.py:505` | DONE |  |
| `stock_gdfx_free_holding_statistics_em` | `stock/gdfx.rs::stock_gdfx_free_holding_statistics_em` | `stock_feature/stock_gdfx_em.py:15` | DONE |  |
| `stock_gdfx_free_holding_teamwork_em` | `stock/gdfx.rs::stock_gdfx_free_holding_teamwork_em` | `stock_feature/stock_gdfx_em.py:892` | DONE |  |
| `stock_gdfx_free_top_10_em` | `stock/gdfx.rs::stock_gdfx_free_top_10_em` | `stock_feature/stock_gdfx_em.py:393` | DONE |  |
| `stock_gdfx_holding_analyse_em` | `stock/gdfx.rs::stock_gdfx_holding_analyse_em` | `stock_feature/stock_gdfx_em.py:789` | DONE |  |
| `stock_gdfx_holding_change_em` | `stock/gdfx.rs::stock_gdfx_holding_change_em` | `stock_feature/stock_gdfx_em.py:313` | DONE |  |
| `stock_gdfx_holding_detail_em` | `stock/gdfx.rs::stock_gdfx_holding_detail_em` | `stock_feature/stock_gdfx_em.py:595` | DONE |  |
| `stock_gdfx_holding_statistics_em` | `stock/gdfx.rs::stock_gdfx_holding_statistics_em` | `stock_feature/stock_gdfx_em.py:119` | DONE |  |
| `stock_gdfx_holding_teamwork_em` | `stock/gdfx.rs::stock_gdfx_holding_teamwork_em` | `stock_feature/stock_gdfx_em.py:955` | DONE |  |
| `stock_gdfx_top_10_em` | `stock/gdfx.rs::stock_gdfx_top_10_em` | `stock_feature/stock_gdfx_em.py:452` | DONE |  |
| `stock_ggcg_em` | `stock_feature/wv_sf_misc3.rs::stock_ggcg_em` | `stock_feature/stock_gdzjc_em.py:15` | DONE |  |
| `stock_gpzy_distribute_statistics_bank_em` | `stock/gpzy.rs::stock_gpzy_distribute_statistics_bank_em` | `stock_feature/stock_gpzy_em.py:381` | DONE |  |
| `stock_gpzy_distribute_statistics_company_em` | `stock/gpzy.rs::stock_gpzy_distribute_statistics_company_em` | `stock_feature/stock_gpzy_em.py:312` | DONE |  |
| `stock_gpzy_individual_pledge_ratio_detail_em` | `stock/gpzy.rs::stock_gpzy_individual_pledge_ratio_detail_em` | `stock_feature/stock_gpzy_em.py:308` | DONE |  |
| `stock_gpzy_industry_data_em` | `stock/gpzy.rs::stock_gpzy_industry_data_em` | `stock_feature/stock_gpzy_em.py:450` | DONE |  |
| `stock_gpzy_pledge_ratio_detail_em` | `stock/gpzy.rs::stock_gpzy_pledge_ratio_detail_em` | `stock_feature/stock_gpzy_em.py:304` | DONE |  |
| `stock_gpzy_pledge_ratio_em` | `stock/gpzy.rs::stock_gpzy_pledge_ratio_em` | `stock_feature/stock_gpzy_em.py:88` | DONE |  |
| `stock_gpzy_profile_em` | `stock/gpzy.rs::stock_gpzy_profile_em` | `stock_feature/stock_gpzy_em.py:21` | DONE |  |
| `stock_hk_ggt_components_em` | `src (present)` | `stock_feature/stock_hsgt_em.py:94` | DONE |  |
| `stock_hk_gxl_lg` | — | `stock_feature/stock_gxl_lg.py:54` | DEFERRED | token/JS/HTML-gated |
| `stock_hk_hist` | `stock/cross/hk.rs::stock_hk_hist` | `stock_feature/stock_hist_em.py:1395` | DONE |  |
| `stock_hk_hist_min_em` | `stock/stock_hist_em.rs::stock_hk_hist_min_em` | `stock_feature/stock_hist_em.py:1467` | DONE |  |
| `stock_hk_indicator_eniu` | `src/stock_feature/sf_gaps.rs::stock_hk_indicator_eniu` | `stock_feature/stock_a_indicator.py:54` | DONE |  |
| `stock_hk_main_board_spot_em` | `stock/stock_hist_em.rs::stock_hk_main_board_spot_em` | `stock_feature/stock_hist_em.py:1310` | DONE |  |
| `stock_hk_spot_em` | `stock/cross/hk.rs::stock_hk_spot_em` | `stock_feature/stock_hist_em.py:1225` | DONE |  |
| `stock_hk_valuation_baidu` | `stock_feature/wv_sf_misc2.rs::stock_hk_valuation_baidu` | `stock_feature/stock_hk_valuation_baidu.py:14` | DONE |  |
| `stock_hot_deal_xq` | — | | `stock_feature/stock_hot_xq.py:207` | DEFERRED | session/token gated (xq_a_token) |
| `stock_hot_follow_xq` | — | | `stock_feature/stock_hot_xq.py:81` | DEFERRED | session/token gated (xq_a_token) |
| `stock_hot_tweet_xq` | — | | `stock_feature/stock_hot_xq.py:144` | DEFERRED | session/token gated (xq_a_token) |
| `stock_hsgt_board_rank_em` | `stock/hsgt.rs::stock_hsgt_board_rank_em` | `stock_feature/stock_hsgt_em.py:1190` | DONE |  |
| `stock_hsgt_fund_flow_summary_em` | `stock/hsgt.rs::stock_hsgt_fund_flow_summary_em` | `stock_feature/stock_hsgt_em.py:18` | DONE |  |
| `stock_hsgt_fund_min_em` | `stock/hsgt.rs::stock_hsgt_fund_min_em` | `stock_feature/stock_hsgt_min_em.py:13` | DONE |  |
| `stock_hsgt_hist_em` | `stock/hsgt.rs::stock_hsgt_hist_em` | `stock_feature/stock_hsgt_em.py:1070` | DONE |  |
| `stock_hsgt_hold_stock_em` | `stock/hsgt.rs::stock_hsgt_hold_stock_em` | `stock_feature/stock_hsgt_em.py:171` | DONE |  |
| `stock_hsgt_individual_detail_em` | `stock/hsgt.rs::stock_hsgt_individual_detail_em` | `stock_feature/stock_hsgt_em.py:1527` | DONE |  |
| `stock_hsgt_individual_em` | `stock/hsgt.rs::stock_hsgt_individual_em` | `stock_feature/stock_hsgt_em.py:1512` | DONE |  |
| `stock_hsgt_institution_statistics_em` | `stock/hsgt.rs::stock_hsgt_institution_statistics_em` | `stock_feature/stock_hsgt_em.py:778` | DONE |  |
| `stock_hsgt_stock_statistics_em` | `stock/hsgt.rs::stock_hsgt_stock_statistics_em` | `stock_feature/stock_hsgt_em.py:336` | DONE |  |
| `stock_index_pb_lg` | — | `stock_feature/stock_a_pe_and_pb.py:507` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_index_pe_lg` | — | `stock_feature/stock_a_pe_and_pb.py:398` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_info_cjzc_em` | `stock_feature/wv_sf_misc3.rs::stock_info_cjzc_em` | `stock_feature/stock_info.py:21` | DONE |  |
| `stock_info_global_cls` | — | `stock_feature/stock_info.py:195` | DEFERRED | token/JS/HTML-gated |
| `stock_info_global_em` | `stock_feature/wv_sf_misc3.rs::stock_info_global_em` | `stock_feature/stock_info.py:61` | DONE |  |
| `stock_info_global_futu` | `stock_feature/wv_sf_misc3.rs::stock_info_global_futu` | `stock_feature/stock_info.py:127` | DONE |  |
| `stock_info_global_sina` | `stock_feature/wv_sf_misc3.rs::stock_info_global_sina` | `stock_feature/stock_info.py:96` | DONE |  |
| `stock_info_global_ths` | `stock_feature/wv_sf_misc3.rs::stock_info_global_ths` | `stock_feature/stock_info.py:162` | DONE |  |
| `stock_inner_trade_xq` | — | | `stock_feature/stock_inner_trade_xq.py:72` | DEFERRED | session/token gated (xq_a_token) |
| `stock_ipo_benefit_ths` | — | `stock_feature/stock_board_industry_ths.py:274` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_irm_ans_cninfo` | `stock_feature/wv_sf_misc1.rs::stock_irm_ans_cninfo` | `stock_feature/stock_irm_cninfo.py:140` | DONE |  |
| `stock_irm_cninfo` | `stock_feature/wv_sf_misc1.rs::stock_irm_cninfo` | `stock_feature/stock_irm_cninfo.py:31` | DONE |  |
| `stock_jgdy_detail_em` | `stock/more2.rs::stock_jgdy_detail_em` | `stock_feature/stock_jgdy_em.py:108` | DONE |  |
| `stock_jgdy_tj_em` | `stock_feature/wv_sf_misc1.rs::stock_jgdy_tj_em` | `stock_feature/stock_jgdy_em.py:16` | DONE |  |
| `stock_kc_a_spot_em` | `stock/stock_hist_em.rs::stock_kc_a_spot_em` | `stock_feature/stock_hist_em.py:670` | DONE |  |
| `stock_lh_yyb_capital` | — | `stock_feature/stock_lh_yybpm.py:42` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_lh_yyb_control` | — | `stock_feature/stock_lh_yybpm.py:65` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_lh_yyb_most` | — | `stock_feature/stock_lh_yybpm.py:19` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_lhb_detail_daily_sina` | — | `stock_feature/stock_lhb_sina.py:18` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_lhb_detail_em` | `stock/lhb.rs::stock_lhb_detail_em` | `stock_feature/stock_lhb_em.py:14` | DONE |  |
| `stock_lhb_ggtj_sina` | — | `stock_feature/stock_lhb_sina.py:91` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_lhb_hyyyb_em` | `stock/lhb.rs::stock_lhb_hyyyb_em` | `stock_feature/stock_lhb_em.py:433` | DONE |  |
| `stock_lhb_jgmmtj_em` | `stock/lhb.rs::stock_lhb_jgmmtj_em` | `stock_feature/stock_lhb_em.py:226` | DONE |  |
| `stock_lhb_jgmx_sina` | — | `stock_feature/stock_lhb_sina.py:208` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_lhb_jgstatistic_em` | `stock/lhb.rs::stock_lhb_jgstatistic_em` | `stock_feature/stock_lhb_em.py:335` | DONE |  |
| `stock_lhb_jgzz_sina` | — | `stock_feature/stock_lhb_sina.py:166` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_lhb_stock_detail_date_em` | `stock/lhb.rs::stock_lhb_stock_detail_date_em` | `stock_feature/stock_lhb_em.py:723` | DONE |  |
| `stock_lhb_stock_detail_em` | `stock/lhb.rs::stock_lhb_stock_detail_em` | `stock_feature/stock_lhb_em.py:766` | DONE |  |
| `stock_lhb_stock_statistic_em` | `stock/lhb.rs::stock_lhb_stock_statistic_em` | `stock_feature/stock_lhb_em.py:137` | DONE |  |
| `stock_lhb_traderstatistic_em` | `stock/lhb.rs::stock_lhb_traderstatistic_em` | `stock_feature/stock_lhb_em.py:648` | DONE |  |
| `stock_lhb_yyb_detail_em` | `stock/lhb.rs::stock_lhb_yyb_detail_em` | `stock_feature/stock_lhb_em.py:904` | DONE |  |
| `stock_lhb_yybph_em` | `stock/lhb.rs::stock_lhb_yybph_em` | `stock_feature/stock_lhb_em.py:512` | DONE |  |
| `stock_lhb_yytj_sina` | — | `stock_feature/stock_lhb_sina.py:128` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_lrb_em` | `stock/financial.rs::stock_lrb_em` | `stock_feature/stock_report_em.py:302` | DONE |  |
| `stock_margin_account_info` | `stock_feature/margin_research.rs::stock_margin_account_info` | `stock_feature/stock_margin_em.py:15` | DONE |  |
| `stock_margin_bse` | `stock_feature/wv_sf_misc1.rs::stock_margin_bse` | `stock_feature/stock_margin_bse.py:71` | DONE |  |
| `stock_margin_detail_bse` | `stock_feature/wv_sf_misc1.rs::stock_margin_detail_bse` | `stock_feature/stock_margin_bse.py:129` | DONE |  |
| `stock_margin_detail_sse` | `stock_feature/wv_sf_misc3.rs::stock_margin_detail_sse` | `stock_feature/stock_margin_sse.py:137` | DONE |  |
| `stock_margin_detail_szse` | `src/stock_feature/excel_gaps.rs::stock_margin_detail_szse` | `stock_feature/stock_margin_szse.py:95` | DONE |  |
| `stock_margin_ratio_pa` | `stock_feature/wv_sf_misc3.rs::stock_margin_ratio_pa` | `stock_feature/stock_margin_sse.py:13` | DONE |  |
| `stock_margin_sse` | `src/stock_feature/wv_sf_more.rs::stock_margin_sse` | `stock_feature/stock_margin_sse.py:68` | DONE |  |
| `stock_margin_szse` | `src/stock_feature/wv_sf_more.rs::stock_margin_szse` | `stock_feature/stock_margin_szse.py:47` | DONE |  |
| `stock_margin_underlying_info_bse` | `stock_feature/wv_sf_misc1.rs::stock_margin_underlying_info_bse` | `stock_feature/stock_margin_bse.py:190` | DONE |  |
| `stock_margin_underlying_info_szse` | `src/stock_feature/excel_gaps.rs::stock_margin_underlying_info_szse` | `stock_feature/stock_margin_szse.py:15` | DONE |  |
| `stock_market_activity_legu` | — | `stock_feature/stock_market_legu.py:18` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_market_pb_lg` | — | `stock_feature/stock_a_pe_and_pb.py:461` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_market_pe_lg` | — | `stock_feature/stock_a_pe_and_pb.py:322` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_new_a_spot_em` | `stock/stock_hist_em.rs::stock_new_a_spot_em` | `stock_feature/stock_hist_em.py:448` | DONE |  |
| `stock_pg_em` | `stock_feature/wv_sf_misc1.rs::stock_pg_em` | `stock_feature/stock_zf_pg.py:99` | DONE |  |
| `stock_profit_sheet_by_quarterly_em` | `stock/financial_three.rs::stock_profit_sheet_by_quarterly_em` | `stock_feature/stock_three_report_em.py:240` | DONE |  |
| `stock_profit_sheet_by_report_delisted_em` | `stock/financial_three.rs::stock_profit_sheet_by_report_delisted_em` | `stock_feature/stock_three_report_em.py:507` | DONE |  |
| `stock_profit_sheet_by_report_em` | `stock/fundamental/eastmoney.rs::stock_profit_sheet_by_report_em` | `stock_feature/stock_three_report_em.py:142` | DONE |  |
| `stock_profit_sheet_by_yearly_em` | `stock/financial_three.rs::stock_profit_sheet_by_yearly_em` | `stock_feature/stock_three_report_em.py:191` | DONE |  |
| `stock_qbzf_em` | `stock/more2.rs::stock_qbzf_em` | `stock_feature/stock_zf_pg.py:18` | DONE |  |
| `stock_qsjy_em` | `stock/more2.rs::stock_qsjy_em` | `stock_feature/stock_qsjy_em.py:13` | DONE |  |
| `stock_rank_cxd_ths` | — | `stock_feature/stock_technology_ths.py:111` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_rank_cxfl_ths` | — | `stock_feature/stock_technology_ths.py:309` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_rank_cxg_ths` | — | `stock_feature/stock_technology_ths.py:35` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_rank_cxsl_ths` | — | `stock_feature/stock_technology_ths.py:401` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_rank_ljqd_ths` | — | `stock_feature/stock_technology_ths.py:782` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_rank_ljqs_ths` | — | `stock_feature/stock_technology_ths.py:694` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_rank_lxsz_ths` | — | `stock_feature/stock_technology_ths.py:187` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_rank_lxxd_ths` | — | `stock_feature/stock_technology_ths.py:248` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_rank_xstp_ths` | — | `stock_feature/stock_technology_ths.py:493` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_rank_xxtp_ths` | — | `stock_feature/stock_technology_ths.py:594` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_rank_xzjp_ths` | — | `stock_feature/stock_technology_ths.py:870` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_report_disclosure` | `stock_feature/wv_sf_misc2.rs::stock_report_disclosure` | `stock_feature/stock_yjyg_cninfo.py:13` | DONE |  |
| `stock_research_report_em` | `stock_feature/wv_sf_misc2.rs::stock_research_report_em` | `stock_feature/stock_research_report_em.py:16` | DONE |  |
| `stock_sgt_reference_exchange_rate_sse` | `stock_feature/wv_sf_sgt.rs::stock_sgt_reference_exchange_rate_sse` | `stock_feature/stock_hsgt_exchange_rate.py:76` | DONE |  |
| `stock_sgt_reference_exchange_rate_szse` | `src/stock_feature/excel_gaps.rs::stock_sgt_reference_exchange_rate_szse` | `stock_feature/stock_hsgt_exchange_rate.py:47` | DONE |  |
| `stock_sgt_settlement_exchange_rate_sse` | `stock_feature/wv_sf_sgt.rs::stock_sgt_settlement_exchange_rate_sse` | `stock_feature/stock_hsgt_exchange_rate.py:134` | DONE |  |
| `stock_sgt_settlement_exchange_rate_szse` | `src/stock_feature/excel_gaps.rs::stock_sgt_settlement_exchange_rate_szse` | `stock_feature/stock_hsgt_exchange_rate.py:18` | DONE |  |
| `stock_sh_a_spot_em` | `stock/stock_hist_em.rs::stock_sh_a_spot_em` | `stock_feature/stock_hist_em.py:124` | DONE |  |
| `stock_sns_sseinfo` | — | `stock_feature/stock_sns_sseinfo.py:56` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_sy_em` | `stock/sy.rs::stock_sy_em` | `stock_feature/stock_sy_em.py:294` | DONE |  |
| `stock_sy_hy_em` | `stock/sy.rs::stock_sy_hy_em` | `stock_feature/stock_sy_em.py:386` | DONE |  |
| `stock_sy_jz_em` | `stock/sy.rs::stock_sy_jz_em` | `stock_feature/stock_sy_em.py:193` | DONE |  |
| `stock_sy_profile_em` | `stock/sy.rs::stock_sy_profile_em` | `stock_feature/stock_sy_em.py:19` | DONE |  |
| `stock_sy_yq_em` | `stock/sy.rs::stock_sy_yq_em` | `stock_feature/stock_sy_em.py:84` | DONE |  |
| `stock_sz_a_spot_em` | `stock/stock_hist_em.rs::stock_sz_a_spot_em` | `stock_feature/stock_hist_em.py:232` | DONE |  |
| `stock_tfp_em` | `stock_feature/wv_sf_misc2.rs::stock_tfp_em` | `stock_feature/stock_tfp_em.py:13` | DONE |  |
| `stock_us_hist` | `stock/cross/us.rs::stock_us_hist` | `stock_feature/stock_hist_em.py:1688` | DONE |  |
| `stock_us_hist_min_em` | `stock/stock_hist_em.rs::stock_us_hist_min_em` | `stock_feature/stock_hist_em.py:1758` | DONE |  |
| `stock_us_spot_em` | `stock/cross/us.rs::stock_us_spot_em` | `stock_feature/stock_hist_em.py:1593` | DONE |  |
| `stock_us_valuation_baidu` | `stock_feature/wv_sf_misc1.rs::stock_us_valuation_baidu` | `stock_feature/stock_us_valuation_baidu.py:16` | DONE |  |
| `stock_value_em` | `stock/indicator.rs::stock_value_em` | `stock_feature/stock_value_em.py:14` | DONE |  |
| `stock_xgsglb_em` | `stock_feature/wv_sf_misc3.rs::stock_xgsglb_em` | `stock_feature/stock_dxsyl_em.py:128` | DONE |  |
| `stock_xgsr_ths` | — | `stock_feature/stock_board_industry_ths.py:222` | DEFERRED | needs JS execution (py_mini_racer/execjs) |
| `stock_xjll_em` | `stock/financial.rs::stock_xjll_em` | `stock_feature/stock_report_em.py:438` | DONE |  |
| `stock_yjbb_em` | `stock/margin.rs::stock_yjbb_em` | `stock_feature/stock_yjbb_em.py:16` | DONE |  |
| `stock_yjkb_em` | `stock_feature/wv_sf_misc1.rs::stock_yjkb_em` | `stock_feature/stock_yjyg_em.py:17` | DONE |  |
| `stock_yjyg_em` | `stock_feature/wv_sf_misc1.rs::stock_yjyg_em` | `stock_feature/stock_yjyg_em.py:135` | DONE |  |
| `stock_yysj_em` | `stock/more2.rs::stock_yysj_em` | `stock_feature/stock_yjyg_em.py:223` | DONE |  |
| `stock_yzxdr_em` | `stock_feature/wv_sf_misc2.rs::stock_yzxdr_em` | `stock_feature/stock_yzxdr_em.py:16` | DONE |  |
| `stock_zcfz_bj_em` | `stock/financial.rs::stock_zcfz_bj_em` | `stock_feature/stock_report_em.py:161` | DONE |  |
| `stock_zcfz_em` | `stock/financial.rs::stock_zcfz_em` | `stock_feature/stock_report_em.py:20` | DONE |  |
| `stock_zdhtmx_em` | `stock/more2.rs::stock_zdhtmx_em` | `stock_feature/stock_zdhtmx_em.py:14` | DONE |  |
| `stock_zh_a_disclosure_relation_cninfo` | `stock_feature/wv_sf_misc1.rs::stock_zh_a_disclosure_relation_cninfo` | `stock_feature/stock_disclosure_cninfo.py:205` | DONE |  |
| `stock_zh_a_disclosure_report_cninfo` | `stock_feature/wv_sf_misc1.rs::stock_zh_a_disclosure_report_cninfo` | `stock_feature/stock_disclosure_cninfo.py:129` | DONE |  |
| `stock_zh_a_gdhs` | `stock/extra.rs::stock_zh_a_gdhs` | `stock_feature/stock_gdhs.py:15` | DONE |  |
| `stock_zh_a_gdhs_detail_em` | `stock/more2.rs::stock_zh_a_gdhs_detail_em` | `stock_feature/stock_gdhs.py:130` | DONE |  |
| `stock_zh_a_hist` | `src (present)` | `stock_feature/stock_hist_em.py:952` | DONE |  |
| `stock_zh_a_hist_min_em` | `stock/misc.rs::stock_zh_a_hist_min_em` | `stock_feature/stock_hist_em.py:1042` | DONE |  |
| `stock_zh_a_hist_pre_min_em` | `stock/stock_hist_em.rs::stock_zh_a_hist_pre_min_em` | `stock_feature/stock_hist_em.py:1170` | DONE |  |
| `stock_zh_a_hist_tx` | `src (present)` | `stock_feature/stock_hist_tx.py:40` | DONE |  |
| `stock_zh_a_spot_em` | `stock/stock_hist_em.rs::stock_zh_a_spot_em` | `stock_feature/stock_hist_em.py:15` | DONE |  |
| `stock_zh_ab_comparison_em` | `stock/stock_hist_em.rs::stock_zh_ab_comparison_em` | `stock_feature/stock_hist_em.py:779` | DONE |  |
| `stock_zh_b_spot_em` | `stock/stock_hist_em.rs::stock_zh_b_spot_em` | `stock_feature/stock_hist_em.py:844` | DONE |  |
| `stock_zh_valuation_baidu` | `stock/indicator.rs::stock_zh_valuation_baidu` | `stock_feature/stock_zh_valuation_baidu.py:13` | DONE |  |
| `stock_zh_vote_baidu` | `stock_feature/wv_sf_misc2.rs::stock_zh_vote_baidu` | `stock_feature/stock_zh_vote_baidu.py:13` | DONE |  |
| `stock_zt_pool_dtgc_em` | `stock_feature/board_zt.rs::stock_zt_pool_dtgc_em` | `stock_feature/stock_ztb_em.py:439` | DONE |  |
| `stock_zt_pool_em` | `stock/more.rs::stock_zt_pool_em` | `stock_feature/stock_ztb_em.py:24` | DONE |  |
| `stock_zt_pool_previous_em` | `stock_feature/board_zt.rs::stock_zt_pool_previous_em` | `stock_feature/stock_ztb_em.py:110` | DONE |  |
| `stock_zt_pool_strong_em` | `stock_feature/board_zt.rs::stock_zt_pool_strong_em` | `stock_feature/stock_ztb_em.py:187` | DONE |  |
| `stock_zt_pool_sub_new_em` | `stock_feature/board_zt.rs::stock_zt_pool_sub_new_em` | `stock_feature/stock_ztb_em.py:276` | DONE |  |
| `stock_zt_pool_zbgc_em` | `stock_feature/board_zt.rs::stock_zt_pool_zbgc_em` | `stock_feature/stock_ztb_em.py:357` | DONE |  |

## stock_fundamental

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `stock_add_stock` | — | `stock_fundamental/stock_finance_sina.py:499` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_circulate_stock_holder` | — | `stock_fundamental/stock_finance_sina.py:563` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_financial_abstract` | `stock/fundamental/more.rs::stock_financial_abstract` | `stock_fundamental/stock_finance_sina.py:94` | DONE |  |
| `stock_financial_abstract_new_ths` | `stock/fundamental/finance_more.rs::stock_financial_abstract_new_ths` | `stock_fundamental/stock_finance_ths.py:194` | DONE |  |
| `stock_financial_abstract_ths` | — | `stock_fundamental/stock_finance_ths.py:18` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_financial_analysis_indicator` | `src (present)` | `stock_fundamental/stock_finance_sina.py:228` | DONE |  |
| `stock_financial_analysis_indicator_em` | `stock/fundamental/eastmoney.rs::stock_financial_analysis_indicator_em` | `stock_fundamental/stock_finance_sina.py:181` | DONE |  |
| `stock_financial_benefit_new_ths` | `stock/fundamental/finance_more.rs::stock_financial_benefit_new_ths` | `stock_fundamental/stock_finance_ths.py:380` | DONE |  |
| `stock_financial_benefit_ths` | `stock/fundamental/finance_more.rs::stock_financial_benefit_ths` | `stock_fundamental/stock_finance_ths.py:92` | DONE |  |
| `stock_financial_cash_new_ths` | `stock/fundamental/finance_more.rs::stock_financial_cash_new_ths` | `stock_fundamental/stock_finance_ths.py:477` | DONE |  |
| `stock_financial_cash_ths` | `stock/fundamental/finance_more.rs::stock_financial_cash_ths` | `stock_fundamental/stock_finance_ths.py:130` | DONE |  |
| `stock_financial_debt_new_ths` | `stock/fundamental/finance_more.rs::stock_financial_debt_new_ths` | `stock_fundamental/stock_finance_ths.py:291` | DONE |  |
| `stock_financial_debt_ths` | `stock/fundamental/finance_more.rs::stock_financial_debt_ths` | `stock_fundamental/stock_finance_ths.py:58` | DONE |  |
| `stock_financial_hk_analysis_indicator_em` | `stock/fundamental/finance_more.rs::stock_financial_hk_analysis_indicator_em` | `stock_fundamental/stock_finance_hk_em.py:108` | DONE |  |
| `stock_financial_hk_report_em` | `stock/fundamental/finance_more.rs::stock_financial_hk_report_em` | `stock_fundamental/stock_finance_hk_em.py:13` | DONE |  |
| `stock_financial_report_sina` | `stock/fundamental/more.rs::stock_financial_report_sina` | `stock_fundamental/stock_finance_sina.py:24` | DONE |  |
| `stock_financial_us_analysis_indicator_em` | `stock/fundamental/finance_more.rs::stock_financial_us_analysis_indicator_em` | `stock_fundamental/stock_finance_us_em.py:158` | DONE |  |
| `stock_financial_us_report_em` | `stock/fundamental/finance_more.rs::stock_financial_us_report_em` | `stock_fundamental/stock_finance_us_em.py:110` | DONE |  |
| `stock_fund_stock_holder` | — | `stock_fundamental/stock_finance_sina.py:638` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_history_dividend` | — | `stock_fundamental/stock_finance_sina.py:327` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_history_dividend_detail` | — | `stock_fundamental/stock_finance_sina.py:360` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_hk_profit_forecast_et` | — | `stock_fundamental/stock_profit_forecast_hk_etnet.py:15` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_individual_basic_info_hk_xq` | — | `stock_fundamental/stock_basic_info_xq.py:129` | DEFERRED | session/token gated (xq_a_token/hexin-v) |
| `stock_individual_basic_info_us_xq` | — | `stock_fundamental/stock_basic_info_xq.py:106` | DEFERRED | session/token gated (xq_a_token/hexin-v) |
| `stock_individual_basic_info_xq` | — | `stock_fundamental/stock_basic_info_xq.py:83` | DEFERRED | session/token gated (xq_a_token/hexin-v) |
| `stock_individual_notice_report` | `stock/fundamental/wv_fund_misc.rs::stock_individual_notice_report` | `stock_fundamental/stock_notice.py:151` | DONE |  |
| `stock_institute_hold` | — | `stock_fundamental/stock_hold.py:17` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_institute_hold_detail` | — | `stock_fundamental/stock_hold.py:58` | DEFERRED | token/JS/HTML-gated |
| `stock_institute_recommend` | — | `stock_fundamental/stock_recommend.py:14` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_institute_recommend_detail` | — | `stock_fundamental/stock_recommend.py:76` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_ipo_declare_em` | `stock/fundamental/registration.rs::stock_ipo_declare_em` | `stock_fundamental/stock_ipo_declare.py:16` | DONE |  |
| `stock_ipo_hk_ths` | — | `stock_fundamental/stock_ipo_ths.py:81` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_ipo_info` | — | `stock_fundamental/stock_finance_sina.py:483` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_ipo_review_em` | `stock/fundamental/registration.rs::stock_ipo_review_em` | `stock_fundamental/stock_ipo_review.py:18` | DONE |  |
| `stock_ipo_ths` | — | `stock_fundamental/stock_ipo_ths.py:14` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_ipo_tutor_em` | `stock/fundamental/registration.rs::stock_ipo_tutor_em` | `stock_fundamental/stock_ipo_tutor.py:18` | DONE |  |
| `stock_kcb_detail_renewal` | `stock/fundamental/wv_fund_misc.rs::stock_kcb_detail_renewal` | `stock_fundamental/stock_kcb_detail_sse.py:14` | DONE |  |
| `stock_kcb_renewal` | `stock/fundamental/wv_fund_misc.rs::stock_kcb_renewal` | `stock_fundamental/stock_kcb_sse.py:14` | DONE |  |
| `stock_main_stock_holder` | — | `stock_fundamental/stock_finance_sina.py:696` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_management_change_ths` | — | `stock_fundamental/stock_finance_ths.py:574` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_notice_report` | `stock/fundamental/wv_fund_misc.rs::stock_notice_report` | `stock_fundamental/stock_notice.py:133` | DONE |  |
| `stock_profit_forecast_em` | `stock/fundamental/registration.rs::stock_profit_forecast_em` | `stock_fundamental/stock_profit_forecast_em.py:15` | DONE |  |
| `stock_profit_forecast_ths` | — | `stock_fundamental/stock_profit_forecast_ths.py:17` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_register_all_em` | `stock/fundamental/registration.rs::stock_register_all_em` | `stock_fundamental/stock_register_em.py:16` | DONE |  |
| `stock_register_bj` | `src (present)` | `stock_fundamental/stock_register_em.py:237` | DONE |  |
| `stock_register_cyb` | `src (present)` | `stock_fundamental/stock_register_em.py:163` | DONE |  |
| `stock_register_db` | `src (present)` | `stock_fundamental/stock_register_em.py:459` | DONE |  |
| `stock_register_kcb` | `src (present)` | `stock_fundamental/stock_register_em.py:89` | DONE |  |
| `stock_register_sh` | `src (present)` | `stock_fundamental/stock_register_em.py:311` | DONE |  |
| `stock_register_sz` | `src (present)` | `stock_fundamental/stock_register_em.py:385` | DONE |  |
| `stock_restricted_release_detail_em` | `stock/restricted.rs::stock_restricted_release_detail_em` | `stock_fundamental/stock_restricted_em.py:106` | DONE |  |
| `stock_restricted_release_queue_em` | `stock/restricted.rs::stock_restricted_release_queue_em` | `stock_fundamental/stock_restricted_em.py:209` | DONE |  |
| `stock_restricted_release_queue_sina` | — | `stock_fundamental/stock_finance_sina.py:531` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_restricted_release_stockholder_em` | `stock/restricted.rs::stock_restricted_release_stockholder_em` | `stock_fundamental/stock_restricted_em.py:301` | DONE |  |
| `stock_restricted_release_summary_em` | `stock/restricted.rs::stock_restricted_release_summary_em` | `stock_fundamental/stock_restricted_em.py:14` | DONE |  |
| `stock_shareholder_change_ths` | — | `stock_fundamental/stock_finance_ths.py:622` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |
| `stock_zh_a_gbjg_em` | `stock/fundamental/more.rs::stock_zh_a_gbjg_em` | `stock_fundamental/stock_gbjg_em.py:62` | DONE |  |
| `stock_zygc_em` | `stock/holder.rs::stock_zygc_em` | `stock_fundamental/stock_zygc.py:13` | DONE |  |
| `stock_zyjs_ths` | — | `stock_fundamental/stock_zyjs_ths.py:14` | DEFERRED | HTML table scraping (pd.read_html/BeautifulSoup) |

## tool

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `tool_trade_date_hist_sina` | — | `tool/trade_date_hist.py:19` | DEFERRED | needs JS execution (py_mini_racer/execjs) |

## utils

| akshare 函数 | 本库路径 | akshare 源文件:行 | 状态 | 原因 |
|---|---|---|---|---|
| `decode` | `src (present)` | `utils/demjson.py:6182` | DONE |  |
| `decode_file` | — | `utils/demjson.py:6387` | INTERNAL | akshare internal helper, not a data endpoint |
| `determine_float_limits` | — | `utils/demjson.py:79` | INTERNAL | akshare internal helper, not a data endpoint |
| `determine_float_precision` | — | `utils/demjson.py:231` | INTERNAL | akshare internal helper, not a data endpoint |
| `encode` | `src (present)` | `utils/demjson.py:6109` | DONE |  |
| `encode_to_file` | — | `utils/demjson.py:6349` | INTERNAL | akshare internal helper, not a data endpoint |
| `execute_js_in_executor` | — | `utils/multi_decrypt.py:32` | INTERNAL | akshare internal helper, not a data endpoint |
| `extend_and_flatten_list_with_sep` | — | `utils/demjson.py:777` | INTERNAL | akshare internal helper, not a data endpoint |
| `extend_list_with_sep` | — | `utils/demjson.py:767` | INTERNAL | akshare internal helper, not a data endpoint |
| `fetch_paginated_data` | — | | `utils/func.py:18` | INTERNAL | akshare internal helper (utils/func.py), not a data endpoint |
| `get_proxies` | — | `utils/context.py:27` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_token` | — | `utils/token_process.py:22` | INTERNAL | akshare internal helper, not a data endpoint |
| `get_tqdm` | — | `utils/tqdm.py:1` | INTERNAL | akshare internal helper, not a data endpoint |
| `js_executor_function` | — | `utils/multi_decrypt.py:15` | INTERNAL | akshare internal helper, not a data endpoint |
| `request_with_retry` | — | `utils/request.py:15` | INTERNAL | akshare internal helper, not a data endpoint |
| `set_df_columns` | — | `utils/func.py:63` | INTERNAL | akshare internal helper, not a data endpoint |
| `set_proxies` | — | `utils/context.py:23` | INTERNAL | akshare internal helper, not a data endpoint |
| `set_token` | — | `utils/token_process.py:15` | INTERNAL | akshare internal helper, not a data endpoint |
| `skipstringsafe` | — | `utils/demjson.py:742` | INTERNAL | akshare internal helper, not a data endpoint |
| `skipstringsafe_slow` | — | `utils/demjson.py:755` | INTERNAL | akshare internal helper, not a data endpoint |
| `smart_sort_transform` | — | | `utils/demjson.py:3068` | INTERNAL | akshare internal helper (utils/demjson.py), not a data endpoint |

---

**汇总**: DONE=775 · DEFERRED=233 · INTERNAL=63 · UNKNOWN=101 (共 1172)
