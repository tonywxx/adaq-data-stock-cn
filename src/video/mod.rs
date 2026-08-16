//! 艺恩 (endata / yien) 视频放映数据 — 电视剧集与综艺节目.
//!
//! Ports `akshare/movie/video_yien.py`. This module owns two akshare functions:
//! `video_tv` and `video_variety_show`.
//!
//! | Rust function | akshare source | notes |
//! |---|---|---|
//! | — | `video_yien.py:65` (`video_tv`) | DEFERRED — see below |
//! | — | `video_yien.py:96` (`video_variety_show`) | DEFERRED — see below |
//!
//! ## DEFERRED
//!
//! - `video_tv` (`video_yien.py:65`) — **JS-signed / client-side decrypt**. POSTs
//!   `https://www.endata.com.cn/API/GetData.ashx` with
//!   `{"tvType": 2, "MethodName": "BoxOffice_GetTvData_PlayIndexRank"}`, then
//!   decrypts the encrypted response via `decrypt()` — which evaluates the local
//!   `jm.js` with `py_mini_racer` and calls `webInstace.shell(origin_data)`. The
//!   response is not usable JSON without running that JS decryption. Per porting
//!   guide (rule 4: JS-signed params / client-side decrypt → DEFER). No
//!   implementation, no fixture.
//! - `video_variety_show` (`video_yien.py:96`) — **same reason**: identical POST
//!   to `GetData.ashx` with `tvType=8`, response likewise decrypted by `jm.js`
//!   (`webInstace.shell`). DEFERRED.
