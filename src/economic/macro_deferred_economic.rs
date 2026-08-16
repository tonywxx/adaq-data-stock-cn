//! Deferred economic-domain functions (akshare `economic` package).
//!
//! Every function in this batch was triaged by reading its akshare source
//! (`file:line`). None are pure-JSON / Eastmoney-datacenter calls; all require a
//! mechanism on the deferral list (Sina `demjson` JSONP / Sina GBK text,
//! Jin10 `x-csrf-token`/`datacenter-api`, Excel `read_excel`, or 同花顺 HTML
//! `read_html`). They are recorded here as `DEFERRED` so re-runs skip them.
//!
//! ## DEFERRED
//!
//! ### Sina JSONP + `demjson` — `akshare/economic/macro_china.py`
//! | akshare fn | src:line | reason |
//! |---|---|---|
//! | `macro_china_society_electricity` | `macro_china.py:3236` | Sina JSONP + `demjson.decode` |
//! | `macro_china_society_traffic_volume` | `macro_china.py:3289` | Sina JSONP + `demjson.decode` |
//! | `macro_china_postal_telecommunicational` | `macro_china.py:3347` | Sina JSONP + `demjson.decode` |
//! | `macro_china_international_tourism_fx` | `macro_china.py:3381` | Sina JSONP + `demjson.decode` |
//! | `macro_china_passenger_load_factor` | `macro_china.py:3415` | Sina JSONP + `demjson.decode` |
//! | `macro_china_central_bank_balance` | `macro_china.py:3526` | Sina JSONP + `demjson.decode` |
//! | `macro_china_insurance` | `macro_china.py:3560` | Sina JSONP + `demjson.decode` |
//! | `macro_china_supply_of_money` | `macro_china.py:3594` | Sina JSONP + `demjson.decode` |
//! | `macro_china_foreign_exchange_gold` | `macro_china.py:3628` | Sina JSONP + `demjson.decode` |
//! | `macro_china_retail_price_index` | `macro_china.py:3663` | Sina JSONP + `demjson.decode` |
//!
//! ### Sina GBK text — `akshare/economic/macro_china.py`
//! | akshare fn | src:line | reason |
//! |---|---|---|
//! | `macro_china_freight_index` | `macro_china.py:3481` | Sina GBK text (`vMacExcle.php`, `demjson`-free but GBK/CSV scrape) |
//!
//! ### Jin10 `datacenter-api` + `x-csrf-token` — `akshare/economic/macro_usa.py`
//! All 39 hit `https://datacenter-api.jin10.com/reports/list_v2` via
//! `__macro_usa_base_func` with `x-csrf-token`/`x-app-id` headers (deferral).
//! | akshare fn | src:line | reason |
//! |---|---|---|
//! | `macro_usa_gdp_monthly` | `macro_usa.py:167` | Jin10 `datacenter-api` |
//! | `macro_usa_cpi_monthly` | `macro_usa.py:186` | Jin10 `datacenter-api` |
//! | `macro_usa_core_cpi_monthly` | `macro_usa.py:205` | Jin10 `datacenter-api` |
//! | `macro_usa_personal_spending` | `macro_usa.py:224` | Jin10 `datacenter-api` |
//! | `macro_usa_retail_sales` | `macro_usa.py:243` | Jin10 `datacenter-api` |
//! | `macro_usa_import_price` | `macro_usa.py:262` | Jin10 `datacenter-api` |
//! | `macro_usa_export_price` | `macro_usa.py:281` | Jin10 `datacenter-api` |
//! | `macro_usa_lmci` | `macro_usa.py:301` | Jin10 `datacenter-api` |
//! | `macro_usa_unemployment_rate` | `macro_usa.py:320` | Jin10 `datacenter-api` |
//! | `macro_usa_job_cuts` | `macro_usa.py:338` | Jin10 `datacenter-api` |
//! | `macro_usa_non_farm` | `macro_usa.py:356` | Jin10 `datacenter-api` |
//! | `macro_usa_adp_employment` | `macro_usa.py:374` | Jin10 `datacenter-api` |
//! | `macro_usa_core_pce_price` | `macro_usa.py:392` | Jin10 `datacenter-api` |
//! | `macro_usa_real_consumer_spending` | `macro_usa.py:410` | Jin10 `datacenter-api` |
//! | `macro_usa_trade_balance` | `macro_usa.py:430` | Jin10 `datacenter-api` |
//! | `macro_usa_current_account` | `macro_usa.py:448` | Jin10 `datacenter-api` |
//! | `macro_usa_ppi` | `macro_usa.py:496` | Jin10 `datacenter-api` |
//! | `macro_usa_core_ppi` | `macro_usa.py:515` | Jin10 `datacenter-api` |
//! | `macro_usa_api_crude_stock` | `macro_usa.py:534` | Jin10 `datacenter-api` |
//! | `macro_usa_pmi` | `macro_usa.py:554` | Jin10 `datacenter-api` |
//! | `macro_usa_ism_pmi` | `macro_usa.py:573` | Jin10 `datacenter-api` |
//! | `macro_usa_industrial_production` | `macro_usa.py:592` | Jin10 `datacenter-api` |
//! | `macro_usa_durable_goods_orders` | `macro_usa.py:611` | Jin10 `datacenter-api` |
//! | `macro_usa_factory_orders` | `macro_usa.py:630` | Jin10 `datacenter-api` |
//! | `macro_usa_services_pmi` | `macro_usa.py:649` | Jin10 `datacenter-api` |
//! | `macro_usa_business_inventories` | `macro_usa.py:668` | Jin10 `datacenter-api` |
//! | `macro_usa_ism_non_pmi` | `macro_usa.py:687` | Jin10 `datacenter-api` |
//! | `macro_usa_nahb_house_market_index` | `macro_usa.py:706` | Jin10 `datacenter-api` |
//! | `macro_usa_house_starts` | `macro_usa.py:725` | Jin10 `datacenter-api` |
//! | `macro_usa_new_home_sales` | `macro_usa.py:744` | Jin10 `datacenter-api` |
//! | `macro_usa_building_permits` | `macro_usa.py:763` | Jin10 `datacenter-api` |
//! | `macro_usa_exist_home_sales` | `macro_usa.py:782` | Jin10 `datacenter-api` |
//! | `macro_usa_house_price_index` | `macro_usa.py:801` | Jin10 `datacenter-api` |
//! | `macro_usa_pending_home_sales` | `macro_usa.py:841` | Jin10 `datacenter-api` |
//! | `macro_usa_cb_consumer_confidence` | `macro_usa.py:862` | Jin10 `datacenter-api` |
//! | `macro_usa_nfib_small_business` | `macro_usa.py:881` | Jin10 `datacenter-api` |
//! | `macro_usa_michigan_consumer_sentiment` | `macro_usa.py:902` | Jin10 `datacenter-api` |
//! | `macro_usa_eia_crude_rate` | `macro_usa.py:923` | Jin10 `datacenter-api` |
//! | `macro_usa_initial_jobless` | `macro_usa.py:942` | Jin10 `datacenter-api` |
//!
//! ### Jin10 `datacenter-api` + `x-csrf-token` — `akshare/economic/macro_constitute.py`
//! | akshare fn | src:line | reason |
//! |---|---|---|
//! | `macro_cons_gold` | `macro_constitute.py:17` | Jin10 `datacenter-api` |
//! | `macro_cons_silver` | `macro_constitute.py:82` | Jin10 `datacenter-api` |
//! | `macro_cons_opec_month` | `macro_constitute.py:147` | Jin10 `datacenter-api` |
//!
//! ### Excel `read_excel` — `akshare/economic/marco_cnbs.py`
//! | akshare fn | src:line | reason |
//! |---|---|---|
//! | `macro_cnbs` | `marco_cnbs.py:12` | Excel download (`pd.read_excel`) |
//!
//! ### 同花顺 HTML `read_html` — `akshare/economic/macro_finance_ths.py`
//! | akshare fn | src:line | reason |
//! |---|---|---|
//! | `macro_stock_finance` | `macro_finance_ths.py:15` | 同花顺 HTML table (`pd.read_html`) |
//! | `macro_rmb_loan` | `macro_finance_ths.py:50` | 同花顺 HTML table (`pd.read_html`) |
//! | `macro_rmb_deposit` | `macro_finance_ths.py:82` | 同花顺 HTML table (`pd.read_html`) |
