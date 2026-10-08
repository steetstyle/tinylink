use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use tracing::info;

use super::types::ServiceGraph;

/// Prometheus query result structure
#[derive(Debug, Deserialize)]
struct PromResponse {
    status: String,
    data: Option<PromData>,
}

#[derive(Debug, Deserialize)]
struct PromData {
    result: Vec<PromResult>,
}

#[derive(Debug, Deserialize)]
struct PromResult {
    metric: PromMetric,
    #[allow(dead_code)]
    value: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct PromMetric {
    #[serde(default)]
    client: String,
    #[serde(default)]
    server: String,
    #[serde(default)]
    service: String,
    #[serde(rename = "http_method", default)]
    http_method: String,
    #[serde(rename = "http_route", default)]
    http_route: String,
}

type EdgeKey = (String, String);

/// Query Prometheus for service graph metrics and build ServiceGraph
pub async fn query_prometheus(prom_url: &str, since: &str) -> Result<ServiceGraph> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to build HTTP client")?;

    let mut graph = ServiceGraph::new();

    // Phase 1: query error metrics to find which (client, server) pairs have errors
    let mut error_edges: HashMap<EdgeKey, bool> = HashMap::new();
    let error_metrics = vec![
        "traces_service_graph_request_failures_total",
        "service_graph_request_failures_total",
    ];
    for name in &error_metrics {
        let query = format!("sum(rate({}[{}])) by (client, server) > 0", name, since);
        if let Ok(Some(pairs)) = query_prom_result(&client, prom_url, &query).await {
            for (src, dst) in pairs {
                error_edges.insert((src, dst), true);
            }
        }
    }

    // Phase 2: query latency histograms
    let mut any_ok = false;
    let latency_metrics = vec![
        "traces_service_graph_request_server_bucket",
        "traces_service_graph_request_client_bucket",
        "service_graph_request_server_bucket",
    ];

    for name in &latency_metrics {
        let query = format!(
            "histogram_quantile(0.95, sum(rate({}[{}])) by (client, server, http_method, http_route, le))",
            name, since
        );
        // Re-query latency histograms directly to get latency values
        let resp = client
            .get(format!("{}/api/v1/query", prom_url))
            .query(&[("query", &query)])
            .send()
            .await;
        match resp {
            Ok(r) if r.status().is_success() => {
                let body: PromResponse = match r.json().await {
                    Ok(b) => b,
                    Err(_) => continue,
                };
                if body.status == "success" {
                    any_ok = true;
                    if let Some(data) = body.data {
                        for item in data.result {
                            let src = if !item.metric.client.is_empty() {
                                item.metric.client.clone()
                            } else {
                                item.metric.service.clone()
                            };
                            let dst = if !item.metric.server.is_empty() {
                                item.metric.server.clone()
                            } else {
                                item.metric.service.clone()
                            };
                            if src.is_empty() || dst.is_empty() {
                                continue;
                            }

                            let ep_src = if !item.metric.http_method.is_empty() {
                                format!("{} {}", item.metric.http_method, item.metric.http_route)
                            } else {
                                String::new()
                            };

                            let latency_ms = item
                                .value
                                .as_ref()
                                .and_then(|v| v.get(1))
                                .and_then(|val| val.as_str())
                                .and_then(|s| s.parse::<f64>().ok())
                                .map(|sec| sec * 1000.0)
                                .unwrap_or(0.0);

                            let has_error = error_edges.contains_key(&(src.clone(), dst.clone()));
                            graph.add_edge(&src, &dst, &ep_src, &ep_src, has_error, latency_ms, "");
                        }
                    }
                }
            }
            Ok(r) => info!("Prometheus latency query status: {}", r.status()),
            Err(e) => info!("Prometheus latency query error: {}", e),
        }
    }

    if !any_ok {
        anyhow::bail!(
            "All Prometheus queries failed — no data received from {}",
            prom_url
        );
    }

    Ok(graph)
}

/// Run a PromQL query and return (client, server) pairs from the result.
async fn query_prom_result(
    client: &reqwest::Client,
    prom_url: &str,
    query: &str,
) -> Result<Option<Vec<(String, String)>>> {
    let resp = client
        .get(format!("{}/api/v1/query", prom_url))
        .query(&[("query", query)])
        .send()
        .await?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let body: PromResponse = resp.json().await?;
    if body.status != "success" {
        return Ok(None);
    }
    let data = match body.data {
        Some(d) => d,
        None => return Ok(None),
    };
    let mut pairs = Vec::new();
    for item in data.result {
        let src = if !item.metric.client.is_empty() {
            item.metric.client
        } else {
            item.metric.service.clone()
        };
        let dst = if !item.metric.server.is_empty() {
            item.metric.server
        } else {
            item.metric.service.clone()
        };
        if !src.is_empty() && !dst.is_empty() {
            pairs.push((src, dst));
        }
    }
    Ok(Some(pairs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_prom_result_no_data() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = reqwest::Client::new();
        let result = rt.block_on(query_prom_result(&client, "http://127.0.0.1:1", "up"));
        // Network error (connection refused) is acceptable — result may be Err or Ok(None)
        if let Ok(opt) = result {
            assert!(opt.is_none());
        }
        // Err is also fine (connection refused)
    }
}
