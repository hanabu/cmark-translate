// SPDX-License-Identifier: MIT
//!
//! Read glossaries from .xlsx
//!

pub fn read_glossary<P: AsRef<std::path::Path>>(
    _xlsx_path: P,
    _from: &str,
    _to: &str,
) -> Result<Vec<(String, String)>, umya_spreadsheet::structs::XlsxError> {
    todo!()
}
