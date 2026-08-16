//! Currency / FX domain.
//!
//! Declares the leaf module that ports `akshare/currency/*` and the FX helpers
//! listed in the porting assignment (`akshare/fx/*`).
pub mod api;

// --- currency "gaps" port: SAFE / Sina BOC HTML-table rates ---
pub mod currency_html_gaps;
