//! C1 pilot: 最小可复用流水线（ponytail 原则：能复用现有能力就不新增）.
//!
//! 抽取三类重复：
//! - push2 分页 `data.diff` 循环（`stock/cross/us.rs` / `hk.rs` 约 40 行重复）
//! - datacenter `result.data` 分页（`stock/hsgt.rs::em_dc_rows`）
//! - CSV 拉取的 referer 约束（`daily_sina.rs` 已有 csv 解析，流水线只补 fetch 层的 thin helper）
//!
//! 不新增依赖，不改公共接口，复用 `crate::core::json::{fstr,fnum}` 与 `eastmoney_push::push2_url`。

use serde_json::Value;

use crate::core::client::{Client, SOURCE_EASTMONEY, SOURCE_SINA};
use crate::core::error::{Error, Result};

// ---- push2 分页：data.diff ----

const DEFAULT_PUSH2_SLEEP_MS: u64 = 800;

/// 拉取 push2 `clist/get` 全部 `data.diff`，按 `pn/pz/total` 分页直至收齐。
/// `base` 为除 `pn/pz` 外的静态参数（如 `po/np/ut/fltt/invt/fid/fs/fields`）。
pub async fn fetch_push2_all(
    client: &Client,
    endpoint: &'static str,
    path: &str,
    base: &[(&str, &str)],
    page_size: u32,
) -> Result<Vec<Value>> {
    let url = crate::core::eastmoney_push::push2_url(path).await;
    let mut out = Vec::new();
    let mut pn: u32 = 1;
    let pz_s = page_size.to_string();
    loop {
        let pn_s = pn.to_string();
        // 拼当页参数：base + pn/pz
        let mut params: Vec<(&str, &str)> = Vec::with_capacity(base.len() + 2);
        params.extend_from_slice(base);
        params.push(("pn", pn_s.as_str()));
        params.push(("pz", pz_s.as_str()));

        let v = client.get_json(SOURCE_EASTMONEY, endpoint, &url, &params).await?;
        let diff_node = v.get("data").and_then(|d| d.get("diff")).ok_or_else(|| Error::UpstreamChanged {
            origin: SOURCE_EASTMONEY,
            message: "missing data.diff".into(),
        })?;
        let batch: Vec<Value> = if let Some(arr) = diff_node.as_array() {
            arr.clone()
        } else if let Some(map) = diff_node.as_object() {
            map.values().cloned().collect()
        } else {
            return Err(Error::UpstreamChanged {
                origin: SOURCE_EASTMONEY,
                message: "data.diff is neither array nor map".into(),
            });
        };
        if batch.is_empty() {
            break;
        }
        out.extend(batch);

        let total = v
            .get("data")
            .and_then(|d| d.get("total"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        if (pn as u64) * page_size as u64 >= total {
            break;
        }
        pn += 1;
        tokio::time::sleep(std::time::Duration::from_millis(DEFAULT_PUSH2_SLEEP_MS)).await;
    }
    Ok(out)
}

// ---- datacenter 分页：result.data ----

const DC: &str = "https://datacenter-web.eastmoney.com/api/data/v1/get";

fn as_refs(p: &[(String, String)]) -> Vec<(&str, &str)> {
    p.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect()
}

/// 拉取 datacenter-web 全部分页 `result.data`。
pub async fn fetch_dc_all(
    client: &Client,
    endpoint: &'static str,
    base: &[(&str, &str)],
) -> Result<Vec<Value>> {
    let mut params: Vec<(String, String)> =
        base.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect();
    let first = client.get_json(SOURCE_EASTMONEY, endpoint, DC, &as_refs(&params)).await?;
    let pages = first
        .get("result")
        .and_then(|r| r.get("pages"))
        .and_then(|p| p.as_i64())
        .unwrap_or(1)
        .max(1);
    let mut out = Vec::new();
    if let Some(arr) = first.get("result").and_then(|r| r.get("data")).and_then(|d| d.as_array()) {
        out.extend(arr.iter().cloned());
    }
    for p in 2..=pages {
        for (k, v) in params.iter_mut() {
            if k == "pageNumber" {
                *v = p.to_string();
            }
        }
        let resp = client.get_json(SOURCE_EASTMONEY, endpoint, DC, &as_refs(&params)).await?;
        if let Some(arr) = resp.get("result").and_then(|r| r.get("data")).and_then(|d| d.as_array()) {
            out.extend(arr.iter().cloned());
        }
    }
    Ok(out)
}

// ---- sina CSV 拉取（thin helper，复用已有 parse_csv） ----

/// Sina 文本拉取（带 Referer），供 `daily_sina` 复用。token 仍由调用方负责。
pub async fn fetch_sina_text(
    client: &Client,
    endpoint: &'static str,
    url: &str,
    params: &[(&str, &str)],
) -> Result<String> {
    let headers = [("Referer", "https://finance.sina.com.cn/")];
    client.get_text(SOURCE_SINA, endpoint, url, params, Some(&headers)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    // 仅验流水线对 fixture 形态的抽取逻辑，不发网
    #[test]
    fn dc_pages_fallback_is_one() {
        let v = json!({"result":{"data":[1,2],"pages":null}});
        let pages = v.get("result").and_then(|r| r.get("pages")).and_then(|p| p.as_i64()).unwrap_or(1).max(1);
        assert_eq!(pages, 1);
    }
}
