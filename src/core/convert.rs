use serde::Serialize;

use crate::core::error::{Error, Result};

/// Serialize rows as a JSON array string.
pub fn to_json<T: Serialize>(rows: &[T]) -> Result<String> {
    serde_json::to_string(rows).map_err(Error::Json)
}

/// Serialize rows as CSV (header from struct field names).
pub fn to_csv<T: Serialize>(rows: &[T]) -> Result<String> {
    let mut w = csv::Writer::from_writer(Vec::new());
    for r in rows {
        w.serialize(r).map_err(|e| Error::Csv(e.to_string()))?;
    }
    w.flush().map_err(|e| Error::Csv(e.to_string()))?;
    let bytes = w.into_inner().map_err(|e| Error::Csv(e.to_string()))?;
    String::from_utf8(bytes).map_err(|e| Error::Csv(e.to_string()))
}

/// Serialize rows to a Parquet file.
///
/// Enabled by the `parquet` feature (ADR-0001 / ADR-0014): the core build stays
/// lean, Parquet support is opt-in.
#[cfg(feature = "parquet")]
pub fn to_parquet<T: Serialize>(rows: &[T], path: &std::path::Path) -> Result<()> {
    use arrow::json::reader::{ReaderBuilder, infer_json_schema};
    use parquet::arrow::ArrowWriter;

    if rows.is_empty() {
        return Err(Error::Parquet("no rows to serialize".into()));
    }
    let values: Vec<serde_json::Value> = rows
        .iter()
        .map(serde_json::to_value)
        .collect::<std::result::Result<_, _>>()
        .map_err(Error::Json)?;
    let schema = infer_json_schema(&mut values.iter(), Some(values.len()))
        .map_err(|e| Error::Parquet(e.to_string()))?;
    let mut reader = ReaderBuilder::new(schema)
        .build(values.into_iter())
        .map_err(|e| Error::Parquet(e.to_string()))?;
    let batch = reader
        .next()
        .transpose()
        .map_err(|e| Error::Parquet(e.to_string()))?
        .ok_or_else(|| Error::Parquet("empty record batch".into()))?;
    let file = std::fs::File::create(path).map_err(Error::Io)?;
    let mut writer = ArrowWriter::try_new(file, batch.schema(), None)
        .map_err(|e| Error::Parquet(e.to_string()))?;
    writer
        .write(&batch)
        .map_err(|e| Error::Parquet(e.to_string()))?;
    writer.close().map_err(|e| Error::Parquet(e.to_string()))?;
    Ok(())
}
