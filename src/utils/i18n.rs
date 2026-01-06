use std::sync::OnceLock;

static CURRENT_LANGUAGE: OnceLock<String> = OnceLock::new();

/// Initialize and set default language
pub fn set_language() {
    let lang = detect_system_language()
        .or_else(load_saved_language)
        .unwrap_or_else(|| "en".to_string());

    CURRENT_LANGUAGE.set(lang.clone()).ok();
    rust_i18n::set_locale(&lang);

    tracing::info!("Language set to: {}", lang);
}

/// Change language at runtime
#[allow(dead_code)]
pub fn change_language(lang: &str) {
    rust_i18n::set_locale(lang);
    save_language_preference(lang);
    tracing::info!("Language changed to: {}", lang);
}

/// Get current language
pub fn current_language() -> String {
    CURRENT_LANGUAGE
        .get()
        .cloned()
        .unwrap_or_else(|| rust_i18n::locale().to_string())
}

/// Detect system language from environment
fn detect_system_language() -> Option<String> {
    std::env::var("LANG").ok().map(|lang| {
        // Parse LANG environment variable (e.g., "zh_CN.UTF-8" -> "zh-CN")
        if lang.starts_with("zh") || lang.contains("zh_CN") || lang.contains("zh_Hans") {
            "zh-CN".to_string()
        } else {
            "en".to_string()
        }
    })
}

/// Load saved language preference from config
fn load_saved_language() -> Option<String> {
    // TODO: Load from config file
    // This will be implemented when we add the config service
    None
}

/// Save language preference to config
#[allow(dead_code)]
fn save_language_preference(lang: &str) {
    // TODO: Save to config file
    // This will be implemented when we add the config service
    tracing::debug!("Saving language preference: {}", lang);
}

/// Get available languages
#[allow(dead_code)]
pub fn available_languages() -> Vec<(&'static str, &'static str)> {
    vec![("zh-CN", "简体中文"), ("en", "English")]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_languages() {
        let langs = available_languages();
        assert_eq!(langs.len(), 2);
        assert!(langs.iter().any(|(code, _)| *code == "zh-CN"));
        assert!(langs.iter().any(|(code, _)| *code == "en"));
    }
}
