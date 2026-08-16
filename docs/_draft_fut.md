# Futures porting draft — wave (futures_hist/foreign/rule + deferred)

Assignment: port akshare `futures/*` and `futures_derivative/*` history/rule
functions to Rust. Feasible Eastmoney/Sina plain JSON/JSONP GETs were ported;
functions whose real request needs HTML scraping, JS execution (`demjson`), or
binary ZIP/TSV parsing (returning a `dict` of DataFrames) were deferred per the
porting DEFER policy.

## PORTED

| ak_fn | path::rust_fn | file:line | status | notes |
|---|---|---|---|---|
| futures_hist_em | crate::futures::wv_futures_more::futures_hist_em | src/futures/wv_futures_more.rs:227 | DONE | |
| futures_foreign_hist | crate::futures::wv_futures_more::futures_foreign_hist | src/futures/wv_futures_more.rs:354 | DONE | |
| futures_rule_em | crate::futures::wv_futures_more::futures_rule_em | src/futures/wv_futures_more.rs:406 | DONE | |

## DEFERRED

| ak_fn | path::rust_fn | file:line | status | reason |
|---|---|---|---|---|
| futures_inventory_99 |  | src/futures/wv_futures_more.rs:417 | DEFERRED | symbol→productId map scraped from 99qh.com `__NEXT_DATA__` via BeautifulSoup (HTML scrape trigger); data endpoint (fx168api.com) is clean JSON GET but unreachable without the scraped map |
| futures_dce_position_rank |  | src/futures/wv_futures_more.rs:417 | DEFERRED | DCE `batchDownload` returns a ZIP of TSV position-rank tables; result is a `dict` of DataFrames (not `Vec<Row>`), needs a `zip` dependency + complex table-slicing; outside Eastmoney/Sina plain-JSON scope |
| futures_display_main_sina |  | src/futures_derivative/wv_more.rs:18 | DEFERRED | uses `akshare.utils.demjson` to lenient-parse a Sina JS document (JS execution trigger) |

## Uncertainties

- `futures_rule_em`: upstream `GetPZJYInfo` returns `Data` as a list of objects
  with an **arbitrary/dynamic column set** (akshare does `pd.DataFrame(data_json["Data"])`
  with no rename). The row is modelled as a flattened `BTreeMap<String, Value>`
  to preserve every column faithfully. The real field names are dynamic; the
  synthetic fixture uses plausible 品种代码/品种名称/... keys and is labelled `_note`.
- `futures_foreign_hist`: the Sina `GlobalFuturesService.getGlobalFuturesDailyKLine`
  payload is mapped positionally to `[date, open, high, low, close, volume]`
  (standard OHLCV). akshare keeps default positional columns, so this matches;
  if upstream emits extra trailing fields they are ignored. Fixture labelled `_note`.
- `futures_hist_em` symbol→`secid` resolution faithfully replays akshare's
  `futsse-static.eastmoney.com` 3-layer `msgid` walk; the public fn is network-
  bound and is covered offline only via the parser / map-building unit tests.
