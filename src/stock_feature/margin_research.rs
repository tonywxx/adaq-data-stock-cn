//! `stock_feature` **融资融券 (margin/short)** and **研报 (research report)**
//! endpoints.
//!
//! These families are not yet ported in this wave. They are tracked for a
//! dedicated `stock`-focused port wave (see `docs/MAPPING.md` for the running
//! inventory). Representative akshare sources:
//!
//! * `stock_margin_em.py` — Eastmoney 融资融券 (margin trading) boards;
//!   mostly `datacenter-web` JSON, so largely **tractable** in a later wave.
//! * `stock_research_report_em.py` / `stock_report_em.py` — Eastmoney research
//!   reports; mixed JSON + HTML, tractability per-endpoint.
//!
//! Until that wave lands, the functions remain neither DONE nor formally
//! DEFERRED; they are simply *unported*. They are enumerated here so the
//! `stock_feature` module surface is explicit and nothing is silently dropped.
