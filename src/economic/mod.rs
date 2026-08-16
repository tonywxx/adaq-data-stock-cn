//! Macro / economic data (akshare `economic` / `cal` packages).

pub mod china;
pub mod extra;
pub mod macro2;
pub mod macro_bank;
pub mod macro_china2;
pub mod macro_intl;
pub mod macro_nbs_euro;
pub mod macro_usa;
pub mod macro_china3;

pub use china::{
    ChinaCpi, ChinaGdp, ChinaMoneySupply, ChinaPpi, macro_china_cpi, macro_china_gdp,
    macro_china_money_supply, macro_china_ppi,
};
pub mod macro_china_more;
pub mod macro_misc;
pub mod macro_usa_more;
