//! Stock fundamentals / financials.
//!
//! Eastmoney report-based financial statements and main indicators, ported
//! from akshare. All endpoints use Eastmoney's `datacenter-web` REST API
//! (no JS signing, no HTML scrape — see `eastmoney.rs`).

pub mod eastmoney;
pub mod registration;

pub use eastmoney::{
    stock_balance_sheet_by_report_em, stock_cash_flow_sheet_by_report_em,
    stock_financial_analysis_indicator_em, stock_profit_sheet_by_report_em, BalanceSheetRow,
    CashFlowSheetRow, FinancialIndicatorRow, ProfitSheetRow,
};
