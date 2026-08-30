# Gap Analysis — `simonlin1212/a-stock-data` skill vs `adaq-data-stock-cn`

**Source skill**: `simonlin1212/a-stock-data` (SKILL.md v3.7.1) — single-file Python agent skill,
dropped akshare in v3.0, direct to 19 upstream sources / ~54 endpoints.
**Target**: port / enrich into this Rust crate, following the project rule
**"add a new source as a default option, never delete the original implementation."**

## Verification rule (agreed)
1. Write a live smoke test (`examples/` or a throwaway bin), run it, confirm real payload.
2. Save the response as an offline fixture under `tests/fixtures/`.
3. Implement the function + an offline `#[cfg(test)]` fixture test.
4. Hosts that are blocked get implemented against a hand-built fixture and tagged **LIVE-UNVERIFIED**.

## Network status (probed 2026-08-29)
- ✅ Reachable no-key HTTP hosts: `qt.gtimg.cn`, `push2his.eastmoney.com`, `finance.pae.baidu.com`,
  `finance.sina.com.cn`, `quotes.sina.cn`, `reportapi.eastmoney.com`, `pdf.dfcfw.com`,
  `basic.10jqka.com.cn`, `zx.10jqka.com.cn`, `data.hexin.cn`, `push2.eastmoney.com`,
  `search-api-web.eastmoney.com`, `www.cls.cn`, `np-weblist.eastmoney.com`, `www.cninfo.com.cn`,
  `push2ex.eastmoney.com`, `data.10jqka.com.cn`, `mobappconfig.securities.eastmoney.com`,
  `dycalchis.eastmoney.com`, `stock.finance.sina.com.cn`, `irm.cninfo.com.cn`, `dq.10jqka.com.cn`,
  `emappdata.eastmoney.com`, `www.pbc.gov.cn`, `www.stats.gov.cn`, `vip.stock.finance.sina.com.cn`.
  (404/403 on bare host = reachable, just needs path/Referer.)
- ❌ **Blocked**: `datacenter-web.eastmoney.com` (connection timeout). Affects the functions marked
  **LIVE-UNVERIFIED** below.
- ⚠️ Degraded: `www.swsresearch.com` returned 508 (overloaded) — retry later.

## Legend
- **DONE** — already in this crate (may still be re-verified live).
- **ENRICH** — we have it via another source; add this skill's source as a default in the `SourceChain`.
- **ADD** — new function to implement.
- **DEFER** — out of scope this round (TCP lib / API key / blocked host), recorded for later.

---

## Quotation
| skill fn | host | auth | status | our existing | notes |
|---|---|---|---|---|---|
| `tdx_client` (bars/quotes/transaction) | 119.97.185.59:7709 | tcp | **DEFER** | — | mootdx TCP; new arch/dep |
| `tencent_quote` | qt.gtimg.cn | none | **DONE** | `stock::spot::tencent` | re-verify live |
| `baidu_kline_with_ma` | finance.pae.baidu.com | referer | **ADD** | — | K-line + MA5/10/20; reachable |
| `sina_adjust_factor` | finance.sina.com.cn | referer | **DONE** ✅ | `stock_zh_a_daily` (qfq/hfq) | 原始复权因子序列 — `src/stock/sina_adjust_factor.rs` |

## Research
| skill fn | host | auth | status | our existing | notes |
|---|---|---|---|---|---|
| `eastmoney_reports` | reportapi.eastmoney.com | referer | **DONE** ✅ | `stock_research_report_em` | 个股研报列表 (live 771 rows) |
| `download_pdf` | pdf.dfcfw.com | referer | **ADD** | — | needs PDF handling (save to path) |
| `eastmoney_industry_reports` | reportapi.eastmoney.com | referer | **ADD** | — | 行业研报 |
| `ths_eps_forecast` | basic.10jqka.com.cn | referer | **DEFER** (403) | — | 一致预期EPS; WAF 403 on this IP |
| `iwencai_search` / `iwencai_query` | openapi.iwencai.com | **key** | **DEFER** | — | needs IWENCAI_API_KEY |

## Signals
| skill fn | host | auth | status | our existing | notes |
|---|---|---|---|---|---|
| `ths_hot_reason` | zx.10jqka.com.cn | referer | **DONE** ✅ | — | 强势股题材归因 — `src/stock/ths_hot_reason.rs` (live 81 rows) |
| `hsgt_realtime` | data.hexin.cn | referer | **DONE** | `stock::hsgt::*` | re-verify live |
| `eastmoney_concept_blocks` | push2.eastmoney.com | referer | **DONE** ✅ | `stock_board_concept_*` | 个股所属板块+BK码 — `src/stock/em_signal.rs` |
| `eastmoney_fund_flow_minute` | push2.eastmoney.com | referer | **DEFER** (502) | — | 分钟级主力净流入; route 502 on this IP |
| `dragon_tiger_board` | datacenter-web | referer | **DONE** (LIVE-UNVERIFIED) | `stock_lhb_*_em` | host blocked → fixture-only |
| `lockup_expiry` | datacenter-web | referer | **ADD** (LIVE-UNVERIFIED) | — | 解禁; host blocked |
| `industry_comparison` | push2.eastmoney.com | referer | **DONE** ✅ | — | 行业涨跌幅排名 — `src/stock/em_signal.rs` (live 100 rows) |
| `board_fund_flow` | push2.eastmoney.com | referer | **ENRICH** | `stock_sector_fund_flow_rank` | 板块×1/5/10日资金流 |
| `daily_dragon_tiger` | datacenter-web | referer | **ADD** (LIVE-UNVERIFIED) | — | 全市场龙虎榜; host blocked |

## Fund / Chips
| skill fn | host | auth | status | our existing | notes |
|---|---|---|---|---|---|
| `margin_trading` | datacenter-web | referer | **DONE** (LIVE-UNVERIFIED) | `stock_margin_*` | host blocked |
| `block_trade` | datacenter-web | referer | **ADD** (LIVE-UNVERIFIED) | — | 大宗交易; host blocked |
| `holder_num_change` | datacenter-web | referer | **ADD** (LIVE-UNVERIFIED) | — | 股东户数; host blocked |
| `dividend_history` | datacenter-web | referer | **ADD** (LIVE-UNVERIFIED) | `dividend_payout_em` (hk) | A-share分红; host blocked |
| `stock_fund_flow_120d` | push2his.eastmoney.com | referer | **DONE** ✅ | `stock_fund_flow_120d` | 120日资金流 |
| `chip_distribution` (CYQ) | local (OHLC) | none | **ADD** | — | 筹码分布; 本地计算, 高价值 |

## News
| skill fn | host | auth | status | our existing | notes |
|---|---|---|---|---|---|
| `eastmoney_stock_news` | search-api-web.eastmoney.com | referer | **DONE** ✅ | `news::stock_news_em` | 个股新闻流 (live 10 rows) |
| `cls_telegraph` | www.cls.cn | **sign** | **DONE** ✅ | `news::telegraph` | 财联社电报; 本地md5(sha1)签名, 无key |
| `eastmoney_global_news` | np-weblist.eastmoney.com | referer | **DONE** ✅ | `stock_info_global_em` | 7×24快讯 (live 200 rows) |

## Base
| skill fn | host | auth | status | our existing | notes |
|---|---|---|---|---|---|
| `client.finance` / `client.F10` (mootdx) | 119.97.185.59:7709 | tcp | **DEFER** | — | TCP |
| `eastmoney_stock_info` | push2.eastmoney.com | referer | **DONE** | `stock_info_a_code_name` 等 | re-verify live |
| `sina_financial_report` | quotes.sina.cn | none | **DONE** | `stock_fundamental` sina | re-verify live |
| `baostock_valuation_history` | api.baostock.com | tcp | **DEFER** | — | TCP |
| `baostock_stock_basic` | api.baostock.com | tcp | **DEFER** | — | TCP |
| `sw_industry_history` | www.swsresearch.com | none | **ADD** (⚠️ retry) | — | 申万行业变迁; 508 |

## Announcements
| skill fn | host | auth | status | our existing | notes |
|---|---|---|---|---|---|
| `cninfo_announcements` | www.cninfo.com.cn | referer | **DONE** ✅ | `stock_zh_a_disclosure_report_cninfo` | 公告检索 (live 62 rows) |

## LimitUp / Monitor
| skill fn | host | auth | status | our existing | notes |
|---|---|---|---|---|---|
| `em_zt_pool` | push2ex.eastmoney.com | referer | **DONE** | `stock_zt_pool_em` | re-verify live |
| `em_zb_pool` | push2ex.eastmoney.com | referer | **DONE** ✅ | `em_zb_pool` | 炸板池 |
| `em_dt_pool` | push2ex.eastmoney.com | referer | **DONE** ✅ | `em_dt_pool` | 跌停池 |
| `em_yzt_pool` | push2ex.eastmoney.com | referer | **DONE** ✅ | `em_yzt_pool` | 昨涨停今表现 |
| `ths_limit_up_pool` | data.10jqka.com.cn | referer | **ADD** | — | 涨停原因/封板率 ✅ |
| `em_stock_monitor` | mobappconfig.securities.eastmoney.com | referer | **DONE** ✅ | — | 重点监控名单 — `src/stock/em_signal.rs` (live 20 rows) |
| `em_price_anomaly` | dycalchis.eastmoney.com | referer | **DONE** ✅ | — | 严重异常波动(12规则) — `src/stock/em_signal.rs` (live 8 rows) |
| `em_price_anomaly_count` | dycalchis.eastmoney.com | referer | **DONE** ✅ | — | 异动统计 — `src/stock/em_signal.rs` (live 8 rows) |

## Options
| skill fn | host | auth | status | our existing | notes |
|---|---|---|---|---|---|
| `sina_option_codes` | stock.finance.sina.com.cn | referer | **DONE** | `option_*_sina` | re-verify live |
| `sina_option_tquote` | hq.sinajs.cn | referer | **DONE** | `option_sse_spot_price_sina` | re-verify live |
| `sina_option_greeks` | hq.sinajs.cn | referer | **DONE** | `option_sse_greeks_sina` | re-verify live |

## Sentiment
| skill fn | host | auth | status | our existing | notes |
|---|---|---|---|---|---|
| `cninfo_irm` | irm.cninfo.com.cn | none | **DONE** ✅ | — | 互动易问答 — `src/news/cninfo_irm.rs` |
| `ths_hot_list` | dq.10jqka.com.cn | none | **DONE** ✅ | — | 同花顺热榜 — `src/stock/sentiment.rs` |
| `em_hot_rank` | emappdata.eastmoney.com | none | **DONE** ✅ | — | 东财人气榜 — `src/stock/sentiment.rs` |
| `em_hot_concept` | emappdata.eastmoney.com | none | **DONE** ✅ | — | 热门概念命中 — `src/stock/sentiment.rs` |

## Macro
| skill fn | host | auth | status | our existing | notes |
|---|---|---|---|---|---|
| `pboc_social_financing` | www.pbc.gov.cn | none | **DONE** | `economic::*` | re-verify live |
| `nbs_pmi` | www.stats.gov.cn | none | **DONE** | `economic::*` | re-verify live |

## Backup (degrade) — optional later
| skill fn | host | status | notes |
|---|---|---|---|
| `dragon_tiger_backup` | sse/szse | optional | 官方龙虎榜兜底 |
| `fund_flow_backup` | vip.stock.finance.sina.com.cn | optional | 新浪资金流兜底 |
| `announcements_backup` | szse / np-anotice-stock.eastmoney.com | optional | 公告兜底 |

---

## Implementation plan (this round)
**Priority 1 — no-key, reachable, high value (implement now, verify live):**
1. ✅ **DONE (live-verified)** `cls_telegraph` (财联社, sign) — `src/news/cls.rs`.
2. ✅ **DONE (live-verified)** `baidu_kline_with_ma` (百度股市通 K线+MA) — `src/stock/baidu_kline.rs`.
3. ✅ **DONE (offline-verified)** `chip_distribution` (CYQ 筹码分布, 本地算法) — `src/stock/chip.rs`.
4. ✅ **DONE (live-verified)** `stock_fund_flow_120d` (push2his) — `src/stock/fund_flow.rs`.
5. ✅ **DONE (live-verified)** `em_zb_pool` / `em_dt_pool` / `em_yzt_pool` (push2ex) — `src/stock/more.rs`.
6. ✅ **DONE (live-verified)** `cninfo_irm`, `ths_hot_list`, `em_hot_rank`, `em_hot_concept` — `src/news/cninfo_irm.rs` + `src/stock/sentiment.rs`.
7. ✅ **DONE (live-verified)** `eastmoney_reports` → `stock_research_report_em`; `eastmoney_stock_news`
   → `news::stock_news_em`; `eastmoney_global_news` → `stock_info_global_em`.
   (`ths_eps_forecast` **DEFERRED** — WAF 403 on this IP.)
8. ✅ **DONE (live-verified)** `em_stock_monitor`, `em_price_anomaly(_count)`,
   `eastmoney_concept_blocks`, `industry_comparison` → `src/stock/em_signal.rs`;
   `ths_hot_reason` → `src/stock/ths_hot_reason.rs`; `sina_adjust_factor` →
   `src/stock/sina_adjust_factor.rs`; `cninfo_announcements` → `stock_zh_a_disclosure_report_cninfo`.
   (`eastmoney_fund_flow_minute` **DEFERRED** — route 502 on this IP.)

**Priority 2 — reachable but re-verify existing (DONE → live smoke test only):**
`tencent_quote`, `hsgt_realtime`, `eastmoney_stock_info`, `sina_financial_report`,
`sina_option_*`, `em_zt_pool`, `pboc_social_financing`, `nbs_pmi`, `board_fund_flow` (ENRICH).

**Priority 3 — LIVE-UNVERIFIED (blocked `datacenter-web`):** implement against fixtures, tag clearly:
`dragon_tiger_board`, `daily_dragon_tiger`, `lockup_expiry`, `margin_trading`,
`block_trade`, `holder_num_change`, `dividend_history`.

**Deferred (record, do later):** `tdx_client`/F10 (TCP), `baostock_*` (TCP), `iwencai_*` (key),
`sw_industry_history` (retry 508), backups, `eastmoney_fund_flow_minute` (push2 route 502 on this IP),
`ths_eps_forecast` (basic.10jqka WAF 403), `download_pdf` (PDF save-to-path), `eastmoney_industry_reports`,
`ths_limit_up_pool`.

---

## Implemented this round (2026-08-29, live-verified)
| function | module | source | status |
|---|---|---|---|
| `news::telegraph` (`cls_telegraph`) | `src/news/cls.rs` | cls.cn (zero-key md5(sha1) sign) | ✅ live 10 items |
| `stock_fund_flow_120d` | `src/stock/fund_flow.rs` | push2his.eastmoney.com | ✅ live 120 rows |
| `em_zb_pool` | `src/stock/more.rs` | push2ex getTopicZBPool | ✅ live 16 rows |
| `em_dt_pool` | `src/stock/more.rs` | push2ex getTopicDTPool | ✅ live 1 row |
| `em_yzt_pool` | `src/stock/more.rs` | push2ex getYesterdayZTPool | ✅ live 77 rows |
| `baidu_kline_with_ma` | `src/stock/baidu_kline.rs` | finance.pae.baidu.com | ✅ live 2001 rows |
| `chip_distribution` | `src/stock/chip.rs` | local (no network) | ✅ invariants tested |
| `cninfo_irm` | `src/news/cninfo_irm.rs` | irm.cninfo.com.cn (2-step) | ✅ live 5 rows |
| `ths_hot_list` | `src/stock/sentiment.rs` | dq.10jqka.com.cn | ✅ live 100 rows |
| `em_hot_rank` | `src/stock/sentiment.rs` | emappdata + push2 ulist.np | ✅ live 10 rows (merged) |
| `em_hot_concept` | `src/stock/sentiment.rs` | emappdata getHotStockRankList | ✅ live 9 rows |
| `eastmoney_concept_blocks` | `src/stock/em_signal.rs` | push2 slist | ✅ live 27 rows |
| `industry_comparison` | `src/stock/em_signal.rs` | push2 clist (m:90+t:2) | ✅ live 100 rows |
| `em_stock_monitor` | `src/stock/em_signal.rs` | mobappconfig stock_monitor.json | ✅ live 20 rows |
| `em_price_anomaly` | `src/stock/em_signal.rs` | dycalchis price-anomaly/list | ✅ live 8 rows |
| `em_price_anomaly_count` | `src/stock/em_signal.rs` | dycalchis price-anomaly/count | ✅ live 8 rows |
| `ths_hot_reason` | `src/stock/ths_hot_reason.rs` | zx.10jqka getharden | ✅ live 81 rows |
| `sina_adjust_factor` | `src/stock/sina_adjust_factor.rs` | finance.sina.com.cn realstock | ✅ live 33 rows (600519 qfq) |
| `eastmoney_reports` | `stock_research_report_em` | reportapi.eastmoney.com | ✅ live 771 rows |
| `eastmoney_stock_news` | `news::stock_news_em` | search-api-web.eastmoney.com (JSONP) | ✅ live 10 rows |
| `eastmoney_global_news` | `stock_info_global_em` | np-weblist.eastmoney.com | ✅ live 200 rows |
| `cninfo_announcements` | `stock_zh_a_disclosure_report_cninfo` | www.cninfo.com.cn hisAnnouncement/query | ✅ live 62 rows |

Per-function offline fixture tests + a live smoke run (`cargo run --example`) both pass.
Fixtures in `tests/fixtures/`: `cls_telegraph.json`, `stock_fund_flow_120d.json`,
`em_zb_pool.json`, `em_dt_pool.json`, `em_yzt_pool.json`, `baidu_kline.json`,
`cninfo_irm.json`, `ths_hot_list.json`, `em_hot_rank.json`, `em_hot_rank_names.json`,
`em_hot_concept.json`, `em_concept_blocks.json`, `em_industry_comparison.json`,
`em_stock_monitor.json`, `em_price_anomaly.json`, `em_price_anomaly_count.json`,
`ths_hot_reason.json`, `sina_adjust_factor.txt`, `em_reports.json`, `em_stock_news.json`,
`em_global_news.json`.

### Note on `chip_distribution` cross-check
The local algorithm is verified by structural invariants (profit_ratio∈[0,1],
cost_90⊇cost_70, concentration_90≥concentration_70, finite peak). A numeric
cross-check against the skill's published 600519 reference (profit_ratio≈15.44%,
avg_cost≈1371.31 for 2026-02-01→2026-08-18) is deferred until a turnover-bearing
daily-K-line source is wired (currently `baostock_*` is TCP-deferred); the math is
identical to the skill's `_triangular_weights` + turnover-decay recurrence.
