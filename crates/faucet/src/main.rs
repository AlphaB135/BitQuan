#![allow(clippy::expect_used)] // Faucet is dev/test tool: compile-time constants are safe

use anyhow::Result;
use dashmap::DashMap;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};
use warp::{Filter, Reply};

lazy_static! {
    // Bech32 address validation (BIP 173): hrp + '1' + 6-90 data chars
    // BitQuan uses "bq" prefix, so pattern is: bq1 + 38-62 characters
    static ref BECH32_REGEX: Regex =
        Regex::new(r"^bq1[a-z0-9]{38,62}$").expect("Invalid regex");
}

#[derive(Debug, Deserialize, Clone)]
struct DripRequest {
    address: String,
}

#[derive(Debug, Serialize)]
struct DripResponse {
    txid: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

struct RateLimiter {
    requests: DashMap<String, Instant>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            requests: DashMap::new(),
        }
    }

    fn check(&self, ip: &str) -> bool {
        let now = Instant::now();
        let duration = Duration::from_secs(60);

        self.requests
            .retain(|_, timestamp| now.duration_since(*timestamp) < duration);

        if self.requests.contains_key(ip) {
            return false;
        }

        self.requests.insert(ip.to_string(), now);
        true
    }
}

#[derive(Clone)]
struct FaucetConfig {
    rpc_url: String,
    rpc_user: String,
    rpc_pass: String,
    drip_amount: f64,
}

#[derive(Deserialize, Serialize)]
struct RpcRequest {
    jsonrpc: String,
    id: String,
    method: String,
    params: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct RpcResponse {
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

/// Extract real client IP for rate limiting.
///
/// SECURITY: By default, uses the socket peer address (cannot be spoofed).
/// Only trusts X-Forwarded-For / X-Real-IP headers when the connection
/// comes from a TRUSTED_PROXY (set via environment variable).
fn extract_real_ip(headers: &warp::http::HeaderMap, socket_addr: Option<SocketAddr>) -> String {
    let peer_ip = socket_addr
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Only trust forwarded headers if TRUSTED_PROXY env var is set
    // and the direct peer IP matches the trusted proxy address.
    let trusted_proxy = env::var("TRUSTED_PROXY").ok();
    let is_trusted = trusted_proxy
        .as_ref()
        .map(|tp| tp == &peer_ip)
        .unwrap_or(false);

    if !is_trusted {
        return peer_ip;
    }

    // Peer is the trusted proxy — extract client IP from forwarded headers
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            // X-Forwarded-For: "client, proxy1, proxy2" — take first (original client)
            if let Some(client_ip) = forwarded_str.split(',').next() {
                let ip = client_ip.trim();
                if !ip.is_empty() {
                    return ip.to_string();
                }
            }
        }
    }

    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(real_ip_str) = real_ip.to_str() {
            let ip = real_ip_str.trim();
            if !ip.is_empty() {
                return ip.to_string();
            }
        }
    }

    // Fallback to socket address
    peer_ip
}

async fn send_to_address(config: &FaucetConfig, address: &str) -> Result<String> {
    // Create client with timeout to prevent thread exhaustion DoS
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {}", e))?;

    let rpc_req = RpcRequest {
        jsonrpc: "2.0".to_string(),
        id: "faucet".to_string(),
        method: "sendtoaddress".to_string(),
        params: vec![
            serde_json::json!(address),
            serde_json::json!(config.drip_amount),
            serde_json::json!("Faucet drip"),
        ],
    };

    let response = client
        .post(&config.rpc_url)
        .basic_auth(&config.rpc_user, Some(&config.rpc_pass))
        .json(&rpc_req)
        .send()
        .await?;

    let rpc_resp: RpcResponse = response.json().await?;

    if let Some(error) = rpc_resp.error {
        anyhow::bail!("RPC error: {}", error);
    }

    let txid = rpc_resp
        .result
        .ok_or_else(|| anyhow::anyhow!("Invalid RPC response: no result"))?;

    if let Some(s) = txid.as_str() {
        Ok(s.to_string())
    } else {
        anyhow::bail!("Invalid RPC response: result is not a string")
    }
}

async fn handle_drip(
    headers: warp::http::HeaderMap,
    socket_addr: Option<SocketAddr>,
    body: DripRequest,
    rate_limiter: Arc<RateLimiter>,
    config: FaucetConfig,
) -> impl Reply {
    // Extract real IP (handles proxy scenarios)
    let ip = extract_real_ip(&headers, socket_addr);

    info!(
        "Received drip request from {} for address {}",
        ip, body.address
    );

    // Check rate limit
    if !rate_limiter.check(&ip) {
        warn!("Rate limit exceeded for IP: {}", ip);
        return warp::reply::json(&ErrorResponse {
            error: "Rate limit exceeded. Please wait 1 minute between requests.".to_string(),
        });
    }

    // Validate address format with regex (BEFORE calling RPC)
    if !BECH32_REGEX.is_match(&body.address) {
        return warp::reply::json(&ErrorResponse {
            error: "Invalid BitQuan address format. Must be bq1... (38-62 characters)".to_string(),
        });
    }

    // Send coins via RPC (with timeout protection)
    match send_to_address(&config, &body.address).await {
        Ok(txid) => {
            info!(
                "Successfully sent {} BQ to {}, txid: {}",
                config.drip_amount, body.address, txid
            );
            warp::reply::json(&DripResponse { txid })
        }
        Err(e) => {
            warn!("Failed to send coins: {}", e);
            warp::reply::json(&ErrorResponse {
                error: format!("Failed to send coins: {}", e),
            })
        }
    }
}

fn index() -> impl Reply {
    warp::reply::html(include_str!("../static/index.html"))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let rpc_url =
        env::var("BITQUAN_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8332".to_string());
    let rpc_user = env::var("BITQUAN_RPC_USER")
        .expect("BITQUAN_RPC_USER env var must be set — refusing to start with default credentials");
    let rpc_pass = env::var("BITQUAN_RPC_PASS")
        .expect("BITQUAN_RPC_PASS env var must be set — refusing to start with default credentials");
    let drip_amount = env::var("FAUCET_DRIP_AMOUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10.0);

    let config = FaucetConfig {
        rpc_url,
        rpc_user,
        rpc_pass,
        drip_amount,
    };

    let rate_limiter = Arc::new(RateLimiter::new());

    let index_route = warp::path::end().map(index);

    // Extract headers for IP detection (warp 0.4.x doesn't have addr::remote())
    let api_drip = warp::post()
        .and(warp::path("api"))
        .and(warp::path("drip"))
        .and(warp::path::end())
        .and(warp::filters::header::headers_cloned())
        .and(warp::body::json())
        .and_then({
            let rate_limiter = rate_limiter.clone();
            let config = config.clone();
            move |headers, body: DripRequest| {
                let rate_limiter = rate_limiter.clone();
                let config = config.clone();
                async move {
                    Ok::<_, warp::Rejection>(
                        handle_drip(headers, None, body, rate_limiter, config).await,
                    )
                }
            }
        });

    let routes = index_route.or(api_drip).with(
        warp::cors()
            .allow_any_origin()
            .allow_methods(vec!["GET", "POST"]),
    );

    let port = env::var("FAUCET_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);

    info!("🚰 BitQuan Faucet starting on port {}", port);
    info!("📡 RPC URL: {}", config.rpc_url);
    info!("💧 Drip amount: {} BQ", config.drip_amount);
    info!("🛡️  Security: Proxy-safe IP extraction, 30s RPC timeout, Regex validation");

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;

    Ok(())
}
