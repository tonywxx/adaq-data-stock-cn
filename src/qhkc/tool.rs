//! 奇货可查-工具数据 (qhkc tool). Ports `akshare/qhkc_web/qhkc_tool.py`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `qhkc_tool_foreign` | `qhkc_tool.py:17` | 外盘比价; POST `toolbox_foreign.php`, `data[].{name,base_time,base_price,latest_price,rate}` |
//! | `qhkc_tool_gdp` | `qhkc_tool.py:111` | 各地区经济数据; `pd.read_html(url)` HTML scrape of `gdp.html` |
//!
//! ## DEFERRED
//!
//! - `qhkc_tool_foreign` — endpoint `https://qhkch.com/ajax/toolbox_foreign.php`
//!   returns `{"code":404,"message":"Not Found"}` (probed 2026-08-15). Public JSON
//!   API withdrawn / behind commercial token auth.
//! - `qhkc_tool_gdp` — the akshare source fetches an HTML page and parses it with
//!   `pd.read_html(url)` (`https://qhkch.com/dist/views/toolbox/gdp.html`), an
//!   HTML-table scrape barrier per the porting guide. It would be deferred even
//!   if reachable, and the endpoint is also effectively dead.
//!
//! (Note: `qhkc_tool_nebula` in the same source file is out of scope for this
//! assignment and is likewise deferred for the same `toolbox_foreign.php` 404.)
