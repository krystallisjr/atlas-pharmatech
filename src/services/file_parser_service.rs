/// Enterprise-grade file parser for pharmaceutical inventory imports
/// Supports: CSV, Excel (XLSX/XLS), JSON with intelligent format detection

use std::io::Cursor;
use csv::ReaderBuilder;
use calamine::{Reader, open_workbook_from_rs, Xlsx, Xls, Data};
use serde_json::Value as JsonValue;
use sha2::{Sha256, Digest};
use crate::middleware::error_handling::{Result, AppError};

// ============================================================================
// Public API Models
// ============================================================================

#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub file_type: FileType,
    pub file_hash: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_rows: usize,
    pub metadata: FileMetadata,
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub original_filename: String,
    pub file_size_bytes: usize,
    pub detected_encoding: Option<String>,
    pub has_header_row: bool,
    pub empty_rows_skipped: usize,
    pub parsing_warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    Csv,
    Excel,
    Json,
}

impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileType::Csv => write!(f, "csv"),
            FileType::Excel => write!(f, "xlsx"),
            FileType::Json => write!(f, "json"),
        }
    }
}

// ============================================================================
// File Parser Service
// ============================================================================

pub struct FileParserService;

impl FileParserService {
    /// Parse any supported file format with intelligent detection
    pub fn parse(
        file_data: &[u8],
        filename: &str,
    ) -> Result<ParsedFile> {
        // Calculate file hash for deduplication
        let file_hash = Self::calculate_hash(file_data);

        // Detect file type
        let file_type = Self::detect_file_type(file_data, filename)?;

        tracing::info!(
            "Parsing file: {} ({} bytes, type: {})",
            filename,
            file_data.len(),
            file_type
        );

        // Parse based on type
        let mut parsed = match file_type {
            FileType::Csv => Self::parse_csv(file_data, filename)?,
            FileType::Excel => Self::parse_excel(file_data, filename)?,
            FileType::Json => Self::parse_json(file_data, filename)?,
        };

        parsed.file_type = file_type;
        parsed.file_hash = file_hash;

        Ok(parsed)
    }

    /// Detect file type from content and filename
    fn detect_file_type(data: &[u8], filename: &str) -> Result<FileType> {
        let filename_lower = filename.to_lowercase();

        // Check file extension first
        if filename_lower.ends_with(".csv") || filename_lower.ends_with(".txt") {
            return Ok(FileType::Csv);
        }

        if filename_lower.ends_with(".xlsx") || filename_lower.ends_with(".xls") {
            return Ok(FileType::Excel);
        }

        if filename_lower.ends_with(".json") {
            return Ok(FileType::Json);
        }

        // Check magic bytes for Excel
        if data.len() >= 4 {
            // XLSX (ZIP format) starts with PK
            if &data[0..2] == b"PK" {
                return Ok(FileType::Excel);
            }

            // XLS (OLE format) starts with D0 CF 11 E0
            if data.len() >= 8 && &data[0..8] == b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1" {
                return Ok(FileType::Excel);
            }
        }

        // Check if it's JSON
        if let Ok(s) = std::str::from_utf8(data) {
            let trimmed = s.trim();
            if (trimmed.starts_with('{') && trimmed.ends_with('}')) ||
               (trimmed.starts_with('[') && trimmed.ends_with(']')) {
                return Ok(FileType::Json);
            }
        }

        // Default to CSV for text files
        if Self::is_likely_csv(data) {
            return Ok(FileType::Csv);
        }

        Err(AppError::InvalidInput(
            "Unsupported file format. Please upload CSV, Excel (XLSX/XLS), or JSON files.".to_string()
        ))
    }

    /// Check if data looks like CSV
    fn is_likely_csv(data: &[u8]) -> bool {
        if let Ok(s) = std::str::from_utf8(data) {
            let first_lines: Vec<&str> = s.lines().take(5).collect();
            if first_lines.is_empty() {
                return false;
            }

            // Check if first few lines have consistent delimiter patterns
            let common_delimiters = [',', '\t', ';', '|'];
            for delimiter in &common_delimiters {
                let counts: Vec<usize> = first_lines.iter()
                    .map(|line| line.matches(*delimiter).count())
                    .collect();

                if counts.len() >= 2 {
                    let first_count = counts[0];
                    if first_count > 0 && counts.iter().all(|&c| c == first_count) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Parse CSV file with intelligent delimiter detection
    fn parse_csv(data: &[u8], filename: &str) -> Result<ParsedFile> {
        let text = std::str::from_utf8(data)
            .map_err(|e| AppError::InvalidInput(format!("Invalid UTF-8 encoding: {}", e)))?;

        // Detect delimiter
        let delimiter = Self::detect_csv_delimiter(text);

        let mut reader = ReaderBuilder::new()
            .delimiter(delimiter as u8)
            .flexible(true)
            .trim(csv::Trim::All)
            .from_reader(text.as_bytes());

        // Get headers
        let headers: Vec<String> = match reader.headers() {
            Ok(h) => h.iter().map(|s| s.trim().to_string()).collect(),
            Err(e) => {
                return Err(AppError::InvalidInput(
                    format!("Failed to read CSV headers: {}", e)
                ));
            }
        };

        // Validate headers
        if headers.is_empty() {
            return Err(AppError::InvalidInput(
                "CSV file has no headers. Please ensure first row contains column names.".to_string()
            ));
        }

        // Parse rows
        let mut rows = Vec::new();
        let mut empty_rows_skipped = 0;
        let mut warnings = Vec::new();

        for (idx, result) in reader.records().enumerate() {
            match result {
                Ok(record) => {
                    let row: Vec<String> = record.iter()
                        .map(|s| s.trim().to_string())
                        .collect();

                    // Skip completely empty rows
                    if row.iter().all(|s| s.is_empty()) {
                        empty_rows_skipped += 1;
                        continue;
                    }

                    // Warn if row has different column count
                    if row.len() != headers.len() {
                        warnings.push(format!(
                            "Row {} has {} columns, expected {}",
                            idx + 2, // +2 because of header and 0-indexing
                            row.len(),
                            headers.len()
                        ));
                    }

                    rows.push(row);
                }
                Err(e) => {
                    warnings.push(format!("Row {} parsing error: {}", idx + 2, e));
                }
            }
        }

        if rows.is_empty() {
            return Err(AppError::InvalidInput(
                "CSV file contains no data rows.".to_string()
            ));
        }

        tracing::info!(
            "Parsed CSV: {} rows, {} columns, {} empty rows skipped",
            rows.len(),
            headers.len(),
            empty_rows_skipped
        );

        Ok(ParsedFile {
            file_type: FileType::Csv,
            file_hash: String::new(), // Will be set by caller
            headers,
            total_rows: rows.len(),
            rows,
            metadata: FileMetadata {
                original_filename: filename.to_string(),
                file_size_bytes: data.len(),
                detected_encoding: Some("UTF-8".to_string()),
                has_header_row: true,
                empty_rows_skipped,
                parsing_warnings: warnings,
            },
        })
    }

    /// Detect CSV delimiter (comma, tab, semicolon, pipe)
    fn detect_csv_delimiter(text: &str) -> char {
        let first_line = text.lines().next().unwrap_or("");

        let delimiters = [(',', "comma"), ('\t', "tab"), (';', "semicolon"), ('|', "pipe")];
        let mut counts: Vec<(char, usize)> = delimiters.iter()
            .map(|(delim, _)| (*delim, first_line.matches(*delim).count()))
            .collect();

        counts.sort_by(|a, b| b.1.cmp(&a.1));

        counts.first().map(|(d, _)| *d).unwrap_or(',')
    }

    /// Parse Excel file (XLSX or XLS)
    fn parse_excel(data: &[u8], filename: &str) -> Result<ParsedFile> {
        let cursor = Cursor::new(data);

        // Try to open as XLSX first, then XLS
        let range = if filename.to_lowercase().ends_with(".xlsx") {
            // Try XLSX first
            match open_workbook_from_rs::<Xlsx<_>, _>(cursor.clone()) {
                Ok(mut workbook) => Self::get_sheet_range_xlsx(&mut workbook)?,
                Err(_) => {
                    // Fallback to XLS
                    let cursor = Cursor::new(data);
                    let mut workbook = open_workbook_from_rs::<Xls<_>, _>(cursor)
                        .map_err(|e| AppError::InvalidInput(
                            format!("Failed to open Excel file: {}", e)
                        ))?;
                    Self::get_sheet_range_xls(&mut workbook)?
                }
            }
        } else {
            // Try XLS first for .xls extension
            match open_workbook_from_rs::<Xls<_>, _>(cursor.clone()) {
                Ok(mut workbook) => Self::get_sheet_range_xls(&mut workbook)?,
                Err(_) => {
                    // Fallback to XLSX
                    let cursor = Cursor::new(data);
                    let mut workbook = open_workbook_from_rs::<Xlsx<_>, _>(cursor)
                        .map_err(|e| AppError::InvalidInput(
                            format!("Failed to open Excel file: {}", e)
                        ))?;
                    Self::get_sheet_range_xlsx(&mut workbook)?
                }
            }
        };

        let mut rows_iter = range.rows();

        // Get headers from first row
        let headers = match rows_iter.next() {
            Some(header_row) => {
                header_row.iter()
                    .map(|cell| Self::cell_to_string(cell))
                    .collect::<Vec<String>>()
            }
            None => {
                return Err(AppError::InvalidInput(
                    "Excel sheet is empty.".to_string()
                ));
            }
        };

        if headers.is_empty() || headers.iter().all(|h| h.is_empty()) {
            return Err(AppError::InvalidInput(
                "Excel file has no headers in first row.".to_string()
            ));
        }

        // Parse data rows
        let mut rows = Vec::new();
        let mut empty_rows_skipped = 0;
        let mut warnings = Vec::new();

        for (idx, row) in rows_iter.enumerate() {
            let row_data: Vec<String> = row.iter()
                .map(|cell| Self::cell_to_string(cell))
                .collect();

            // Skip completely empty rows
            if row_data.iter().all(|s| s.is_empty()) {
                empty_rows_skipped += 1;
                continue;
            }

            // Warn if row has different column count
            if row_data.len() != headers.len() {
                warnings.push(format!(
                    "Row {} has {} columns, expected {}",
                    idx + 2,
                    row_data.len(),
                    headers.len()
                ));
            }

            rows.push(row_data);
        }

        if rows.is_empty() {
            return Err(AppError::InvalidInput(
                "Excel file contains no data rows.".to_string()
            ));
        }

        tracing::info!(
            "Parsed Excel: {} rows, {} columns, {} empty rows skipped",
            rows.len(),
            headers.len(),
            empty_rows_skipped
        );

        Ok(ParsedFile {
            file_type: FileType::Excel,
            file_hash: String::new(),
            headers,
            total_rows: rows.len(),
            rows,
            metadata: FileMetadata {
                original_filename: filename.to_string(),
                file_size_bytes: data.len(),
                detected_encoding: None,
                has_header_row: true,
                empty_rows_skipped,
                parsing_warnings: warnings,
            },
        })
    }

    /// Get first sheet range from XLSX workbook
    fn get_sheet_range_xlsx(workbook: &mut Xlsx<Cursor<&[u8]>>) -> Result<calamine::Range<Data>> {
        let sheet_names = workbook.sheet_names().to_owned();
        if sheet_names.is_empty() {
            return Err(AppError::InvalidInput(
                "Excel file contains no sheets.".to_string()
            ));
        }

        let sheet_name = &sheet_names[0];
        workbook
            .worksheet_range(sheet_name)
            .map_err(|e| AppError::InvalidInput(
                format!("Failed to parse sheet {}: {}", sheet_name, e)
            ))
    }

    /// Get first sheet range from XLS workbook
    fn get_sheet_range_xls(workbook: &mut Xls<Cursor<&[u8]>>) -> Result<calamine::Range<Data>> {
        let sheet_names = workbook.sheet_names().to_owned();
        if sheet_names.is_empty() {
            return Err(AppError::InvalidInput(
                "Excel file contains no sheets.".to_string()
            ));
        }

        let sheet_name = &sheet_names[0];
        workbook
            .worksheet_range(sheet_name)
            .map_err(|e| AppError::InvalidInput(
                format!("Failed to parse sheet {}: {}", sheet_name, e)
            ))
    }

    /// Convert Excel cell to string
    fn cell_to_string(cell: &Data) -> String {
        match cell {
            Data::Int(i) => i.to_string(),
            Data::Float(f) => {
                // Remove trailing zeros and decimal point if whole number
                let s = format!("{}", f);
                if s.contains('.') {
                    s.trim_end_matches('0').trim_end_matches('.').to_string()
                } else {
                    s
                }
            }
            Data::String(s) => s.trim().to_string(),
            Data::Bool(b) => b.to_string(),
            Data::DateTime(dt) => format!("{}", dt),
            Data::DateTimeIso(s) => s.clone(),
            Data::DurationIso(s) => s.clone(),
            Data::Error(e) => format!("ERROR: {:?}", e),
            Data::Empty => String::new(),
        }
    }

    /// Parse JSON file (array of objects or single object)
    fn parse_json(data: &[u8], filename: &str) -> Result<ParsedFile> {
        let text = std::str::from_utf8(data)
            .map_err(|e| AppError::InvalidInput(format!("Invalid UTF-8 encoding: {}", e)))?;

        let json: JsonValue = serde_json::from_str(text)
            .map_err(|e| AppError::InvalidInput(format!("Invalid JSON: {}", e)))?;

        let objects = match json {
            JsonValue::Array(arr) => arr,
            JsonValue::Object(_) => vec![json],
            _ => {
                return Err(AppError::InvalidInput(
                    "JSON must be an array of objects or a single object.".to_string()
                ));
            }
        };

        if objects.is_empty() {
            return Err(AppError::InvalidInput(
                "JSON file contains no data.".to_string()
            ));
        }

        // Extract headers from all objects (union of all keys)
        let mut header_set = std::collections::BTreeSet::new();
        for obj in &objects {
            if let JsonValue::Object(map) = obj {
                for key in map.keys() {
                    header_set.insert(key.clone());
                }
            }
        }

        let headers: Vec<String> = header_set.into_iter().collect();

        if headers.is_empty() {
            return Err(AppError::InvalidInput(
                "JSON objects have no properties.".to_string()
            ));
        }

        // Convert objects to rows
        let mut rows = Vec::new();
        let mut warnings = Vec::new();

        for (idx, obj) in objects.iter().enumerate() {
            if let JsonValue::Object(map) = obj {
                let row: Vec<String> = headers.iter()
                    .map(|header| {
                        match map.get(header) {
                            Some(JsonValue::String(s)) => s.clone(),
                            Some(JsonValue::Number(n)) => n.to_string(),
                            Some(JsonValue::Bool(b)) => b.to_string(),
                            Some(JsonValue::Null) => String::new(),
                            Some(other) => serde_json::to_string(other).unwrap_or_default(),
                            None => String::new(),
                        }
                    })
                    .collect();

                rows.push(row);
            } else {
                warnings.push(format!("Item {} is not a JSON object", idx + 1));
            }
        }

        tracing::info!(
            "Parsed JSON: {} rows, {} columns",
            rows.len(),
            headers.len()
        );

        Ok(ParsedFile {
            file_type: FileType::Json,
            file_hash: String::new(),
            headers,
            total_rows: rows.len(),
            rows,
            metadata: FileMetadata {
                original_filename: filename.to_string(),
                file_size_bytes: data.len(),
                detected_encoding: Some("UTF-8".to_string()),
                has_header_row: true,
                empty_rows_skipped: 0,
                parsing_warnings: warnings,
            },
        })
    }

    /// Calculate SHA256 hash of file for deduplication
    fn calculate_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_parsing() {
        let csv_data = b"NDC,Product,Quantity,Expiry\n12345-678-90,Amoxicillin,1000,2026-12-31\n";
        let result = FileParserService::parse(csv_data, "test.csv").unwrap();
        assert_eq!(result.file_type, FileType::Csv);
        assert_eq!(result.headers.len(), 4);
        assert_eq!(result.rows.len(), 1);
    }

    #[test]
    fn test_json_parsing() {
        let json_data = br#"[{"ndc": "12345-678-90", "product": "Amoxicillin", "quantity": 1000}]"#;
        let result = FileParserService::parse(json_data, "test.json").unwrap();
        assert_eq!(result.file_type, FileType::Json);
        assert!(result.headers.len() >= 3);
        assert_eq!(result.rows.len(), 1);
    }
}
