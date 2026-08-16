//! United States macro indicators — Jin10 `datacenter-api.jin10.com` port
//! (`akshare/economic/macro_usa.py`).
//!
//! **Every function in this module is DEFERRED.** They all resolve to akshare's
//! private `__macro_usa_base_func`, which issues a `GET` to
//! `https://datacenter-api.jin10.com/reports/list_v2?category=ec&attr_id=<N>&...`
//! with a required `x-csrf-token` header (akshare hardcodes the placeholder
//! `"x-csrf-token": "x-csrf-token"`) and depends on a Jin10 session cookie. The
//! real token + cookie are issued by Jin10's JS frontend and cannot be obtained
//! without a browser session, so these are auth/session-gated and not ported.
//!
//! Pagination also relies on a `max_date` cursor computed in Python from the
//! previous page's last row — pure HTTP, but irrelevant given the token gate.
//!
//! ## DEFERRED
//!
//! All deferred for the same reason: **Jin10 token-gated** (`x-csrf-token` +
//! session cookie required by `datacenter-api.jin10.com/reports/list_v2`; no
//! plain Eastmoney/Sina/other unauthenticated JSON available for these series).
//!
//! | Rust fn | akshare fn | akshare line | `attr_id` |
//! | --- | --- | --- | --- |
//! | `macro_usa_adp_employment` | `macro_usa_adp_employment` | macro_usa.py:374 | 1 |
//! | `macro_usa_api_crude_stock` | `macro_usa_api_crude_stock` | macro_usa.py:534 | 69 |
//! | `macro_usa_building_permits` | `macro_usa_building_permits` | macro_usa.py:763 | 3 |
//! | `macro_usa_business_inventories` | `macro_usa_business_inventories` | macro_usa.py:668 | 4 |
//! | `macro_usa_cb_consumer_confidence` | `macro_usa_cb_consumer_confidence` | macro_usa.py:862 | 5 |
//! | `macro_usa_core_cpi_monthly` | `macro_usa_core_cpi_monthly` | macro_usa.py:205 | 6 |
//! | `macro_usa_core_pce_price` | `macro_usa_core_pce_price` | macro_usa.py:392 | 80 |
//! | `macro_usa_core_ppi` | `macro_usa_core_ppi` | macro_usa.py:515 | 7 |
//! | `macro_usa_cpi_monthly` | `macro_usa_cpi_monthly` | macro_usa.py:186 | 9 |
//! | `macro_usa_current_account` | `macro_usa_current_account` | macro_usa.py:448 | 12 |
//! | `macro_usa_durable_goods_orders` | `macro_usa_durable_goods_orders` | macro_usa.py:611 | 13 |
//! | `macro_usa_eia_crude_rate` | `macro_usa_eia_crude_rate` | macro_usa.py:923 | 10 |
//! | `macro_usa_exist_home_sales` | `macro_usa_exist_home_sales` | macro_usa.py:782 | 15 |
//! | `macro_usa_export_price` | `macro_usa_export_price` | macro_usa.py:281 | 79 |
//! | `macro_usa_factory_orders` | `macro_usa_factory_orders` | macro_usa.py:630 | 16 |
//! | `macro_usa_gdp_monthly` | `macro_usa_gdp_monthly` | macro_usa.py:167 | 53 |
//! | `macro_usa_house_price_index` | `macro_usa_house_price_index` | macro_usa.py:801 | 51 |
//! | `macro_usa_house_starts` | `macro_usa_house_starts` | macro_usa.py:725 | 17 |
//! | `macro_usa_import_price` | `macro_usa_import_price` | macro_usa.py:262 | 18 |
//! | `macro_usa_industrial_production` | `macro_usa_industrial_production` | macro_usa.py:592 | 20 |
//! | `macro_usa_initial_jobless` | `macro_usa_initial_jobless` | macro_usa.py:942 | 44 |
//! | `macro_usa_ism_non_pmi` | `macro_usa_ism_non_pmi` | macro_usa.py:687 | 29 |
//! | `macro_usa_ism_pmi` | `macro_usa_ism_pmi` | macro_usa.py:573 | 28 |
//! | `macro_usa_job_cuts` | `macro_usa_job_cuts` | macro_usa.py:338 | 78 |
//! | `macro_usa_lmci` | `macro_usa_lmci` | macro_usa.py:301 | 93 |
//! | `macro_usa_michigan_consumer_sentiment` | `macro_usa_michigan_consumer_sentiment` | macro_usa.py:902 | 50 |
//! | `macro_usa_nahb_house_market_index` | `macro_usa_nahb_house_market_index` | macro_usa.py:706 | 31 |
//! | `macro_usa_new_home_sales` | `macro_usa_new_home_sales` | macro_usa.py:744 | 32 |
//! | `macro_usa_nfib_small_business` | `macro_usa_nfib_small_business` | macro_usa.py:881 | 63 |
//! | `macro_usa_non_farm` | `macro_usa_non_farm` | macro_usa.py:356 | 33 |
//! | `macro_usa_pending_home_sales` | `macro_usa_pending_home_sales` | macro_usa.py:841 | 34 |
//! | `macro_usa_personal_spending` | `macro_usa_personal_spending` | macro_usa.py:224 | 35 |
//! | `macro_usa_pmi` | `macro_usa_pmi` | macro_usa.py:554 | 74 |
//! | `macro_usa_ppi` | `macro_usa_ppi` | macro_usa.py:496 | 37 |
//! | `macro_usa_real_consumer_spending` | `macro_usa_real_consumer_spending` | macro_usa.py:410 | 81 |
//! | `macro_usa_retail_sales` | `macro_usa_retail_sales` | macro_usa.py:243 | 39 |
//! | `macro_usa_services_pmi` | `macro_usa_services_pmi` | macro_usa.py:649 | 89 |
//! | `macro_usa_spcs20` | `macro_usa_spcs20` | macro_usa.py:820 | 52 |
//! | `macro_usa_trade_balance` | `macro_usa_trade_balance` | macro_usa.py:430 | 42 |
//! | `macro_usa_unemployment_rate` | `macro_usa_unemployment_rate` | macro_usa.py:320 | 47 |
