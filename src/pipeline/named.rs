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
    entries: Vec<MluEntry>,
    pool: Vec<u8>,
}

impl Default for Mlu {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Internal helpers ----

fn parse_language_code(s: &str) -> LanguageCode {
    let b = s.as_bytes();
    LanguageCode([
        b.first().copied().unwrap_or(0),
        b.get(1).copied().unwrap_or(0),
    ])
}

fn parse_country_code(s: &str) -> CountryCode {
    let b = s.as_bytes();
    CountryCode([
        b.first().copied().unwrap_or(0),
        b.get(1).copied().unwrap_or(0),
    ])
}

fn utf8_to_utf16be(s: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(s.len() * 2);
    for code_unit in s.encode_utf16() {
        buf.extend_from_slice(&code_unit.to_be_bytes());
    }
    buf
}

fn utf16be_to_string(bytes: &[u8]) -> String {
    let iter = bytes
        .chunks_exact(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]));
    char::decode_utf16(iter)
        .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
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
    pub fn set_ascii(&mut self, lang: &str, country: &str, text: &str) -> bool {
        if !text.is_ascii() {
            return false;
        }
        self.set_utf8(lang, country, text)
    }

    /// Set UTF-8 text for a language/country pair.
    pub fn set_utf8(&mut self, lang: &str, country: &str, text: &str) -> bool {
        let language = parse_language_code(lang);
        let ctry = parse_country_code(country);
        let encoded = utf8_to_utf16be(text);

        // Overwrite existing entry for same language/country
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|e| e.language == language && e.country == ctry)
        {
            let offset = self.pool.len();
            self.pool.extend_from_slice(&encoded);
            entry.offset = offset;
            entry.len = encoded.len();
        } else {
            let offset = self.pool.len();
            self.pool.extend_from_slice(&encoded);
            self.entries.push(MluEntry {
                language,
                country: ctry,
                offset,
                len: encoded.len(),
            });
        }
        true
    }

    /// Get text as ASCII for a language/country pair.
    ///
    /// Uses 3-level fallback: exact match → language-only → first entry.
    /// Non-ASCII characters are replaced with `'?'`.
    /// Returns `None` if the MLU has no entries.
    pub fn get_ascii(&self, lang: &str, country: &str) -> Option<String> {
        let s = self.get_utf8(lang, country)?;
        Some(
            s.chars()
                .map(|c| if c.is_ascii() { c } else { '?' })
                .collect(),
        )
    }

    /// Get text as UTF-8 for a language/country pair.
    ///
    /// Uses 3-level fallback: exact match → language-only → first entry.
    /// Returns `None` if the MLU has no entries.
    pub fn get_utf8(&self, lang: &str, country: &str) -> Option<String> {
        let idx = self.find_best(lang, country)?;
        let entry = &self.entries[idx];
        Some(utf16be_to_string(
            &self.pool[entry.offset..entry.offset + entry.len],
        ))
    }

    /// Number of language/country entries.
    pub fn translations_count(&self) -> usize {
        self.entries.len()
    }

    /// Get language/country codes for entry at `index`.
    pub fn translation_codes(&self, index: usize) -> Option<(LanguageCode, CountryCode)> {
        self.entries.get(index).map(|e| (e.language, e.country))
    }

    /// 3-level fallback search: exact → language-only → first entry.
    fn find_best(&self, lang: &str, country: &str) -> Option<usize> {
        if self.entries.is_empty() {
            return None;
        }
        let language = parse_language_code(lang);
        let ctry = parse_country_code(country);

        // 1. Exact match
        if let Some(i) = self
            .entries
            .iter()
            .position(|e| e.language == language && e.country == ctry)
        {
            return Some(i);
        }

        // 2. Language-only match
        if let Some(i) = self.entries.iter().position(|e| e.language == language) {
            return Some(i);
        }

        // 3. First entry
        Some(0)
    }
}

// ============================================================================
// Named Color
// ============================================================================

use crate::types::MAX_CHANNELS;

/// A single named color entry.
///
/// C版: `_cmsNAMEDCOLOR`
#[derive(Debug, Clone)]
pub struct NamedColor {
    pub name: String,
    pub pcs: [u16; 3],
    pub colorant: [u16; MAX_CHANNELS],
}

/// Named color palette.
///
/// C版: `cmsNAMEDCOLORLIST`
#[derive(Debug, Clone)]
pub struct NamedColorList {
    colors: Vec<NamedColor>,
    colorant_count: u32,
    prefix: String,
    suffix: String,
}

impl NamedColorList {
    /// Create a new named color list.
    ///
    /// `colorant_count` is the number of device colorant channels (must be <= MAX_CHANNELS).
    /// Returns `None` if `colorant_count` exceeds `MAX_CHANNELS`.
    pub fn new(colorant_count: u32, prefix: &str, suffix: &str) -> Option<Self> {
        if colorant_count as usize > MAX_CHANNELS {
            return None;
        }
        Some(Self {
            colors: Vec::new(),
            colorant_count,
            prefix: prefix.to_string(),
            suffix: suffix.to_string(),
        })
    }

    /// Append a named color.
    ///
    /// `colorant` slice length must match the list's `colorant_count`.
    pub fn append(&mut self, name: &str, pcs: &[u16; 3], colorant: Option<&[u16]>) -> bool {
        let mut device = [0u16; MAX_CHANNELS];
        if let Some(c) = colorant {
            let n = c.len().min(self.colorant_count as usize).min(MAX_CHANNELS);
            device[..n].copy_from_slice(&c[..n]);
        }
        self.colors.push(NamedColor {
            name: name.to_string(),
            pcs: *pcs,
            colorant: device,
        });
        true
    }

    /// Number of colors in the list.
    pub fn count(&self) -> usize {
        self.colors.len()
    }

    /// Get color info at `index`.
    pub fn info(&self, index: usize) -> Option<&NamedColor> {
        self.colors.get(index)
    }

    /// Find a color by name (case-sensitive). Returns its index.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.colors.iter().position(|c| c.name == name)
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn suffix(&self) -> &str {
        &self.suffix
    }

    pub fn colorant_count(&self) -> u32 {
        self.colorant_count
    }
}

// ============================================================================
// Profile Sequence Description
// ============================================================================

use crate::types::TechnologySignature;

/// A single entry in a profile sequence description.
///
/// C版: `cmsPSEQDESC`
#[derive(Debug, Clone)]
pub struct ProfileSequenceDescEntry {
    pub device_mfg: u32,
    pub device_model: u32,
    pub attributes: u64,
    pub technology: Option<TechnologySignature>,
    pub manufacturer: Mlu,
    pub model: Mlu,
}

/// Profile sequence description — ordered list of profiles used in a transform.
///
/// C版: `cmsSEQ`
#[derive(Debug, Clone)]
pub struct ProfileSequenceDesc {
    #[allow(dead_code)]
    entries: Vec<ProfileSequenceDescEntry>,
}

impl ProfileSequenceDesc {
    /// Create a new profile sequence description with `n` empty entries.
    pub fn new(_n: usize) -> Self {
        todo!()
    }

    pub fn len(&self) -> usize {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        todo!()
    }

    pub fn get(&self, _index: usize) -> Option<&ProfileSequenceDescEntry> {
        todo!()
    }

    pub fn get_mut(&mut self, _index: usize) -> Option<&mut ProfileSequenceDescEntry> {
        todo!()
    }
}

// ============================================================================
// Dictionary
// ============================================================================

/// A single dictionary entry.
///
/// C版: `cmsDICTentry`
#[derive(Debug, Clone)]
pub struct DictEntry {
    pub name: String,
    pub value: Option<String>,
    pub display_name: Option<Mlu>,
    pub display_value: Option<Mlu>,
}

/// Metadata dictionary — key/value pairs with optional MLU display strings.
///
/// C版: `cmsDICT`
#[derive(Debug, Clone)]
pub struct Dict {
    #[allow(dead_code)]
    entries: Vec<DictEntry>,
}

impl Default for Dict {
    fn default() -> Self {
        Self::new()
    }
}

impl Dict {
    pub fn new() -> Self {
        todo!()
    }

    pub fn add(
        &mut self,
        _name: &str,
        _value: Option<&str>,
        _display_name: Option<&Mlu>,
        _display_value: Option<&Mlu>,
    ) -> bool {
        todo!()
    }

    pub fn iter(&self) -> impl Iterator<Item = &DictEntry> {
        todo!();
        #[allow(unreachable_code)]
        std::iter::empty()
    }

    pub fn len(&self) -> usize {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]

    fn mlu_new_is_empty() {
        let mlu = Mlu::new();
        assert_eq!(mlu.translations_count(), 0);
    }

    #[test]

    fn mlu_set_get_ascii() {
        let mut mlu = Mlu::new();
        assert!(mlu.set_ascii("en", "US", "Hello"));
        assert_eq!(mlu.get_ascii("en", "US"), Some("Hello".to_string()));
    }

    #[test]

    fn mlu_set_get_utf8() {
        let mut mlu = Mlu::new();
        assert!(mlu.set_utf8("ja", "JP", "こんにちは"));
        assert_eq!(mlu.get_utf8("ja", "JP"), Some("こんにちは".to_string()));
    }

    #[test]

    fn mlu_ascii_retrieved_as_utf8() {
        let mut mlu = Mlu::new();
        assert!(mlu.set_ascii("en", "US", "Hello"));
        assert_eq!(mlu.get_utf8("en", "US"), Some("Hello".to_string()));
    }

    #[test]

    fn mlu_set_ascii_rejects_non_ascii() {
        let mut mlu = Mlu::new();
        assert!(!mlu.set_ascii("ja", "JP", "こんにちは"));
        assert_eq!(mlu.translations_count(), 0);
    }

    #[test]

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

    fn mlu_overwrite_same_entry() {
        let mut mlu = Mlu::new();
        assert!(mlu.set_utf8("en", "US", "Hello"));
        assert!(mlu.set_utf8("en", "US", "World"));
        assert_eq!(mlu.translations_count(), 1);
        assert_eq!(mlu.get_utf8("en", "US"), Some("World".to_string()));
    }

    #[test]

    fn mlu_fallback_language_only() {
        let mut mlu = Mlu::new();
        assert!(mlu.set_utf8("en", "US", "Color"));
        // No exact match for en-GB, but language "en" matches
        assert_eq!(mlu.get_utf8("en", "GB"), Some("Color".to_string()));
    }

    #[test]

    fn mlu_fallback_first_entry() {
        let mut mlu = Mlu::new();
        assert!(mlu.set_utf8("en", "US", "Hello"));
        assert!(mlu.set_utf8("ja", "JP", "こんにちは"));
        // No match for fr-FR at all, falls back to first entry
        assert_eq!(mlu.get_utf8("fr", "FR"), Some("Hello".to_string()));
    }

    #[test]

    fn mlu_empty_returns_none() {
        let mlu = Mlu::new();
        assert_eq!(mlu.get_utf8("en", "US"), None);
        assert_eq!(mlu.get_ascii("en", "US"), None);
    }

    #[test]

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

    fn mlu_clone() {
        let mut mlu = Mlu::new();
        assert!(mlu.set_utf8("en", "US", "Hello"));

        let mlu2 = mlu.clone();
        assert_eq!(mlu2.get_utf8("en", "US"), Some("Hello".to_string()));
        assert_eq!(mlu2.translations_count(), 1);
    }

    // --- NamedColorList tests ---

    #[test]
    fn named_color_list_new() {
        let list = NamedColorList::new(3, "prefix", "suffix").unwrap();
        assert_eq!(list.count(), 0);
        assert_eq!(list.colorant_count(), 3);
        assert_eq!(list.prefix(), "prefix");
        assert_eq!(list.suffix(), "suffix");
    }

    #[test]
    fn named_color_list_new_rejects_too_many_channels() {
        assert!(NamedColorList::new((MAX_CHANNELS + 1) as u32, "", "").is_none());
    }

    #[test]
    fn named_color_list_append_and_count() {
        let mut list = NamedColorList::new(3, "", "").unwrap();
        let pcs = [1000, 2000, 3000];
        let colorant = [100, 200, 300];

        assert!(list.append("Red", &pcs, Some(&colorant)));
        assert!(list.append("Green", &pcs, Some(&colorant)));
        assert_eq!(list.count(), 2);
    }

    #[test]
    fn named_color_list_info() {
        let mut list = NamedColorList::new(3, "", "").unwrap();
        let pcs = [1000, 2000, 3000];
        let colorant = [100, 200, 300];
        list.append("Red", &pcs, Some(&colorant));

        let color = list.info(0).unwrap();
        assert_eq!(color.name, "Red");
        assert_eq!(color.pcs, pcs);
        assert_eq!(color.colorant[..3], colorant);

        assert!(list.info(1).is_none());
    }

    #[test]
    fn named_color_list_find() {
        let mut list = NamedColorList::new(3, "", "").unwrap();
        let pcs = [0, 0, 0];
        list.append("Red", &pcs, None);
        list.append("Green", &pcs, None);
        list.append("Blue", &pcs, None);

        assert_eq!(list.find("Green"), Some(1));
        assert_eq!(list.find("Blue"), Some(2));
        assert_eq!(list.find("Yellow"), None);
    }

    #[test]
    fn named_color_list_append_none_colorant() {
        let mut list = NamedColorList::new(4, "", "").unwrap();
        let pcs = [500, 600, 700];
        assert!(list.append("Test", &pcs, None));

        let color = list.info(0).unwrap();
        assert_eq!(color.pcs, pcs);
        assert_eq!(color.colorant, [0u16; MAX_CHANNELS]);
    }

    #[test]
    fn named_color_list_clone() {
        let mut list = NamedColorList::new(3, "pfx", "sfx").unwrap();
        let pcs = [100, 200, 300];
        list.append("Color1", &pcs, None);

        let list2 = list.clone();
        assert_eq!(list2.count(), 1);
        assert_eq!(list2.prefix(), "pfx");
        assert_eq!(list2.info(0).unwrap().name, "Color1");
    }

    // --- ProfileSequenceDesc tests ---

    #[test]
    #[ignore = "not yet implemented"]
    fn profile_sequence_desc_new() {
        let seq = ProfileSequenceDesc::new(3);
        assert_eq!(seq.len(), 3);
        assert!(!seq.is_empty());

        let empty = ProfileSequenceDesc::new(0);
        assert!(empty.is_empty());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn profile_sequence_desc_get_and_mutate() {
        let mut seq = ProfileSequenceDesc::new(2);

        let entry = seq.get_mut(0).unwrap();
        entry.device_mfg = 0x4150504C; // 'APPL'
        entry.manufacturer.set_ascii("en", "US", "Apple");

        let entry = seq.get(0).unwrap();
        assert_eq!(entry.device_mfg, 0x4150504C);
        assert_eq!(
            entry.manufacturer.get_ascii("en", "US"),
            Some("Apple".to_string())
        );

        assert!(seq.get(2).is_none());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn profile_sequence_desc_clone() {
        let mut seq = ProfileSequenceDesc::new(1);
        seq.get_mut(0).unwrap().device_model = 42;

        let seq2 = seq.clone();
        assert_eq!(seq2.get(0).unwrap().device_model, 42);
    }

    // --- Dict tests ---

    #[test]
    #[ignore = "not yet implemented"]
    fn dict_new_is_empty() {
        let dict = Dict::new();
        assert_eq!(dict.len(), 0);
        assert!(dict.is_empty());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn dict_add_and_iterate() {
        let mut dict = Dict::new();
        assert!(dict.add("key1", Some("value1"), None, None));
        assert!(dict.add("key2", Some("value2"), None, None));
        assert_eq!(dict.len(), 2);

        let entries: Vec<&DictEntry> = dict.iter().collect();
        assert_eq!(entries[0].name, "key1");
        assert_eq!(entries[0].value.as_deref(), Some("value1"));
        assert_eq!(entries[1].name, "key2");
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn dict_add_with_display_names() {
        let mut dict = Dict::new();
        let mut display_name = Mlu::new();
        display_name.set_utf8("en", "US", "Display Key");

        assert!(dict.add("key", Some("val"), Some(&display_name), None));

        let entry = dict.iter().next().unwrap();
        assert_eq!(
            entry.display_name.as_ref().unwrap().get_utf8("en", "US"),
            Some("Display Key".to_string())
        );
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn dict_add_none_value() {
        let mut dict = Dict::new();
        assert!(dict.add("key_only", None, None, None));

        let entry = dict.iter().next().unwrap();
        assert_eq!(entry.name, "key_only");
        assert!(entry.value.is_none());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn dict_clone() {
        let mut dict = Dict::new();
        dict.add("k", Some("v"), None, None);

        let dict2 = dict.clone();
        assert_eq!(dict2.len(), 1);
        assert_eq!(dict2.iter().next().unwrap().name, "k");
    }
}
