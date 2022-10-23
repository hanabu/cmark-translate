// SPDX-License-Identifier: MIT
//!
//! Read glossaries from .xlsx
//!

pub fn read_glossary<P: AsRef<std::path::Path>>(
    xlsx_path: P,
    from: &str,
    to: &str,
) -> Result<Vec<(String, String)>, umya_spreadsheet::reader::xlsx::XlsxError> {
    todo!()
}
