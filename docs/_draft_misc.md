# Draft triage — misc akshare ports (2026-08-16)

Ported/deferred records for the assigned akshare functions. Format:
`PORTED: | ak_fn | crate::path::rust_fn | src/file:line | DONE | |`
`DEFERRED: | ak_fn |  | akshare/file.py:line | DEFERRED | reason |`

## PORTED

| ak_fn | rust_fn | file:line | status | note |
|---|---|---|---|---|
| fx_quote_baidu | alt::wv_fx_more::fx_quote_baidu | src/alt/wv_fx_more.rs:119 | DONE | |

## DEFERRED

| ak_fn |  | akshare source | status | reason |
|---|---|---|---|---|
| spot_price_qh |  | spot/spot_price_qh.py:79 | DEFERRED | requires dynamic anti-bot `_pcc` token from `centerapi.fx168api.com/app/common/v.js` response header; plain JSON GET is gated (module `src/spot/price_qh.rs` already declares this deferred). |
| macro_china_swap_rate |  | bond/bond_china_money.py:192 | DEFERRED | `requests.post(...)` — ChinaMoney swap-rate endpoint is a POST (form body), not a plain JSON/JSONP GET; also calls `bond_china_close_return_map()` setup first. |
| migration_area_baidu |  | event/migration.py:16 | DEFERRED | JSONP (`cityrank.jsonp`) behind Baidu huiyan anti-bot; needs province/city id map from `event/cons.py`. Deferred-policy domain. |
| migration_scale_baidu |  | event/migration.py:56 | DEFERRED | JSONP (`historycurve.jsonp`) behind Baidu huiyan anti-bot; needs province/city id map from `event/cons.py`. Deferred-policy domain. |
| qhkc_tool_foreign |  | qhkc_web/qhkc_tool.py:17 | DEFERRED | `requests.post(...)` to qhkch.com; POST endpoint + commercial token auth (deferred-policy domain). |
| qhkc_tool_nebula |  | qhkc_web/qhkc_tool.py:65 | DEFERRED | `requests.post(...)` to qhkch.com; POST endpoint + commercial token auth (deferred-policy domain). |

## Notes

- `fx_quote_baidu` is a plain JSON GET but the upstream enforces a Baidu
  anti-bot `acs-token` (akshare's `token` param). Live probe from this
  environment returned HTTP 403 without a valid browser token. The port is
  structurally faithful (GET + optional `acs-token` header, default empty,
  matching akshare's `token=""`); the token must be supplied at call time.
  Fixture `tests/fixtures/fx_quote_baidu.json` is synthetic (`_note`).
- Deferred `wv_*` modules (bond, event, qhkc) were intentionally NOT created
  as empty files; the DEFERRED policy is "record, don't implement". The
  `event` and `qhkc` domains already document these as deferred in their
  `mod.rs` docs.
