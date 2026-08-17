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
    use std::io::BufReader;

    if rows.is_empty() {
        return Err(Error::Parquet("no rows to serialize".into()));
    }

    // Serialize each row to a JSON object, one per line (JSON Lines), which the
    // arrow JSON reader streams through a `BufRead`.
    let mut text = String::new();
    for r in rows {
        let v = serde_json::to_value(r).map_err(Error::Json)?;
        text.push_str(&serde_json::to_string(&v).map_err(Error::Json)?);
        text.push('\n');
    }
    let bytes = text.as_bytes();

    let (schema, _) = infer_json_schema(
        BufReader::new(std::io::Cursor::new(bytes)),
        Some(rows.len()),
    )
    .map_err(|e| Error::Parquet(e.to_string()))?;
    let schema = std::sync::Arc::new(schema);

    let mut reader = ReaderBuilder::new(schema)
        .build(BufReader::new(std::io::Cursor::new(bytes)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Row {
        code: String,
        price: Option<f64>,
    }

    #[test]
    fn to_json_roundtrip() {
        let rows = vec![
            Row { code: "USD".into(), price: Some(7.1) },
            Row { code: "EUR".into(), price: None },
        ];
        let json = to_json(&rows).unwrap();
        let parsed: Vec<Row> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, rows);
    }

    #[test]
    fn to_csv_roundtrip() {
        let rows = vec![
            Row { code: "USD".into(), price: Some(7.1) },
            Row { code: "EUR".into(), price: None },
        ];
        let csv = to_csv(&rows).unwrap();
        let mut reader = csv::Reader::from_reader(csv.as_bytes());
        let got: Vec<Row> = reader.deserialize().map(|r| r.unwrap()).collect();
        assert_eq!(got, rows);
    }

    #[test]
    fn to_csv_empty_is_empty() {
        // The csv writer emits a header on the first `serialize` call; with no
        // rows there is nothing to flush, so the empty input yields empty output.
        let rows: Vec<Row> = Vec::new();
        let csv = to_csv(&rows).unwrap();
        assert!(csv.is_empty());
    }
}
