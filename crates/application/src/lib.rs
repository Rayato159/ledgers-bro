//! Use cases and ports. Adapters depend on this crate, never the reverse.
mod account_deletion;
mod accounting_export;
mod cashflow;
mod credit;
pub use credit::*;
mod entry_text;
mod error;
mod local_model;
pub mod model_contract;
mod model_resolution;
pub use local_model::*;
pub use model_resolution::*;
mod ports;
mod quick_entry;
mod receipt;
mod receipt_input;
mod receipt_lines;
mod report;
mod service;

pub use account_deletion::*;
pub use accounting_export::*;
pub use cashflow::*;
pub use entry_text::*;
pub use error::*;
pub use ports::*;
pub use quick_entry::*;
pub use receipt::*;
pub use receipt_input::*;
pub use report::*;
pub use service::*;

mod tax;
pub use tax::*;
mod crypto;
mod tax_income;
pub use crypto::*;
mod data_transfer;
mod profiles;
pub use data_transfer::*;
pub use profiles::*;
pub use tax_income::*;

mod recurring;
pub use recurring::*;
mod receivable;
pub use receivable::*;
mod prompt;
pub use prompt::*;
mod prompt_text;
pub use prompt_text::*;

mod receipt_prompt;
pub use receipt_prompt::*;

mod currency;
pub use currency::normalize_prompt_currency;

mod preferences;
pub use preferences::*;

mod model_catalog;
pub use model_catalog::*;
