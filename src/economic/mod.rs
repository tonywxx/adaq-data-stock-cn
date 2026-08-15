//! Macro / economic data (akshare `economic` / `cal` packages).

pub mod china;
pub mod extra;
pub mod macro2;
pub mod macro_intl;
pub mod macro_china2;
pub mod macro_usa;
pub mod macro_bank;
pub mod macro_nbs_euro;

pub use china::{
    macro_china_cpi, macro_china_gdp, macro_china_money_supply, macro_china_ppi, ChinaCpi,
    ChinaGdp, ChinaMoneySupply, ChinaPpi,
};
