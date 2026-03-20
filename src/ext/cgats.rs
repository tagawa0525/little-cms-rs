// ============================================================================
// CGATS / IT8 text format parser (C版: cmscgats.c)
// ============================================================================
//
// Reads and writes CGATS.17 / IT8.7 color measurement data files.
// Supports header properties, data format definitions, and data tables.

use crate::context::{CmsError, ErrorCode};
use std::collections::BTreeMap;

// ============================================================================
// Data structures
// ============================================================================

/// A single table within an IT8 stream.
#[derive(Clone, Debug)]
struct Table {
    sheet_type: String,
    properties: BTreeMap<String, String>,
    /// Column names in order.
    data_format: Vec<String>,
    /// Row-major data: data[row * n_cols + col].
    data: Vec<String>,
    n_rows: usize,
    /// Column index that holds patch/sample IDs (default: 0 = "SAMPLE_ID").
    sample_id_col: usize,
}

impl Table {
    fn new() -> Self {
        Self {
            sheet_type: "CGATS.17".into(),
            properties: BTreeMap::new(),
            data_format: Vec::new(),
            data: Vec::new(),
            n_rows: 0,
            sample_id_col: 0,
        }
    }

    fn n_cols(&self) -> usize {
        self.data_format.len()
    }
}

/// CGATS / IT8 data container supporting multiple tables.
/// C版: `cmsIT8`
#[derive(Clone, Debug)]
pub struct It8 {
    tables: Vec<Table>,
    current: usize,
}

impl It8 {
    /// Create a new empty IT8 object with one default table.
    pub fn new() -> Self {
        Self {
            tables: vec![Table::new()],
            current: 0,
        }
    }

    /// Parse IT8/CGATS text.
    pub fn load_from_str(text: &str) -> Result<Self, CmsError> {
        Parser::parse(text).map_err(|msg| CmsError {
            code: ErrorCode::CorruptionDetected,
            message: msg,
        })
    }

    /// Load from file.
    /// C版: `cmsIT8LoadFromFile`
    pub fn load_from_file(path: &str) -> Result<Self, CmsError> {
        let text = std::fs::read_to_string(path).map_err(|e| CmsError {
            code: ErrorCode::File,
            message: format!("cannot read '{}': {}", path, e),
        })?;
        Self::load_from_str(&text)
    }

    /// Save to file.
    /// C版: `cmsIT8SaveToFile`
    pub fn save_to_file(&self, path: &str) -> Result<(), CmsError> {
        std::fs::write(path, self.save_to_string()).map_err(|e| CmsError {
            code: ErrorCode::File,
            message: format!("cannot write '{}': {}", path, e),
        })
    }

    /// Serialize to CGATS text format.
    pub fn save_to_string(&self) -> String {
        let mut out = String::new();
        for (i, table) in self.tables.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            write_table(&mut out, table);
        }
        out
    }

    // ---- Table management ----

    pub fn table_count(&self) -> usize {
        self.tables.len()
    }

    /// Switch active table. Creates new tables if needed.
    pub fn set_table(&mut self, index: usize) -> usize {
        while self.tables.len() <= index {
            self.tables.push(Table::new());
        }
        self.current = index;
        index
    }

    pub fn sheet_type(&self) -> &str {
        &self.tables[self.current].sheet_type
    }

    pub fn set_sheet_type(&mut self, t: &str) {
        self.tables[self.current].sheet_type = t.to_string();
    }

    // ---- Properties ----

    pub fn set_property(&mut self, key: &str, value: &str) {
        self.tables[self.current]
            .properties
            .insert(key.to_string(), value.to_string());
    }

    pub fn property(&self, key: &str) -> Option<&str> {
        self.tables[self.current]
            .properties
            .get(key)
            .map(|s| s.as_str())
    }

    pub fn property_f64(&self, key: &str) -> Option<f64> {
        self.property(key)?.parse().ok()
    }

    pub fn properties(&self) -> Vec<&str> {
        self.tables[self.current]
            .properties
            .keys()
            .map(|s| s.as_str())
            .collect()
    }

    // ---- Data format (columns) ----

    pub fn set_data_format(&mut self, col: usize, name: &str) {
        let t = &mut self.tables[self.current];
        if col >= t.data_format.len() {
            t.data_format.resize(col + 1, String::new());
        }
        t.data_format[col] = name.to_string();
        if name == "SAMPLE_ID" {
            t.sample_id_col = col;
        }
    }

    pub fn find_data_format(&self, name: &str) -> Option<usize> {
        self.tables[self.current]
            .data_format
            .iter()
            .position(|s| s == name)
    }

    pub fn data_format(&self) -> &[String] {
        &self.tables[self.current].data_format
    }

    // ---- Data access by row/col ----

    pub fn n_rows(&self) -> usize {
        self.tables[self.current].n_rows
    }

    pub fn data_row_col(&self, row: usize, col: usize) -> Option<&str> {
        let t = &self.tables[self.current];
        let idx = row.checked_mul(t.n_cols())?.checked_add(col)?;
        t.data.get(idx).map(|s| s.as_str())
    }

    pub fn data_row_col_f64(&self, row: usize, col: usize) -> Option<f64> {
        self.data_row_col(row, col)?.parse().ok()
    }

    pub fn set_data_row_col(&mut self, row: usize, col: usize, value: &str) {
        let t = &mut self.tables[self.current];
        let n_cols = t.n_cols();
        if n_cols == 0 || col >= n_cols {
            return; // no data format defined or column out of range
        }
        // Ensure enough rows
        if row >= t.n_rows {
            t.n_rows = row + 1;
        }
        let needed = t.n_rows * n_cols;
        if t.data.len() < needed {
            t.data.resize(needed, String::new());
        }
        t.data[row * n_cols + col] = value.to_string();
    }

    // ---- Data access by patch name ----

    pub fn data(&self, patch: &str, sample: &str) -> Option<&str> {
        let row = self.find_patch(patch)?;
        let col = self.find_data_format(sample)?;
        self.data_row_col(row, col)
    }

    pub fn data_f64(&self, patch: &str, sample: &str) -> Option<f64> {
        self.data(patch, sample)?.parse().ok()
    }

    pub fn set_data(&mut self, patch: &str, sample: &str, value: &str) {
        let col = match self.find_data_format(sample) {
            Some(c) => c,
            None => return,
        };
        // Find or create row for this patch
        let row = match self.find_patch(patch) {
            Some(r) => r,
            None => {
                let r = self.tables[self.current].n_rows;
                self.set_data_row_col(r, self.tables[self.current].sample_id_col, patch);
                r
            }
        };
        self.set_data_row_col(row, col, value);
    }

    /// Set a property as a floating-point value.
    /// C版: `cmsIT8SetPropertyDbl`
    pub fn set_property_f64(&mut self, _key: &str, _value: f64) {
        todo!("Phase 14a-D: not yet implemented")
    }

    /// Set a data cell by row/col as a floating-point value.
    /// C版: `cmsIT8SetDataRowColDbl`
    pub fn set_data_row_col_f64(&mut self, _row: usize, _col: usize, _value: f64) {
        todo!("Phase 14a-D: not yet implemented")
    }

    /// Set a data cell by patch name and sample as a floating-point value.
    /// C版: `cmsIT8SetDataDbl`
    pub fn set_data_f64(&mut self, _patch: &str, _sample: &str, _value: f64) {
        todo!("Phase 14a-D: not yet implemented")
    }

    /// Get the patch name (SAMPLE_ID) for a given row.
    /// C版: `cmsIT8GetPatchName`
    pub fn get_patch_name(&self, _row: usize) -> Option<&str> {
        todo!("Phase 14a-D: not yet implemented")
    }

    /// Set the format string used for floating-point data output.
    /// C版: `cmsIT8DefineDblFormat`
    pub fn define_dbl_format(&mut self, _fmt: &str) {
        todo!("Phase 14a-D: not yet implemented")
    }

    fn find_patch(&self, patch: &str) -> Option<usize> {
        let t = &self.tables[self.current];
        let sid = t.sample_id_col;
        (0..t.n_rows).find(|&r| t.data.get(r * t.n_cols() + sid).is_some_and(|v| v == patch))
    }
}

impl Default for It8 {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Writer
// ============================================================================

/// Quote a string value: escape backslashes and double-quotes.
fn quote_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Quote a cell value if it contains whitespace, quotes, or backslashes.
fn quote_cell(s: &str) -> String {
    if s.contains(|c: char| c.is_whitespace() || c == '"' || c == '\\') {
        quote_string(s)
    } else {
        s.to_string()
    }
}

fn write_table(out: &mut String, table: &Table) {
    use std::fmt::Write;

    // Sheet type
    writeln!(out, "{}", table.sheet_type).unwrap();

    // Properties
    for (key, value) in &table.properties {
        // Pure numeric values are unquoted; everything else is quoted
        if value.parse::<f64>().is_ok() {
            writeln!(out, "{key}\t{value}").unwrap();
        } else {
            writeln!(out, "{key}\t{}", quote_string(value)).unwrap();
        }
    }

    // Data format
    if !table.data_format.is_empty() {
        writeln!(out, "BEGIN_DATA_FORMAT").unwrap();
        writeln!(out, "{}", table.data_format.join("\t")).unwrap();
        writeln!(out, "END_DATA_FORMAT").unwrap();
    }

    // Data
    let n_cols = table.n_cols();
    if n_cols > 0 && table.n_rows > 0 {
        writeln!(out, "BEGIN_DATA").unwrap();
        for row in 0..table.n_rows {
            let start = row * n_cols;
            let end = start + n_cols;
            let cells: Vec<String> = table.data[start..end]
                .iter()
                .map(|s| quote_cell(s))
                .collect();
            writeln!(out, "{}", cells.join("\t")).unwrap();
        }
        writeln!(out, "END_DATA").unwrap();
    }
}

// ============================================================================
// Parser (tokenizer + recursive descent)
// ============================================================================

struct Parser<'a> {
    input: &'a str,
    pos: usize,
    line: usize,
}

impl<'a> Parser<'a> {
    fn parse(text: &str) -> Result<It8, String> {
        let mut p = Parser {
            input: text,
            pos: 0,
            line: 1,
        };
        let mut it8 = It8::new();

        // Parse first sheet type
        p.skip_whitespace_and_comments();
        if p.pos < p.input.len() {
            let sheet_type = p.read_token()?;
            it8.tables[0].sheet_type = sheet_type;
        }

        loop {
            p.skip_whitespace_and_comments();
            if p.at_end() {
                break;
            }

            let token = p.read_token()?;
            match token.as_str() {
                "BEGIN_DATA_FORMAT" => {
                    p.parse_data_format(&mut it8)?;
                }
                "BEGIN_DATA" => {
                    p.parse_data(&mut it8)?;
                    // parse_data consumes END_DATA.
                    // Check if another table follows.
                    p.skip_whitespace_and_comments();
                    if !p.at_end()
                        && p.peek_token()
                            .as_deref()
                            .is_some_and(|t| t != "BEGIN_DATA_FORMAT" && t != "BEGIN_DATA")
                    {
                        let sheet_type = p.read_token()?;
                        let idx = it8.tables.len();
                        it8.set_table(idx);
                        it8.tables[idx].sheet_type = sheet_type;
                    }
                }
                _ => {
                    // Property: KEY VALUE
                    p.skip_inline_whitespace();
                    if !p.at_eol() && !p.at_end() {
                        let value = p.read_value()?;
                        let table = &mut it8.tables[it8.current];
                        table.properties.insert(token, value);
                    }
                }
            }
        }

        // Fix active table back to 0
        it8.current = 0;
        Ok(it8)
    }

    fn parse_data_format(&mut self, it8: &mut It8) -> Result<(), String> {
        let table = &mut it8.tables[it8.current];
        table.data_format.clear();
        loop {
            self.skip_whitespace_and_comments();
            if self.at_end() {
                return Err(format!("line {}: unexpected EOF in DATA_FORMAT", self.line));
            }
            let tok = self.read_token()?;
            if tok == "END_DATA_FORMAT" {
                break;
            }
            if tok == "SAMPLE_ID" {
                table.sample_id_col = table.data_format.len();
            }
            table.data_format.push(tok);
        }
        Ok(())
    }

    fn parse_data(&mut self, it8: &mut It8) -> Result<(), String> {
        let table = &mut it8.tables[it8.current];
        let n_cols = table.n_cols();
        if n_cols == 0 {
            return Err(format!(
                "line {}: BEGIN_DATA without data format",
                self.line
            ));
        }

        table.data.clear();
        table.n_rows = 0;

        loop {
            self.skip_whitespace_and_comments();
            if self.at_end() {
                return Err(format!("line {}: unexpected EOF in DATA", self.line));
            }

            // Peek for END_DATA
            if self.peek_token().as_deref() == Some("END_DATA") {
                self.read_token()?; // consume it
                break;
            }

            // Read one row
            for _ in 0..n_cols {
                self.skip_inline_whitespace();
                let val = self.read_value()?;
                table.data.push(val);
            }
            table.n_rows += 1;
            self.skip_to_eol();
        }

        Ok(())
    }

    // ---- Low-level tokenizer ----

    fn at_end(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn at_eol(&self) -> bool {
        self.peek_char().is_some_and(|c| c == '\n' || c == '\r')
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        self.pos += c.len_utf8();
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }

    fn skip_whitespace_and_comments(&mut self) {
        while !self.at_end() {
            match self.peek_char() {
                Some(' ' | '\t' | '\r' | '\n') => {
                    self.advance();
                }
                Some('#') => {
                    // Skip comment to end of line
                    while !self.at_end() && !self.at_eol() {
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn skip_inline_whitespace(&mut self) {
        while let Some(' ' | '\t') = self.peek_char() {
            self.advance();
        }
    }

    fn skip_to_eol(&mut self) {
        while !self.at_end() && !self.at_eol() {
            self.advance();
        }
    }

    /// Read a token (identifier, number, or keyword).
    fn read_token(&mut self) -> Result<String, String> {
        self.skip_inline_whitespace();
        if self.at_end() {
            return Err(format!("line {}: unexpected end of input", self.line));
        }

        if self.peek_char() == Some('"') {
            return self.read_quoted_string();
        }

        let start = self.pos;
        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/' || c == '+' {
                self.advance();
            } else {
                break;
            }
        }

        if self.pos == start {
            return Err(format!(
                "line {}: unexpected character '{}'",
                self.line,
                self.peek_char().unwrap_or('?')
            ));
        }

        Ok(self.input[start..self.pos].to_string())
    }

    /// Read a value (token or quoted string).
    fn read_value(&mut self) -> Result<String, String> {
        self.skip_inline_whitespace();
        if self.peek_char() == Some('"') {
            self.read_quoted_string()
        } else {
            self.read_token()
        }
    }

    fn read_quoted_string(&mut self) -> Result<String, String> {
        self.advance(); // skip opening quote
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('"') => return Ok(s),
                Some('\\') => {
                    if let Some(c) = self.advance() {
                        s.push(c);
                    }
                }
                Some(c) => s.push(c),
                None => return Err(format!("line {}: unterminated string", self.line)),
            }
        }
    }

    fn peek_token(&self) -> Option<String> {
        let mut clone = Parser {
            input: self.input,
            pos: self.pos,
            line: self.line,
        };
        clone.skip_inline_whitespace();
        clone.read_token().ok()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_IT8: &str = r#"CGATS.17
ORIGINATOR	"test instrument"
NUMBER_OF_FIELDS	4
NUMBER_OF_SETS	3
BEGIN_DATA_FORMAT
SAMPLE_ID	LAB_L	LAB_A	LAB_B
END_DATA_FORMAT
BEGIN_DATA
A1	95.0	-0.5	1.2
A2	50.0	20.0	-10.0
A3	10.0	0.0	0.0
END_DATA
"#;

    #[test]
    fn parse_basic_it8() {
        let it8 = It8::load_from_str(SAMPLE_IT8).unwrap();
        assert_eq!(it8.sheet_type(), "CGATS.17");
        assert_eq!(it8.table_count(), 1);
        assert_eq!(it8.n_rows(), 3);
    }

    #[test]
    fn read_properties() {
        let it8 = It8::load_from_str(SAMPLE_IT8).unwrap();
        assert_eq!(it8.property("ORIGINATOR"), Some("test instrument"));
        assert_eq!(it8.property_f64("NUMBER_OF_FIELDS"), Some(4.0));
        assert_eq!(it8.property_f64("NUMBER_OF_SETS"), Some(3.0));
    }

    #[test]
    fn read_data_by_row_col() {
        let it8 = It8::load_from_str(SAMPLE_IT8).unwrap();
        assert_eq!(it8.data_row_col(0, 0), Some("A1"));
        assert_eq!(it8.data_row_col_f64(0, 1), Some(95.0));
        assert_eq!(it8.data_row_col_f64(1, 3), Some(-10.0));
        assert_eq!(it8.data_row_col_f64(2, 1), Some(10.0));
    }

    #[test]
    fn read_data_by_patch_name() {
        let it8 = It8::load_from_str(SAMPLE_IT8).unwrap();
        assert_eq!(it8.data_f64("A1", "LAB_L"), Some(95.0));
        assert_eq!(it8.data_f64("A2", "LAB_A"), Some(20.0));
        assert_eq!(it8.data("A3", "SAMPLE_ID"), Some("A3"));
    }

    #[test]
    fn save_and_reload_round_trip() {
        let it8 = It8::load_from_str(SAMPLE_IT8).unwrap();
        let text = it8.save_to_string();
        let it8b = It8::load_from_str(&text).unwrap();

        assert_eq!(it8b.sheet_type(), "CGATS.17");
        assert_eq!(it8b.n_rows(), 3);
        assert_eq!(it8b.data_f64("A1", "LAB_L"), Some(95.0));
        assert_eq!(it8b.data_f64("A2", "LAB_A"), Some(20.0));
    }

    #[test]
    fn programmatic_construction() {
        let mut it8 = It8::new();
        it8.set_sheet_type("IT8.7/1");
        it8.set_property("ORIGINATOR", "test");
        it8.set_data_format(0, "SAMPLE_ID");
        it8.set_data_format(1, "RGB_R");
        it8.set_data_format(2, "RGB_G");
        it8.set_data_format(3, "RGB_B");

        it8.set_data_row_col(0, 0, "P1");
        it8.set_data_row_col(0, 1, "255");
        it8.set_data_row_col(0, 2, "0");
        it8.set_data_row_col(0, 3, "0");

        assert_eq!(it8.data_f64("P1", "RGB_R"), Some(255.0));

        let text = it8.save_to_string();
        assert!(text.contains("IT8.7/1"));
        assert!(text.contains("BEGIN_DATA"));
    }

    #[test]
    fn empty_it8() {
        let it8 = It8::new();
        assert_eq!(it8.table_count(), 1);
        assert_eq!(it8.n_rows(), 0);
        assert_eq!(it8.sheet_type(), "CGATS.17");
        assert!(it8.property("ANYTHING").is_none());
    }

    #[test]
    fn multi_table() {
        let text = "TABLE1\nNUMBER_OF_FIELDS\t2\nNUMBER_OF_SETS\t1\n\
                     BEGIN_DATA_FORMAT\nX\tY\nEND_DATA_FORMAT\n\
                     BEGIN_DATA\n1\t2\nEND_DATA\n\
                     TABLE2\nNUMBER_OF_FIELDS\t2\nNUMBER_OF_SETS\t1\n\
                     BEGIN_DATA_FORMAT\nA\tB\nEND_DATA_FORMAT\n\
                     BEGIN_DATA\n3\t4\nEND_DATA\n";
        let mut it8 = It8::load_from_str(text).unwrap();
        assert_eq!(it8.table_count(), 2);

        it8.set_table(0);
        assert_eq!(it8.sheet_type(), "TABLE1");
        assert_eq!(it8.data_row_col_f64(0, 0), Some(1.0));

        it8.set_table(1);
        assert_eq!(it8.sheet_type(), "TABLE2");
        assert_eq!(it8.data_row_col_f64(0, 0), Some(3.0));
    }

    #[test]
    fn comment_lines_ignored() {
        let text = "CGATS.17\n# This is a comment\nORIGINATOR\t\"test\"\n\
                     NUMBER_OF_FIELDS\t1\nNUMBER_OF_SETS\t1\n\
                     BEGIN_DATA_FORMAT\nX\nEND_DATA_FORMAT\n\
                     # Another comment\n\
                     BEGIN_DATA\n42\nEND_DATA\n";
        let it8 = It8::load_from_str(text).unwrap();
        assert_eq!(it8.property("ORIGINATOR"), Some("test"));
        assert_eq!(it8.data_row_col_f64(0, 0), Some(42.0));
    }

    // ================================================================
    // Phase 13c: File I/O
    // ================================================================

    #[test]
    fn file_io_round_trip() {
        let it8 = It8::load_from_str(SAMPLE_IT8).unwrap();
        let dir = std::env::temp_dir();
        let name = format!("test_it8_round_trip_{}.it8", std::process::id());
        let path = dir.join(name);
        let path_str = path.to_str().unwrap();

        it8.save_to_file(path_str).unwrap();
        let loaded = It8::load_from_file(path_str).unwrap();

        assert_eq!(loaded.property("ORIGINATOR"), Some("test instrument"));
        assert_eq!(loaded.n_rows(), 3);
        assert_eq!(loaded.data_row_col_f64(0, 1), Some(95.0));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn load_from_nonexistent_file() {
        let dir = std::env::temp_dir();
        let name = format!("nonexistent_it8_{}.it8", std::process::id());
        let path = dir.join(name);
        assert!(!path.exists());
        let result = It8::load_from_file(path.to_str().unwrap());
        assert!(result.is_err());
    }

    // ================================================================
    // Phase 14a-D: Numeric setter/getter APIs
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn set_property_f64_round_trip() {
        let mut it8 = It8::new();
        it8.set_property_f64("MY_VALUE", 1.234);
        let val = it8.property_f64("MY_VALUE").unwrap();
        assert!((val - 1.234).abs() < 1e-10);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn set_data_row_col_f64_round_trip() {
        let mut it8 = It8::new();
        it8.set_data_format(0, "SAMPLE_ID");
        it8.set_data_format(1, "VALUE");
        it8.set_data_row_col(0, 0, "P1");
        it8.set_data_row_col_f64(0, 1, 42.5);
        assert_eq!(it8.data_row_col_f64(0, 1), Some(42.5));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn set_data_f64_round_trip() {
        let mut it8 = It8::new();
        it8.set_data_format(0, "SAMPLE_ID");
        it8.set_data_format(1, "MEASURE");
        it8.set_data_row_col(0, 0, "P1");
        it8.set_data_f64("P1", "MEASURE", 99.9);
        assert_eq!(it8.data_f64("P1", "MEASURE"), Some(99.9));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn get_patch_name_returns_sample_id() {
        let it8 = It8::load_from_str(SAMPLE_IT8).unwrap();
        assert_eq!(it8.get_patch_name(0), Some("A1"));
        assert_eq!(it8.get_patch_name(1), Some("A2"));
        assert_eq!(it8.get_patch_name(2), Some("A3"));
        assert_eq!(it8.get_patch_name(3), None);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn define_dbl_format_affects_output() {
        let mut it8 = It8::new();
        it8.set_data_format(0, "SAMPLE_ID");
        it8.set_data_format(1, "VALUE");
        it8.define_dbl_format("{:.2}");
        it8.set_data_row_col(0, 0, "P1");
        it8.set_data_row_col_f64(0, 1, 1.23456);
        // With format "{:.2}", the value should be written as "1.23"
        assert_eq!(it8.data_row_col(0, 1), Some("1.23"));
    }
}
