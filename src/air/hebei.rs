//! 河北省空气质量预报信息发布系统 (Hebei air-quality forecast). Ports
//! `akshare/air/air_hebei.py`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | `air_quality_hebei` | `air/air_hebei.py:23` | 未来 6 天逐小时 AQI, XML feed |
//!
//! ## DEFERRED
//!
//! `air_quality_hebei` (`air/air_hebei.py:23`) — plain `GET
//! `http://218.11.10.130:8080/api/hour/130000.xml`` but the response is **XML**,
//! parsed in the source via `BeautifulSoup(r.content, features="xml")`
//! (`City`/`Pointer`/`Poll` elements). The allowed dependency set has no XML or
//! HTML parser (`reqwest` + `serde_json` only handle JSON), so the feed cannot be
//! parsed without adding a crate. DEFER until an XML/HTML parser dependency is
//! approved.
