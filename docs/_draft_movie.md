# Draft: movie functions port (yien artist / video / weekly)

Triage of 6 akshare movie functions for porting to `adaq-data-stock-cn`.

Template read: `src/economic/macro_econ.rs`, `src/core/client.rs`, `src/core/error.rs`.
Existing ports inspected: `src/alt/movie.rs`, `src/alt/movie_yien.rs`.

akshare source root: `akshare/akshare/movie/`.

## Result

| akshare fn | rust port | source:line | status | reason |
| --- | --- | --- | --- | --- |
| business_value_artist |  | akshare/movie/artist_yien.py:65 | DEFERRED | POST `https://www.endata.com.cn/API/GetData.ashx` then `json.loads(decrypt(r.text))`; `decrypt` runs the bundled `jm.js` via `py_mini_racer` (`webInstace.shell`) — JS execution required, not a clean JSON endpoint. |
| online_value_artist |  | akshare/movie/artist_yien.py:103 | DEFERRED | Same `py_mini_racer` `decrypt()` JS path over `GetData.ashx` response — not a clean JSON endpoint. |
| movie_boxoffice_weekly |  | akshare/movie/movie_yien.py:340 | DEFERRED | Function body calls `_raise_week_permission_error(interface_name="movie_boxoffice_weekly")` — upstream permission gate, akshare itself raises; no deterministic endpoint to call. |
| movie_boxoffice_cinema_weekly |  | akshare/movie/movie_yien.py:642 | DEFERRED | Same `_raise_week_permission_error` upstream gate as `movie_boxoffice_weekly`; no deterministic endpoint. |
| video_tv |  | akshare/movie/video_yien.py:65 | DEFERRED | POST `GetData.ashx` then `decrypt(r.text)` via `py_mini_racer` (`jm.js`) — JS execution required. |
| video_variety_show |  | akshare/movie/video_yien.py:96 | DEFERRED | Same `py_mini_racer` `decrypt()` JS path over `GetData.ashx` response. |

## Notes

- 0 ported, 6 deferred. No `src/alt/wv_movie_more.rs` created (no plain-JSON GET identified).
- The four `GetData.ashx` endpoints (`business_value_artist`, `online_value_artist`, `video_tv`, `video_variety_show`) share one shape: POST form data, then `decrypt()` with the `jm.js` obfuscator. Per ADR-0005 these are to be reversed to pure Rust (no JS engine embedded, e.g. `boa`/`rusty_v8`) — currently DEFERRED.
- The two weekly box-office fns are hard-gated inside akshare and never reach the network.
