//! Dioxus presentation. No SQL, filesystem access, model calls, or tax formulas.
mod account_deletion;
mod app;
mod profiles;
pub use profiles::LoginRoot;
mod artwork;
mod batch_entry;
mod cashflow;
mod components;
mod data_transfer;
mod entry;
mod export;
mod gateway;
mod model;
mod overview;
mod pages;
mod pie;
mod receipt;
mod receipt_editor;
mod state;
mod tax;
mod typography;
mod voice;

pub use app::App;
pub use gateway::{ArtAssets, Gateway, HostInfo, ReceiptScan, UiFuture, UiGateway};
pub use voice::{VoiceEvent, VoiceSessionId};

mod recurring;

mod receivables;
mod recurring_picker;

mod prompt_review;

mod prompt_examples;

mod i18n;

mod crypto;
mod currency;

mod navigation;

mod credit_payment;
mod debt_visuals;
mod income_tax;
mod theme;
