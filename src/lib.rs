// Load i18n translations
rust_i18n::i18n!("locales", fallback = "en");

pub mod models;
pub mod services;
pub mod state;
pub mod utils;
