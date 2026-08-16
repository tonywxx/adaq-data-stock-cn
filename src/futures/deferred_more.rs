//! Additional deferred futures ports (re-triaged, not fakeable in Rust).
//!
//! These akshare functions from the assignment brief were re-read against
//! their source and **deferred** per `docs/PORTING_GUIDE.md` rule #5: they
//! require JS execution (`demjson` / `py_mini_racer`), HTML-table scraping,
//! or Excel/ZIP downloads that cannot be replicated source-faithfully.
//!
//! (A prior batch — `futures_zh_realtime`, `futures_comm_js`,
//! `futures_spot_price_daily`, `get_roll_yield`, `get_roll_yield_bar`,
//! `get_receipt`, `get_futures_daily` — is recorded in `deferred.rs`.)
//!
//! | akshare fn | source | reason |
//! |---|---|---|
//! | `futures_comm_info` | `futures_comm_qihuo.py:172` | `pd.read_html` scrape of 9qihuo.com fee table |
//! | `futures_contract_detail` | `futures_contract_detail.py:16` | `pd.read_html` scrape of Sina shtml |
//! | `futures_contract_detail_em` | `futures_contract_detail.py:41` | BeautifulSoup HTML parse to discover inner symbol, then JSON |
//! | `futures_delivery_czce` | `futures_to_spot.py:244` | CZCE `.xls` Excel download |
//! | `futures_delivery_dce` | `futures_to_spot.py:57` | DCE `pd.read_html` scrape |
//! | `futures_delivery_match_czce` | `futures_to_spot.py:198` | CZCE `.xls` Excel download |
//! | `futures_delivery_match_dce` | `futures_to_spot.py:128` | DCE `pd.read_html` scrape |
//! | `futures_fees_info` | `futures_comm_ctp.py:17` | BeautifulSoup + `pd.read_html` scrape of openctp.cn |
//! | `futures_foreign_commodity_realtime` | `futures_hq_sina.py:103` | `demjson` JS decode + BeautifulSoup (RMB price table) |
//! | `futures_foreign_commodity_subscribe_exchange_symbol` | `futures_hq_sina.py:38` | `demjson` JS decode (static equivalent already in `sina_hq.rs`) |
//! | `futures_foreign_detail` | `futures_foreign.py:45` | `pd.read_html` scrape of Sina shtml |
//! | `futures_rule` | `futures_rule.py:15` | `pd.read_html` scrape of gtjaqh.com calendar |
//! | `futures_settlement_price_sgx` | `futures_settlement_price_sgx.py:63` | SGX ZIP download |
//! | `futures_spot_price` | `futures_basis.py:79` | `pandas_read_html_link` scrape of 100ppi.com |
//! | `futures_spot_price_previous` | `futures_basis.py:300` | `pandas_read_html_link` scrape of 100ppi.com |
//! | `futures_spot_stock` | `futures_spot_stock_em.py:15` | `demjson` JS decode of Eastmoney page |
//! | `futures_stock_shfe_js` | `futures_stock_js.py:14` | JS execution (`py_mini_racer`) |
//! | `futures_symbol_mark` | `futures_zh_sina.py:28` | `demjson` JS decode |
//! | `futures_to_spot_czce` | `futures_to_spot.py:155` | CZCE `.xls` Excel download |
//! | `futures_to_spot_dce` | `futures_to_spot.py:97` | DCE `pd.read_html` scrape |
//! | `futures_warehouse_receipt_czce` | `futures_warehouse_receipt.py:23` | CZCE `.xls`/`.xlsx` Excel download |
//! | `get_cffex_daily` | `futures_daily_bar.py:108` | CFFEX ZIP download (CSV inside) |
//! | `match_main_contract` | `futures_zh_sina.py:171` | `demjson` JS decode |

#![allow(dead_code)]

/// Newly deferred akshare futures fns (see module doc table for reasons).
pub const DEFERRED_FNS: &[&str] = &[
    "futures_comm_info",
    "futures_contract_detail",
    "futures_contract_detail_em",
    "futures_delivery_czce",
    "futures_delivery_dce",
    "futures_delivery_match_czce",
    "futures_delivery_match_dce",
    "futures_fees_info",
    "futures_foreign_commodity_realtime",
    "futures_foreign_commodity_subscribe_exchange_symbol",
    "futures_foreign_detail",
    "futures_rule",
    "futures_settlement_price_sgx",
    "futures_spot_price",
    "futures_spot_price_previous",
    "futures_spot_stock",
    "futures_stock_shfe_js",
    "futures_symbol_mark",
    "futures_to_spot_czce",
    "futures_to_spot_dce",
    "futures_warehouse_receipt_czce",
    "get_cffex_daily",
    "match_main_contract",
];
