# Economic porting draft (_draft_econ.md)

Auto-generated port/defer ledger for the `macro_more3` assignment.
Format: `| ak_fn | rust_loc | akshare_loc | status | reason |`

## PORTED (5)

| ak_fn | rust_loc | akshare_loc | status | reason |
| --- | --- | --- | --- | --- |
| macro_china_shrzgm | src/economic/macro_more3.rs::macro_china_shrzgm | macro_china.py:258 | DONE | |
| macro_china_urban_unemployment | src/economic/macro_more3.rs::macro_china_urban_unemployment | macro_china.py:318 | DONE | |
| macro_cons_gold | src/economic/macro_more3.rs::macro_cons_gold | macro_constitute.py:17 | DONE | |
| macro_cons_silver | src/economic/macro_more3.rs::macro_cons_silver | macro_constitute.py:82 | DONE | |
| macro_cons_opec_month | src/economic/macro_more3.rs::macro_cons_opec_month | macro_constitute.py:147 | DONE | |

## DEFERRED (46)

| ak_fn | rust_loc | akshare_loc | status | reason |
| --- | --- | --- | --- | --- |
| macro_china_daily_energy |  | macro_china.py:750 | DEFERRED | Jin10 CDN `.js` file with embedded JSON extracted via text-slice (not a plain JSON endpoint) |
| macro_china_freight_index |  | macro_china.py:3481 | DEFERRED | Sina GBK text-slice/JSONP, not plain JSON |
| macro_china_nbs_nation |  | macro_china_nbs.py:517 | DEFERRED | Jin10 `reports/list_v2` session warmup via curl_cffi |
| macro_china_nbs_region |  | macro_china_nbs.py:566 | DEFERRED | Jin10 `reports/list_v2` session warmup via curl_cffi |
| macro_euro_lme_holding |  | macro_euro.py:839 | DEFERRED | upstream `eval`'d nested tuple strings |
| macro_euro_lme_stock |  | macro_euro.py:870 | DEFERRED | upstream `eval`'d nested tuple strings |
| macro_usa_adp_employment |  | macro_usa.py:374 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_api_crude_stock |  | macro_usa.py:534 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_building_permits |  | macro_usa.py:763 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_business_inventories |  | macro_usa.py:668 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_cb_consumer_confidence |  | macro_usa.py:862 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_core_cpi_monthly |  | macro_usa.py:205 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_core_pce_price |  | macro_usa.py:392 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_core_ppi |  | macro_usa.py:515 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_cpi_monthly |  | macro_usa.py:186 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_current_account |  | macro_usa.py:448 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_durable_goods_orders |  | macro_usa.py:611 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_eia_crude_rate |  | macro_usa.py:923 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_exist_home_sales |  | macro_usa.py:782 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_export_price |  | macro_usa.py:281 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_factory_orders |  | macro_usa.py:630 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_gdp_monthly |  | macro_usa.py:167 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_house_price_index |  | macro_usa.py:801 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_house_starts |  | macro_usa.py:725 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_import_price |  | macro_usa.py:262 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_industrial_production |  | macro_usa.py:592 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_initial_jobless |  | macro_usa.py:942 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_ism_non_pmi |  | macro_usa.py:687 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_ism_pmi |  | macro_usa.py:573 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_job_cuts |  | macro_usa.py:338 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_lmci |  | macro_usa.py:301 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_michigan_consumer_sentiment |  | macro_usa.py:902 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_nahb_house_market_index |  | macro_usa.py:706 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_new_home_sales |  | macro_usa.py:744 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_nfib_small_business |  | macro_usa.py:881 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_non_farm |  | macro_usa.py:356 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_pending_home_sales |  | macro_usa.py:841 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_personal_spending |  | macro_usa.py:224 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_pmi |  | macro_usa.py:554 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_ppi |  | macro_usa.py:496 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_real_consumer_spending |  | macro_usa.py:410 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_retail_sales |  | macro_usa.py:243 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_services_pmi |  | macro_usa.py:649 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_spcs20 |  | macro_usa.py:820 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_trade_balance |  | macro_usa.py:430 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
| macro_usa_unemployment_rate |  | macro_usa.py:320 | DEFERRED | Jin10 `datacenter-api` `reports/list_v2` dynamic `x-csrf-token` (session-gated) |
