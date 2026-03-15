/// ISO 639-1 language code (2 bytes, e.g. `*b"en"`, `*b"ja"`).
///
/// C版: `_cmsMLUentry.Language` (packed u16)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageCode(pub [u8; 2]);

/// ISO 3166-1 country code (2 bytes, e.g. `*b"US"`, `*b"JP"`).
///
/// C版: `_cmsMLUentry.Country` (packed u16)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountryCode(pub [u8; 2]);

#[allow(dead_code)] // Fields used in implementation commit
#[derive(Debug, Clone)]
struct MluEntry {
    language: LanguageCode,
    country: CountryCode,
    offset: usize,
    len: usize,
}

/// Multi Local Unicode text.
///
/// Stores multilingual strings as UTF-16BE byte pools per ICC spec.
/// Public API accepts and returns UTF-8 (`&str` / `String`).
///
/// C版: `cmsMLU`
#[derive(Debug, Clone)]
pub struct Mlu {
    #[allow(dead_code)] // Used in implementation commit
    entries: Vec<MluEntry>,
    #[allow(dead_code)] // Used in implementation commit
    pool: Vec<u8>,
}

impl Default for Mlu {
    fn default() -> Self {
        Self::new()
    }
}

impl Mlu {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            pool: Vec::new(),
        }
    }

    /// Set ASCII text for a language/country pair.
    ///
    /// Returns `false` if `text` contains non-ASCII characters.
    pub fn set_ascii(&mut self, _lang: &str, _country: &str, _text: &str) -> bool {
        todo!()
    }

    /// Set UTF-8 text for a language/country pair.
    pub fn set_utf8(&mut self, _lang: &str, _country: &str, _text: &str) -> bool {
        todo!()
    }

    /// Get text as ASCII for a language/country pair.
    ///
    /// Uses 3-level fallback: exact match → language-only → first entry.
    /// Non-ASCII characters are replaced with `'?'`.
    /// Returns `None` if the MLU has no entries.
    pub fn get_ascii(&self, _lang: &str, _country: &str) -> Option<String> {
        todo!()
    }

    /// Get text as UTF-8 for a language/country pair.
    ///
    /// Uses 3-level fallback: exact match → language-only → first entry.
    /// Returns `None` if the MLU has no entries.
    pub fn get_utf8(&self, _lang: &str, _country: &str) -> Option<String> {
        todo!()
    }

    /// Number of language/country entries.
    pub fn translations_count(&self) -> usize {
        todo!()
    }

    /// Get language/country codes for entry at `index`.
    pub fn translation_codes(&self, _index: usize) -> Option<(LanguageCode, CountryCode)> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "not yet implemented"]
    fn mlu_new_is_empty() {
        let mlu = Mlu::new();
        assert_eq!(mlu.translations_count(), 0);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mlu_set_get_ascii() {
        let mut mlu = Mlu::new();
        assert!(mlu.set_ascii("en", "US", "Hello"));
        assert_eq!(mlu.get_ascii("en", "US"), Some("Hello".to_string()));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mlu_set_get_utf8() {
        let mut mlu = Mlu::new();
        assert!(mlu.set_utf8("ja", "JP", "こんにちは"));
        assert_eq!(mlu.get_utf8("ja", "JP"), Some("こんにちは".to_string()));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mlu_ascii_retrieved_as_utf8() {
        let mut mlu = Mlu::new();
        assert!(mlu.set_ascii("en", "US", "Hello"));
        assert_eq!(mlu.get_utf8("en", "US"), Some("Hello".to_string()));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mlu_set_ascii_rejects_non_ascii() {
        let mut mlu = Mlu::new();
        assert!(!mlu.set_ascii("ja", "JP", "こんにちは"));
        assert_eq!(mlu.translations_count(), 0);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mlu_multiple_languages() {
        let mut mlu = Mlu::new();
        assert!(mlu.set_utf8("en", "US", "Hello"));
        assert!(mlu.set_utf8("ja", "JP", "こんにちは"));
        assert!(mlu.set_utf8("de", "DE", "Hallo"));

        assert_eq!(mlu.translations_count(), 3);
        assert_eq!(mlu.get_utf8("en", "US"), Some("Hello".to_string()));
        assert_eq!(mlu.get_utf8("ja", "JP"), Some("こんにちは".to_string()));
        assert_eq!(mlu.get_utf8("de", "DE"), Some("Hallo".to_string()));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mlu_overwrite_same_entry() {
        let mut mlu = Mlu::new();
        assert!(mlu.set_utf8("en", "US", "Hello"));
        assert!(mlu.set_utf8("en", "US", "World"));
        assert_eq!(mlu.translations_count(), 1);
        assert_eq!(mlu.get_utf8("en", "US"), Some("World".to_string()));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mlu_fallback_language_only() {
        let mut mlu = Mlu::new();
        assert!(mlu.set_utf8("en", "US", "Color"));
        // No exact match for en-GB, but language "en" matches
        assert_eq!(mlu.get_utf8("en", "GB"), Some("Color".to_string()));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mlu_fallback_first_entry() {
        let mut mlu = Mlu::new();
        assert!(mlu.set_utf8("en", "US", "Hello"));
        assert!(mlu.set_utf8("ja", "JP", "こんにちは"));
        // No match for fr-FR at all, falls back to first entry
        assert_eq!(mlu.get_utf8("fr", "FR"), Some("Hello".to_string()));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mlu_empty_returns_none() {
        let mlu = Mlu::new();
        assert_eq!(mlu.get_utf8("en", "US"), None);
        assert_eq!(mlu.get_ascii("en", "US"), None);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mlu_translation_codes() {
        let mut mlu = Mlu::new();
        assert!(mlu.set_utf8("en", "US", "Hello"));
        assert!(mlu.set_utf8("ja", "JP", "こんにちは"));

        let (lang, country) = mlu.translation_codes(0).unwrap();
        assert_eq!(lang, LanguageCode(*b"en"));
        assert_eq!(country, CountryCode(*b"US"));

        let (lang, country) = mlu.translation_codes(1).unwrap();
        assert_eq!(lang, LanguageCode(*b"ja"));
        assert_eq!(country, CountryCode(*b"JP"));

        assert!(mlu.translation_codes(2).is_none());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn mlu_clone() {
        let mut mlu = Mlu::new();
        assert!(mlu.set_utf8("en", "US", "Hello"));

        let mlu2 = mlu.clone();
        assert_eq!(mlu2.get_utf8("en", "US"), Some("Hello".to_string()));
        assert_eq!(mlu2.translations_count(), 1);
    }
}
