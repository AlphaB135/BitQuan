//! HTTP server for handling JSON-RPC requests

use crate::{
    error_codes, methods, tls::TlsConfig, validation::InputValidator, JsonRpcRequest,
    JsonRpcResponse, RpcConfig,
};
use base64::Engine;
use chrono;
use http::StatusCode;
use once_cell::sync::Lazy;
use serde_json::json;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_rustls::server::TlsStream;
use tracing::{error, info, warn};

/// Security event types
#[derive(Debug, Clone)]
pub enum SecurityEventType {
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Authentication failed
    AuthenticationFailed,
    /// Input validation failed
    InputValidationFailed,
    /// Suspicious request detected
    SuspiciousRequest,
    /// Connection established
    ConnectionEstablished,
    /// Connection terminated
    ConnectionTerminated,
    /// Request processed successfully
    RequestProcessed,
    /// Slowloris attack detected
    SlowlorisAttackDetected,
    /// Repeated authentication failures
    RepeatedAuthFailures,
    /// Injection attempt detected
    InjectionAttempt,
}

/// Security severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecuritySeverity {
    /// Informational only
    Info,
    /// Potentially suspicious
    Low,
    /// Definitely suspicious
    Medium,
    /// Dangerous activity
    High,
    /// Critical security threat
    Critical,
}

/// Security event for logging and monitoring
#[derive(Debug, Clone)]
pub struct SecurityEvent {
    /// Event timestamp (UTC)
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Client IP address
    pub client_ip: String,
    /// Event type
    pub event_type: SecurityEventType,
    /// Event severity
    pub severity: SecuritySeverity,
    /// Event details
    pub details: serde_json::Value,
    /// Request ID (if available)
    pub request_id: Option<String>,
}

impl SecurityEvent {
    /// Create a new security event
    pub fn new(
        client_ip: String,
        event_type: SecurityEventType,
        severity: SecuritySeverity,
        details: serde_json::Value,
    ) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            client_ip,
            event_type,
            severity,
            details,
            request_id: None,
        }
    }

    /// Convert to JSON for logging
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "timestamp": self.timestamp.to_rfc3339(),
            "client_ip": self.client_ip,
            "event_type": format!("{:?}", self.event_type),
            "severity": format!("{:?}", self.severity),
            "details": self.details,
            "request_id": self.request_id
        })
    }

    /// Check if event should trigger an alert
    pub fn should_alert(&self) -> bool {
        matches!(
            self.severity,
            SecuritySeverity::High | SecuritySeverity::Critical
        )
    }

    /// Log the event using tracing
    pub fn log(&self) {
        let message = format!(
            "Security event from {}: {:?} - {:?}",
            self.client_ip, self.event_type, self.severity
        );

        match self.severity {
            SecuritySeverity::Info => info!(message = %message, event = ?self.to_json()),
            SecuritySeverity::Low => warn!(message = %message, event = ?self.to_json()),
            SecuritySeverity::Medium => warn!(message = %message, event = ?self.to_json()),
            SecuritySeverity::High => error!(message = %message, event = ?self.to_json()),
            SecuritySeverity::Critical => error!(message = %message, event = ?self.to_json()),
        }

        // Trigger alert if needed
        if self.should_alert() {
            self.trigger_alert();
        }
    }

    /// Trigger alert for high-priority security events
    fn trigger_alert(&self) {
        // In production, this could integrate with:
        // - PagerDuty, OpsGenie, or other alerting systems
        // - Slack/Teams notifications
        // - Email alerts
        // - SIEM systems

        error!(
            "🚨 SECURITY ALERT: {:?} from {} - {}",
            self.event_type,
            self.client_ip,
            serde_json::to_string_pretty(&self.details).unwrap_or_default()
        );
    }
}

/// Authentication for RPC server (JWT only)
pub type AuthMethod = Arc<crate::jwt::JwtAuth>;

/// Simple HTTP JSON-RPC server
pub struct RpcServer<T> {
    handler: Arc<T>,
    addr: String,
    auth: Option<AuthMethod>,
    config: RpcConfig,
    limiter: Arc<Mutex<HashMap<IpAddr, TokenBucket>>>,
    auth_backoff: Arc<Mutex<HashMap<IpAddr, BackoffState>>>,
    tls: Option<Arc<TlsConfig>>,
    force_tls: bool,
    basic_auth: Option<(String, String)>,
    validator: Arc<InputValidator>,
}

impl<T: methods::RpcMethods + Send + Sync + 'static> RpcServer<T> {
    /// Create a new RPC server with the given handler and configuration
    pub fn new(
        handler: T,
        addr: String,
        jwt_auth: crate::jwt::JwtAuth,
        config: RpcConfig,
        basic_auth: Option<(String, String)>,
    ) -> Self {
        let require_tls = config.require_tls;
        Self {
            handler: Arc::new(handler),
            addr,
            auth: Some(Arc::new(jwt_auth)),
            config,
            limiter: Arc::new(Mutex::new(HashMap::new())),
            auth_backoff: Arc::new(Mutex::new(HashMap::new())),
            tls: None,
            force_tls: require_tls,
            basic_auth,
            validator: Arc::new(InputValidator::default()),
        }
    }

    /// Set TLS configuration for the server
    pub fn with_tls_config(mut self, tls: TlsConfig) -> Self {
        self.tls = Some(Arc::new(tls));
        self
    }

    /// Set whether TLS is required for connections
    pub fn require_tls(mut self, required: bool) -> Self {
        self.force_tls = required;
        self
    }

    /// Start the RPC server and begin accepting connections
    pub async fn serve(&self) -> std::io::Result<()> {
        if self.force_tls && self.tls.is_none() {
            return Err(std::io::Error::other(
                "TLS is required but no TLS configuration was provided",
            ));
        }
        let listener = TcpListener::bind(&self.addr).await?;
        self.accept_loop(listener).await
    }

    /// Start the RPC server with a specific listener and shutdown signal
    pub fn serve_with_listener_and_shutdown(
        self,
        listener: TcpListener,
        shutdown_signal: Option<tokio::sync::mpsc::Receiver<()>>,
    ) -> std::io::Result<()> {
        if self.force_tls && self.tls.is_none() {
            return Err(std::io::Error::other(
                "TLS is required but no TLS configuration was provided",
            ));
        }

        let bound_addr = listener.local_addr()?;
        println!("RPC server listening on {}", bound_addr);

        // Create runtime for async operations
        let rt = tokio::runtime::Runtime::new()?;

        rt.block_on(async {
            let shutdown_signal = async {
                if let Some(mut rx) = shutdown_signal {
                    let _ = rx.recv().await;
                }
            };

            tokio::select! {
                result = self.accept_loop_async(listener) => result,
                _ = shutdown_signal => {
                    println!("RPC server shutting down...");
                    Ok(())
                }
            }
        })
    }

    async fn accept_loop_async(&self, listener: TcpListener) -> std::io::Result<()> {
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    self.spawn_worker(stream, Some(peer_addr.ip()));
                }
                Err(e) => eprintln!("Connection error: {}", e),
            }
        }
    }

    async fn accept_loop(&self, listener: TcpListener) -> std::io::Result<()> {
        let bound_addr = listener.local_addr()?;
        println!("RPC server listening on {}", bound_addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    self.spawn_worker(stream, Some(peer_addr.ip()));
                }
                Err(e) => eprintln!("Connection error: {}", e),
            }
        }
    }

    fn spawn_worker(&self, stream: TcpStream, peer: Option<IpAddr>) {
        let handler = Arc::clone(&self.handler);
        let auth = self.auth.clone();
        let config = self.config.clone();
        let limiter = Arc::clone(&self.limiter);
        let auth_backoff = Arc::clone(&self.auth_backoff);
        let tls = self.tls.clone();
        let force_tls = self.force_tls;
        let basic_auth = self.basic_auth.clone();
        let validator = Arc::clone(&self.validator);

        tokio::spawn(async move {
            let peer_ip = peer
                .or_else(|| stream.peer_addr().ok().map(|addr| addr.ip()))
                .unwrap_or(IpAddr::from([127, 0, 0, 1]));
            let options = ConnectionOptions {
                config: &config,
                limiter: &limiter,
                auth_backoff: &auth_backoff,
                tls: tls.as_ref(),
                force_tls,
                basic_auth,
                validator: &validator,
                jwt_auth: auth.as_ref(),
            };
            if let Err(e) = handle_connection(stream, peer_ip, handler.as_ref(), options).await {
                if e.kind() != std::io::ErrorKind::ConnectionReset
                    && e.kind() != std::io::ErrorKind::BrokenPipe
                {
                    eprintln!("Error handling connection: {}", e);
                }
            }
        });
    }
}

struct ConnectionOptions<'a> {
    config: &'a RpcConfig,
    limiter: &'a Arc<Mutex<HashMap<IpAddr, TokenBucket>>>,
    auth_backoff: &'a Arc<Mutex<HashMap<IpAddr, BackoffState>>>,
    tls: Option<&'a Arc<TlsConfig>>,
    force_tls: bool,
    basic_auth: Option<(String, String)>,
    validator: &'a Arc<InputValidator>,
    jwt_auth: Option<&'a AuthMethod>,
}

enum RpcStream {
    Plain(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
}

// Manual implementation of Read/Write traits for the enum
impl AsyncRead for RpcStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            RpcStream::Plain(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            RpcStream::Tls(s) => std::pin::Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for RpcStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            RpcStream::Plain(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            RpcStream::Tls(s) => std::pin::Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            RpcStream::Plain(s) => std::pin::Pin::new(s).poll_flush(cx),
            RpcStream::Tls(s) => std::pin::Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            RpcStream::Plain(s) => std::pin::Pin::new(s).poll_shutdown(cx),
            RpcStream::Tls(s) => std::pin::Pin::new(s).poll_shutdown(cx),
        }
    }
}

async fn upgrade_to_tls(stream: TcpStream, tls_config: &TlsConfig) -> std::io::Result<RpcStream> {
    let acceptor = tokio_rustls::TlsAcceptor::from(tls_config.server_config());
    let tls_stream = acceptor.accept(stream).await?;
    Ok(RpcStream::Tls(Box::new(tls_stream)))
}

async fn handle_connection<T: methods::RpcMethods>(
    mut stream: TcpStream,
    peer_ip: IpAddr,
    handler: &T,
    options: ConnectionOptions<'_>,
) -> std::io::Result<()> {
    let start = Instant::now();
    let config = options.config;

    // Log connection established
    let connection_event = SecurityEvent::new(
        peer_ip.to_string(),
        SecurityEventType::ConnectionEstablished,
        SecuritySeverity::Info,
        json!({
            "action": "connection_established",
            "tls_required": options.force_tls,
            "has_tls_config": options.tls.is_some(),
        }),
    );
    connection_event.log();

    // Apply rate limiting before processing
    if !check_rate_limit(peer_ip, options.limiter, config).await {
        // Log rate limit exceeded event
        let rate_limit_event = SecurityEvent::new(
            peer_ip.to_string(),
            SecurityEventType::RateLimitExceeded,
            SecuritySeverity::Medium,
            json!({
                "action": "connection_blocked",
                "reason": "rate_limit_exceeded",
                "cooldown_seconds": config.cooldown_duration.as_secs()
            }),
        );
        rate_limit_event.log();

        let cooldown = apply_cooldown(peer_ip, options.limiter, config).await;

        // Send rate limit error response
        let response = json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32603,
                "message": "Rate limit exceeded",
                "data": format!("Retry after {} seconds", cooldown.as_secs())
            },
            "id": null
        });

        let response_json = serde_json::to_string(&response).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to serialize response",
            )
        })?;
        let response_str = format!(
            "HTTP/1.1 429 Too Many Requests\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Retry-After: {}\r\n\
             \r\n\
             {}",
            response_json.len(),
            cooldown.as_secs(),
            response_json
        );

        return stream.write_all(response_str.as_bytes()).await;
    }

    // Check authentication backoff
    if apply_auth_backoff(peer_ip, options.auth_backoff).await {
        let backoff_map = options.auth_backoff.lock().await;

        // Log authentication backoff event
        let auth_backoff_event = SecurityEvent::new(
            peer_ip.to_string(),
            SecurityEventType::RepeatedAuthFailures,
            SecuritySeverity::High,
            json!({
                "action": "authentication_blocked",
                "reason": "repeated_failures",
                "failure_count": backoff_map.get(&peer_ip).map(|s| s.failed_attempts).unwrap_or(0),
                "locked_until": backoff_map.get(&peer_ip).and_then(|s| s.locked_until).map(|_| "locked".to_string())
            }),
        );
        auth_backoff_event.log();
        if let Some(state) = backoff_map.get(&peer_ip) {
            if let Some(remaining_time) = state.remaining_lock_time() {
                let response = json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32603,
                        "message": "Too many authentication attempts",
                        "data": format!("Account locked. Try again in {} seconds", remaining_time.as_secs())
                    },
                    "id": null
                });

                let response_json = serde_json::to_string(&response).map_err(|_| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Failed to serialize response",
                    )
                })?;
                let response_str = format!(
                    "HTTP/1.1 403 Forbidden\r\n\
                     Content-Type: application/json\r\n\
                     Content-Length: {}\r\n\
                     \r\n\
                     {}",
                    response_json.len(),
                    response_json
                );

                return stream.write_all(response_str.as_bytes()).await;
            }
        }
    }

    let mut channel = match (options.tls, options.force_tls) {
        (Some(tls_config), _) => upgrade_to_tls(stream, tls_config.as_ref()).await?,
        (None, true) => return send_upgrade_required(stream, peer_ip, start).await,
        (None, false) => RpcStream::Plain(stream),
    };

    let mut buf_reader = BufReader::new(&mut channel);

    // --- Async HTTP Header Parsing ---
    let mut header_buf = Vec::new();
    let header_read_timeout = Duration::from_millis(config.header_read_timeout_ms);

    loop {
        let byte_result = tokio::time::timeout(header_read_timeout, buf_reader.read_u8()).await;
        let byte = byte_result.map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::TimedOut, "header read timeout")
        })??;
        header_buf.push(byte);
        if header_buf.len() > config.max_header_bytes {
            return send_header_too_large(buf_reader.into_inner()).await;
        }
        if header_buf.ends_with(b"\r\n\r\n") {
            break;
        }
    }

    let mut http_headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut http_headers);
    let status = req
        .parse(&header_buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    if !status.is_complete() {
        return send_bad_request(buf_reader.into_inner()).await;
    }

    let content_length = req
        .headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("content-length"))
        .and_then(|h| std::str::from_utf8(h.value).ok()?.parse::<usize>().ok())
        .unwrap_or(0);

    // --- Basic Authentication Check ---
    if let Some((username, password)) = &options.basic_auth {
        let auth_header = req
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("authorization"))
            .and_then(|h| std::str::from_utf8(h.value).ok());

        let authorized = if let Some(header_val) = auth_header {
            if let Some(base64_creds) = header_val.strip_prefix("Basic ") {
                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(base64_creds)
                {
                    if let Ok(creds_str) = std::str::from_utf8(&decoded) {
                        if let Some((u, p)) = creds_str.split_once(':') {
                            u == username && p == password
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if !authorized {
            // Log authentication failure
            let auth_event = SecurityEvent::new(
                peer_ip.to_string(),
                SecurityEventType::AuthenticationFailed,
                SecuritySeverity::Medium,
                json!({
                    "action": "authentication_failed",
                    "auth_type": "basic_auth",
                    "has_auth_header": auth_header.is_some(),
                    "user_agent": req.headers.iter()
                        .find(|h| h.name.eq_ignore_ascii_case("user-agent"))
                        .and_then(|h| std::str::from_utf8(h.value).ok())
                }),
            );
            auth_event.log();

            let response = "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"BitQuan RPC\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let stream_inner = buf_reader.into_inner();
            stream_inner.write_all(response.as_bytes()).await?;
            stream_inner.flush().await?;
            stream_inner.shutdown().await?;
            return Ok(());
        }
    }
    // --- End Basic Authentication Check ---

    // --- JWT Bearer Token Authentication ---
    if config.require_jwt_auth {
        let auth_header = req
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("authorization"))
            .and_then(|h| std::str::from_utf8(h.value).ok());

        let jwt_result = verify_jwt_token(
            auth_header,
            options.jwt_auth,
            config.jwt_max_age_secs,
            peer_ip,
        );

        if let Err(jwt_error) = jwt_result {
            // Log JWT authentication failure
            let auth_event = SecurityEvent::new(
                peer_ip.to_string(),
                SecurityEventType::AuthenticationFailed,
                SecuritySeverity::Medium,
                json!({
                    "action": "authentication_failed",
                    "auth_type": "jwt_bearer",
                    "error": jwt_error,
                    "has_auth_header": auth_header.is_some(),
                }),
            );
            auth_event.log();

            // Record auth failure for backoff
            {
                let mut backoff_map = options.auth_backoff.lock().await;
                let state = backoff_map
                    .entry(peer_ip)
                    .or_insert_with(|| BackoffState::new(5, Duration::from_secs(900)));
                state.record_failure();
            }

            let error_body = serde_json::json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32001,
                    "message": "Unauthorized",
                    "data": jwt_error
                },
                "id": null
            });
            let error_json = serde_json::to_string(&error_body).unwrap_or_default();
            let response = format!(
                "HTTP/1.1 401 Unauthorized\r\n\
                 WWW-Authenticate: Bearer realm=\"BitQuan RPC\"\r\n\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\r\n{}",
                error_json.len(),
                error_json
            );
            let stream_inner = buf_reader.into_inner();
            stream_inner.write_all(response.as_bytes()).await?;
            stream_inner.flush().await?;
            stream_inner.shutdown().await?;
            return Ok(());
        }
    }
    // --- End JWT Bearer Token Authentication ---

    // --- End Header Parsing ---

    if content_length > config.max_body_bytes {
        return send_payload_too_large(buf_reader.into_inner()).await;
    }

    let mut body = vec![0u8; content_length];
    let body_read_timeout = Duration::from_millis(config.body_read_timeout_ms);

    tokio::time::timeout(body_read_timeout, buf_reader.read_exact(&mut body))
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "body read timeout"))??;

    let stream = buf_reader.into_inner();

    // Validate input before parsing
    let request_value = match serde_json::from_slice::<serde_json::Value>(&body) {
        Ok(value) => value,
        Err(e) => {
            let validation_event = SecurityEvent::new(
                peer_ip.to_string(),
                SecurityEventType::InputValidationFailed,
                SecuritySeverity::Medium,
                json!({
                    "action": "json_parse_failed",
                    "reason": "invalid_json_format",
                    "error": e.to_string(),
                    "body_size": body.len()
                }),
            );
            validation_event.log();

            let err_resp = JsonRpcResponse::error(
                serde_json::Value::Null,
                error_codes::PARSE_ERROR,
                format!("Parse error: {e}"),
            );
            return respond_json(stream, &err_resp, config).await;
        }
    };

    // Validate using InputValidator
    if let Err(e) = options.validator.validate_request(&request_value) {
        let validation_event = SecurityEvent::new(
            peer_ip.to_string(),
            SecurityEventType::InputValidationFailed,
            SecuritySeverity::Medium,
            json!({
                "action": "request_validation_failed",
                "reason": "input_validation_error",
                "error": e.to_string(),
                "request": request_value
            }),
        );
        validation_event.log();

        let err_resp = JsonRpcResponse::error(
            serde_json::Value::Null,
            error_codes::INVALID_PARAMS,
            format!("Invalid request: {e}"),
        );
        return respond_json(stream, &err_resp, config).await;
    }

    let json_request: JsonRpcRequest = match serde_json::from_value(request_value) {
        Ok(req) => req,
        Err(e) => {
            let err_resp = JsonRpcResponse::error(
                serde_json::Value::Null,
                error_codes::PARSE_ERROR,
                format!("Parse error: {e}"),
            );
            return respond_json(stream, &err_resp, config).await;
        }
    };

    let json_response = if json_request.jsonrpc != "2.0" {
        // Log invalid JSON-RPC version
        let version_event = SecurityEvent::new(
            peer_ip.to_string(),
            SecurityEventType::InputValidationFailed,
            SecuritySeverity::Medium,
            json!({
                "action": "invalid_jsonrpc_version",
                "method": json_request.method,
                "expected_version": "2.0",
                "actual_version": json_request.jsonrpc
            }),
        );
        version_event.log();

        JsonRpcResponse::error(
            json_request.id,
            error_codes::INVALID_REQUEST,
            "Invalid JSON-RPC version".to_string(),
        )
    } else {
        let response = methods::dispatch_call(
            handler,
            &json_request.method,
            json_request.params,
            json_request.id,
        )
        .await;

        // Log successful request processing
        let success_event = SecurityEvent::new(
            peer_ip.to_string(),
            SecurityEventType::RequestProcessed,
            SecuritySeverity::Info,
            json!({
                "action": "request_processed_successfully",
                "method": json_request.method,
                "has_error": response.error.is_some(),
                "processing_time_ms": start.elapsed().as_millis()
            }),
        );
        success_event.log();

        response
    };

    let result = respond_json(stream, &json_response, config).await;

    // Log connection termination
    let termination_event = SecurityEvent::new(
        peer_ip.to_string(),
        SecurityEventType::ConnectionTerminated,
        SecuritySeverity::Info,
        json!({
            "action": "connection_terminated",
            "duration_ms": start.elapsed().as_millis(),
            "success": result.is_ok()
        }),
    );
    termination_event.log();

    result
}

async fn respond_json(
    stream: &mut RpcStream,
    response: &JsonRpcResponse,
    config: &RpcConfig,
) -> std::io::Result<()> {
    let response_body = serde_json::to_string(response).unwrap_or_else(|_| r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"internal serialization error"},"id":null}"# .to_string());
    let security_headers = build_security_headers(config);
    let http_response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n{}",
        response_body.len(),
        security_headers,
        response_body
    );
    stream.write_all(http_response.as_bytes()).await?;
    stream.flush().await?;
    stream.shutdown().await?;
    Ok(())
}

async fn send_bad_request(stream: &mut RpcStream) -> std::io::Result<()> {
    let response = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    stream.shutdown().await
}

async fn send_payload_too_large(stream: &mut RpcStream) -> std::io::Result<()> {
    let response =
        "HTTP/1.1 413 Payload Too Large\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    stream.shutdown().await
}

async fn send_header_too_large(stream: &mut RpcStream) -> std::io::Result<()> {
    let response = "HTTP/1.1 431 Request Header Fields Too Large\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    stream.shutdown().await
}

async fn send_upgrade_required(
    mut stream: TcpStream,
    _peer_ip: IpAddr,
    _start: Instant,
) -> std::io::Result<()> {
    let body = r#"{"error":"TLS Required","message":"This server requires HTTPS. Please upgrade your connection to HTTPS."}"#;
    let response = format!(
        "HTTP/1.1 426 Upgrade Required\r\nUpgrade: TLS/1.3, HTTP/1.1\r\nConnection: Upgrade\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

fn build_security_headers(config: &RpcConfig) -> String {
    let mut headers = vec![
        // Security headers
        "X-Content-Type-Options: nosniff".to_string(),
        "X-Frame-Options: DENY".to_string(),
        "X-XSS-Protection: 1; mode=block".to_string(),
        "Referrer-Policy: strict-origin-when-cross-origin".to_string(),
        "Content-Security-Policy: default-src 'none'; script-src 'none'; object-src 'none';"
            .to_string(),
    ];

    // HSTS if HTTPS is enabled
    if config.require_tls && config.enable_hsts {
        let max_age = config.hsts_max_age;
        let include_subdomains = if config.hsts_include_subdomains {
            "; includeSubDomains"
        } else {
            ""
        };
        let hsts_header = format!(
            "Strict-Transport-Security: max-age={}{}",
            max_age, include_subdomains
        );
        headers.push(hsts_header);
    }

    // Remove server signature
    headers.push("Server: BitQuan".to_string());

    headers.join("\r\n")
}

// Dummy structs and functions for compilation
/// Rate limiting token bucket for IP addresses
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: u32,
    max_tokens: u32,
    refill_rate: u32,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_tokens: u32, refill_rate: u32) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn consume(&mut self, tokens: u32) -> bool {
        self.refill();
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let tokens_to_add = (elapsed.as_secs() as u32 * self.refill_rate) / 60;

        if tokens_to_add > 0 {
            self.tokens = (self.tokens + tokens_to_add).min(self.max_tokens);
            self.last_refill = now;
        }
    }

    fn reset(&mut self) {
        self.tokens = self.max_tokens;
        self.last_refill = Instant::now();
    }
}

/// Authentication backoff state for failed attempts
#[derive(Debug, Clone)]
struct BackoffState {
    failed_attempts: u32,
    locked_until: Option<Instant>,
    max_attempts: u32,
    lockout_duration: Duration,
}

impl BackoffState {
    fn new(max_attempts: u32, lockout_duration: Duration) -> Self {
        Self {
            failed_attempts: 0,
            locked_until: None,
            max_attempts,
            lockout_duration,
        }
    }

    fn record_failure(&mut self) -> bool {
        self.failed_attempts += 1;

        if self.failed_attempts >= self.max_attempts {
            self.locked_until = Some(Instant::now() + self.lockout_duration);
            true // Should apply backoff
        } else {
            false
        }
    }

    #[allow(dead_code)]
    fn record_success(&mut self) {
        self.failed_attempts = 0;
        self.locked_until = None;
    }

    fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            Instant::now() < locked_until
        } else {
            false
        }
    }

    fn remaining_lock_time(&self) -> Option<Duration> {
        if let Some(locked_until) = self.locked_until {
            let now = Instant::now();
            if now < locked_until {
                Some(locked_until - now)
            } else {
                None
            }
        } else {
            None
        }
    }
}

// These are no longer used in the async version but are kept to avoid breaking other parts of the code if they are used elsewhere.
// In a real scenario, these would be removed or refactored.
/// Apply connection cooldown for rate limit violations
async fn apply_cooldown(
    ip: IpAddr,
    limiter: &Arc<Mutex<HashMap<IpAddr, TokenBucket>>>,
    config: &RpcConfig,
) -> Duration {
    let mut limiter_map = limiter.lock().await;

    if let Some(bucket) = limiter_map.get_mut(&ip) {
        // Reset bucket and apply longer cooldown
        bucket.reset();
    }

    config.cooldown_duration
}
/// Resolve client IP considering proxy headers
#[allow(dead_code)]
fn resolve_client_ip(peer_ip: IpAddr, headers: &[String], config: &RpcConfig) -> IpAddr {
    if !config.trust_proxy {
        return peer_ip;
    }

    // Check for X-Forwarded-For header
    for header in headers {
        if header.to_lowercase().starts_with("x-forwarded-for:") {
            if let Some(ip_str) = header.split(':').nth(1).map(|s| s.trim()) {
                if let Ok(ip) = ip_str.parse::<IpAddr>() {
                    // Take the first IP in the chain (original client)
                    return ip;
                }
            }
        }
    }

    peer_ip
}
/// Apply authentication backoff for failed attempts
async fn apply_auth_backoff(
    ip: IpAddr,
    backoff: &Arc<Mutex<HashMap<IpAddr, BackoffState>>>,
) -> bool {
    let mut backoff_map = backoff.lock().await;

    let state = backoff_map.entry(ip).or_insert_with(|| {
        BackoffState::new(
            5,                        // Max 5 failed attempts
            Duration::from_secs(900), // 15 minute lockout
        )
    });

    if state.is_locked() {
        return true; // Already locked
    }

    state.record_failure()
}
/// Reset authentication backoff after successful authentication
#[allow(dead_code)]
async fn reset_auth_backoff(ip: IpAddr, backoff: &Arc<Mutex<HashMap<IpAddr, BackoffState>>>) {
    let mut backoff_map = backoff.lock().await;

    if let Some(state) = backoff_map.get_mut(&ip) {
        state.record_success();
    }
}

/// Apply rate limiting based on client IP and configuration
async fn check_rate_limit(
    ip: IpAddr,
    limiter: &Arc<Mutex<HashMap<IpAddr, TokenBucket>>>,
    config: &RpcConfig,
) -> bool {
    let mut limiter_map = limiter.lock().await;

    let bucket = limiter_map
        .entry(ip)
        .or_insert_with(|| TokenBucket::new(config.rate_limit_requests, config.rate_limit_window));

    bucket.consume(1) // Each request consumes 1 token
}
#[allow(dead_code)]
static METRICS: Lazy<RpcMetrics> = Lazy::new(RpcMetrics::default);
#[derive(Default)]
struct RpcMetrics {
    // ... fields
}
impl RpcMetrics {
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    fn record(
        &self,
        _status: StatusCode,
        _latency_ms: u64,
        _rate_limited: bool,
        _body_limit: bool,
        _auth_fail: bool,
        _header_limit: bool,
        _body_timeout: bool,
    ) {
    }
}

/// Verify JWT Bearer token from Authorization header.
///
/// Validates:
/// 1. Authorization header is present and starts with "Bearer "
/// 2. Token signature is valid
/// 3. Token is not expired
/// 4. Token `iat` (issued at) is within acceptable age
///
/// # Arguments
/// * `auth_header` - The Authorization header value
/// * `jwt_auth` - The JWT authentication manager
/// * `max_age_secs` - Maximum age for the `iat` claim
/// * `peer_ip` - Client IP for logging
///
/// # Returns
/// * `Ok(())` if token is valid
/// * `Err(String)` with error message if validation fails
fn verify_jwt_token(
    auth_header: Option<&str>,
    jwt_auth: Option<&AuthMethod>,
    max_age_secs: u64,
    peer_ip: IpAddr,
) -> Result<(), String> {
    // Check if JWT auth is configured
    let jwt = jwt_auth.ok_or("JWT authentication not configured")?;

    // Check Authorization header
    let header = auth_header.ok_or("Missing Authorization header")?;

    // Extract Bearer token
    let token = header
        .strip_prefix("Bearer ")
        .ok_or("Authorization header must use Bearer scheme")?;

    // Verify token signature and expiration
    let claims = jwt.verify_token(token)?;

    // Check token freshness (iat claim)
    let now = chrono::Utc::now().timestamp();
    let token_age = now.saturating_sub(claims.iat);

    if token_age < 0 {
        warn!(
            "JWT token from {} has future iat: {} seconds in the future",
            peer_ip, -token_age
        );
        return Err("Token issued in the future".to_string());
    }

    if token_age > max_age_secs as i64 {
        info!(
            "JWT token from {} is stale: {} seconds old (max: {})",
            peer_ip, token_age, max_age_secs
        );
        return Err(format!(
            "Token too old: {} seconds (max: {})",
            token_age, max_age_secs
        ));
    }

    // Token is valid
    info!(
        "JWT authentication successful for user '{}' (role: {}) from {}",
        claims.sub, claims.role, peer_ip
    );

    Ok(())
}
