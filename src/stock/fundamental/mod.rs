//! Stock fundamentals / financials.
//!
//! Eastmoney report-based financial statements and main indicators, ported
//! from akshare. All endpoints use Eastmoney's `datacenter-web` REST API
//! (no JS signing, no HTML scrape — see `eastmoney.rs`).

pub mod eastmoney;
pub mod finance_more;
pub mod registration;
pub mod more;

pub use eastmoney::{
    BalanceSheetRow, CashFlowSheetRow, FinancialIndicatorRow, ProfitSheetRow,
    stock_balance_sheet_by_report_em, stock_cash_flow_sheet_by_report_em,
    stock_financial_analysis_indicator_em, stock_profit_sheet_by_report_em,
};

pub mod wv_fund_misc;
