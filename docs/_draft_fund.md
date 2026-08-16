# Fund porting draft (_draft_fund.md)

Auto-generated tracking for the `src/fund/wv_fund_more.rs` batch.
Ported/deferred status for akshare fund functions assigned to this module.

## PORTED
| akshare fn | rust location | source | status | note |
|---|---|---|---|---|

## DEFERRED
| fund_etf_dividend_sina |  | fund/fund_etf_sina.py:152 | DEFERRED | Sina `hfq.js` returns `var x = {...}` JS object literal parsed via Python `eval` (non-JSON special format); not a clean JSON/JSONP GET, cannot port faithfully offline |
| fund_individual_basic_info_xq |  | fund/fund_xq.py:13 | DEFERRED | danjuanfunds/xueqiu API requires `xq_a_token` cookie / session |
| fund_individual_achievement_xq |  | fund/fund_xq.py:78 | DEFERRED | xueqiu `xq_a_token` cookie gate |
| fund_individual_analysis_xq |  | fund/fund_xq.py:132 | DEFERRED | xueqiu `xq_a_token` cookie gate |
| fund_individual_profit_probability_xq |  | fund/fund_xq.py:185 | DEFERRED | xueqiu `xq_a_token` cookie gate |
| fund_individual_detail_info_xq |  | fund/fund_xq.py:224 | DEFERRED | xueqiu `xq_a_token` cookie gate |
| fund_individual_detail_hold_xq |  | fund/fund_xq.py:270 | DEFERRED | xueqiu `xq_a_token` cookie gate |
