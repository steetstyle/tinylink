use anyhow::{Context, Result};
use serde::Deserialize;
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

/// Query Prometheus for service graph metrics and build ServiceGraph
pub async fn query_prometheus(prom_url: &str, since: &str) -> Result<ServiceGraph> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to build HTTP client")?;

    let queries = build_queries(since);
    let mut graph = ServiceGraph::new();
    let mut any_ok = false;

    for query in &queries {
        let resp = client
            .get(format!("{}/api/v1/query", prom_url))
            .query(&[("query", query)])
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                let body: PromResponse = r.json().await
                    .context("Failed to parse Prometheus response")?;
                if body.status == "success" {
                    any_ok = true;
                    if let Some(data) = body.data {
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
                        let ep_src = if !item.metric.http_method.is_empty() {
                                format!("{} {}", item.metric.http_method, item.metric.http_route)
                            } else {
                                String::new()
                            };

                            // Extract latency value (quantile in seconds → ms)
                            let latency_ms = item.value.as_ref()
                                .and_then(|v| v.get(1))
                                .and_then(|val| val.as_str())
                                .and_then(|s| s.parse::<f64>().ok())
                                .map(|sec| sec * 1000.0)
                                .unwrap_or(0.0);

                            graph.add_edge(&src, &dst, &ep_src, &ep_src, false, latency_ms, "");
                            }
                        }
                    }
                } else {
                    info!("Prometheus query returned status: {}", body.status);
                }
            }
            Ok(r) => {
                info!("Prometheus query failed with status: {}", r.status());
            }
            Err(e) => {
                info!("Prometheus query error: {}", e);
            }
        }
    }

    if !any_ok {
        anyhow::bail!("All Prometheus queries failed — no data received from {}", prom_url);
    }

    Ok(graph)
}

/// Build candidate PromQL queries (multiple metric name patterns)
fn build_queries(since: &str) -> Vec<String> {
    let metric_names = vec![
        "traces_service_graph_request_server_bucket",
        "traces_service_graph_request_total",
        "traces_service_graph_request_client_bucket",
        "service_graph_request_server_bucket",
        "service_graph_request_total",
    ];

    metric_names.into_iter().map(|name| {
        format!(
            "histogram_quantile(0.95, sum(rate({}[{}])) by (client, server, http_method, http_route, le))",
            name, since
        )
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_queries_multiple_metrics() {
        let queries = build_queries("5m");
        assert_eq!(queries.len(), 5);
        for q in &queries {
            assert!(q.contains("histogram_quantile"));
            assert!(q.contains("[5m]"));
        }
        assert!(queries[0].contains("traces_service_graph_request_server_bucket"));
        assert!(queries[4].contains("service_graph_request_total"));
    }

    #[cfg(feature = "online")]
    #[tokio::test]
    async fn test_query_prometheus_invalid_url() {
        let result = query_prometheus("http://127.0.0.1:99999", "5m").await;
        assert!(result.is_err());
    }
}
