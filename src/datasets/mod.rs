//! File-path helpers ported (or documented) from `akshare/datasets.py`.
//!
//! ## DEFERRED — akshare-internal bundled-asset loaders, not data APIs
//!
//! All three functions merely resolve a `pathlib.Path` to a file shipped
//! *inside the akshare package* (`akshare/data/*.json`, `*.js`, `*.zip`) via
//! `importlib.resources`. They do no downloading and return filesystem paths,
//! not data. There is no network or parsing work to port; they are akshare's
//! own asset plumbing. Recorded here (and in `docs/MAPPING.md`) for
//! completeness.
//!
//! * `get_ths_js(file="ths.js")` (`datasets.py:12`) — path to the THS
//!   JavaScript used by `py_mini_racer`-signed endpoints.
//! * `get_crypto_info_csv(file="crypto_info.zip")` (`datasets.py:23`) — path to
//!   the bundled crypto-info zip.
//! * `get_registry_json(file="interfaces.json")` (`datasets.py:34`) — path to
//!   the bundled interface registry (consumed by `registry.py`).
