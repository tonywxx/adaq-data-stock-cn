//! Macro / economic data (akshare `economic` / `cal` packages).

pub mod china;
pub mod extra;
pub mod macro2;

pub use china::{
    macro_china_cpi, macro_china_gdp, macro_china_money_supply, macro_china_ppi, ChinaCpi,
    ChinaGdp, ChinaMoneySupply, ChinaPpi,
};
