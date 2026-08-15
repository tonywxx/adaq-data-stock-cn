use serde::Serialize;
use serde_json::Value;

use crate::core::client::Client;
use crate::core::error::{Error, Result};

const SOURCE_OWNTHINK: &str = "ownthink";
const KG_URL: &str = "https://api.ownthink.com/kg/knowledge";
const BOT_URL: &str = "https://api.ownthink.com/bot";

/// A single knowledge-graph attribute (`nlp_ownthink`).
#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeRow {
    /// Attribute name (e.g. `entity`, `desc`, `tag`) or an `avp` field name.
    pub field: String,
    pub value: String,
}

/// OwnThink knowledge-graph lookup (`nlp_ownthink`).
///
/// `indicator` selects which slice of `data` is returned: `entity`, `desc`, `avp`
/// (attribute-value pairs) or `tag`. Returns an empty `Vec` when the entity is unknown.
pub async fn nlp_ownthink(
    client: &Client,
    word: &str,
    indicator: &str,
) -> Result<Vec<KnowledgeRow>> {
    let params = [("entity", word)];
    let v = client
        .post_form_json(SOURCE_OWNTHINK, "nlp_ownthink", KG_URL, &params, None)
        .await?;
    let data = match v.get("data") {
        Some(d) if !d.is_null() => d,
        _ => return Ok(Vec::new()),
    };
    match indicator {
        "entity" => {
            let s = data.get("entity").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            if s.is_empty() {
                Ok(Vec::new())
            } else {
                Ok(vec![KnowledgeRow { field: "entity".into(), value: s }])
            }
        }
        "desc" => {
            let s = data.get("desc").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            Ok(vec![KnowledgeRow { field: "desc".into(), value: s }])
        }
        "avp" => {
            let arr = data
                .get("avp")
                .and_then(|a| a.as_array())
                .ok_or_else(|| Error::UpstreamChanged {
                    origin: SOURCE_OWNTHINK,
                    message: "missing data.avp".into(),
                })?;
            Ok(arr
                .iter()
                .map(|pair| KnowledgeRow {
                    field: pair.get(0).and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    value: pair
                        .get(1)
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                })
                .collect())
        }
        "tag" => {
            let arr = data
                .get("tag")
                .and_then(|a| a.as_array())
                .ok_or_else(|| Error::UpstreamChanged {
                    origin: SOURCE_OWNTHINK,
                    message: "missing data.tag".into(),
                })?;
            Ok(arr
                .iter()
                .map(|v| KnowledgeRow {
                    field: "tag".into(),
                    value: v.as_str().unwrap_or_default().to_string(),
                })
                .collect())
        }
        other => Err(Error::InvalidParam(format!("unknown indicator `{other}`"))),
    }
}

/// OwnThink intelligent Q&A (`nlp_answer`). Returns the answer text.
pub async fn nlp_answer(client: &Client, question: &str) -> Result<String> {
    let params = [("spoken", question)];
    let v = client
        .get_json(SOURCE_OWNTHINK, "nlp_answer", BOT_URL, &params)
        .await?;
    v.get("data")
        .and_then(|d| d.get("info"))
        .and_then(|i| i.get("text"))
        .and_then(|t| t.as_str())
        .map(|t| t.to_string())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_OWNTHINK,
            message: "missing data.info.text".into(),
        })
}

/// Parse the `avp` slice of an OwnThink knowledge response into [`KnowledgeRow`]s.
#[allow(dead_code)] // offline test entry point; the live path uses `nlp_answer`
pub(crate) fn parse_avp(resp: &Value) -> Result<Vec<KnowledgeRow>> {
    let arr = resp
        .get("data")
        .and_then(|d| d.get("avp"))
        .and_then(|a| a.as_array())
        .ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_OWNTHINK,
            message: "missing data.avp".into(),
        })?;
    Ok(arr
        .iter()
        .map(|pair| KnowledgeRow {
            field: pair.get(0).and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            value: pair.get(1).and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_nlp_ownthink_avp() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/nlp_ownthink.json");
        let txt = std::fs::read_to_string(path).unwrap();
        let v: Value = serde_json::from_str(&txt).unwrap();
        let rows = parse_avp(&v).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].field, "别名");
        assert_eq!(rows[0].value, "AI");
        assert_eq!(rows[1].field, "领域");
        assert_eq!(rows[1].value, "计算机科学");
    }
}
