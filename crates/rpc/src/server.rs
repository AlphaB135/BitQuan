//! HTTP server for handling JSON-RPC requests

use crate::{error_codes, methods, tls::TlsConfig, JsonRpcRequest, JsonRpcResponse, RpcConfig};
use http::StatusCode;
use once_cell::sync::Lazy;
use rustls::{ServerConnection, StreamOwned};
use serde_json::json;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Shutdown, TcpListener, TcpStream};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Basic authentication configuration for RPC server.
#[derive(Clone, Debug)]
pub struct RpcAuth {
    username: String,
    password: String,
}

impl RpcAuth {
    /// Creates a new auth configuration from username/password strings.
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }

    fn matches(&self, provided: &str) -> bool {
        provided == format!("{}:{}", self.username, self.password)
    }

    /// Username accessor (primarily for tests).
    #[cfg(test)]
    pub(crate) fn username(&self) -> &str {
        &self.username
    }

    /// Password accessor (primarily for tests).
    #[cfg(test)]
    pub(crate) fn password(&self) -> &str {
        &self.password
    }
}

/// Simple HTTP JSON-RPC server
pub struct RpcServer<T> {
    handler: Arc<T>,
    addr: String,
    auth: Option<RpcAuth>,
    config: RpcConfig,
    limiter: Arc<Mutex<HashMap<IpAddr, TokenBucket>>>,
    auth_backoff: Arc<Mutex<HashMap<IpAddr, BackoffState>>>,
    tls: Option<Arc<TlsConfig>>,
    force_tls: bool,
}

impl<T: methods::RpcMethods + Send + Sync + 'static> RpcServer<T> {
    /// Create new RPC server
    pub fn new(handler: T, addr: String) -> Self {
        Self::with_auth(handler, addr, None, RpcConfig::default())
    }

    /// Create RPC server with optional authentication.
    pub fn with_auth(handler: T, addr: String, auth: Option<RpcAuth>, config: RpcConfig) -> Self {
        Self {
            handler: Arc::new(handler),
            addr,
            auth,
            config,
            limiter: Arc::new(Mutex::new(HashMap::new())),
            auth_backoff: Arc::new(Mutex::new(HashMap::new())),
            tls: None,
            force_tls: config.require_tls,
        }
    }

    /// Attach a TLS configuration to the server (does not automatically enforce TLS).
    pub fn with_tls_config(mut self, tls: TlsConfig) -> Self {
        self.tls = Some(Arc::new(tls));
        self
    }

    /// Require all connections to use TLS (typically for mainnet deployments).
    pub fn require_tls(mut self, required: bool) -> Self {
        self.force_tls = required;
        self
    }

    /// Start serving requests (blocking)
    pub fn serve(&self) -> std::io::Result<()> {
        if self.force_tls && self.tls.is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "TLS is required but no TLS configuration was provided",
            ));
        }
        let listener = TcpListener::bind(&self.addr)?;
        self.accept_loop(listener, None)
    }

    #[cfg(test)]
    pub(crate) fn serve_with_listener_and_shutdown(
        &self,
        listener: TcpListener,
        shutdown: Option<Receiver<()>>,
    ) -> std::io::Result<()> {
        self.accept_loop(listener, shutdown)
    }

    fn accept_loop(
        &self,
        listener: TcpListener,
        mut shutdown: Option<Receiver<()>>,
    ) -> std::io::Result<()> {
        let bound_addr = listener.local_addr()?;
        println!("RPC server listening on {}", bound_addr);
        println!(
            "RPC config: max_body_bytes={} rl_burst={} rl_refill_per_sec={} conn_cooldown_ms={} require_tls={}",
            self.config.max_body_bytes,
            self.config.rl_burst,
            self.config.rl_refill_per_sec,
            self.config.conn_cooldown_ms,
            self.force_tls
        );
        println!("RPC health endpoint: GET /health (no auth)");

        if shutdown.is_none() {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let peer = stream.peer_addr().ok().map(|addr| addr.ip());
                        self.spawn_worker(stream, peer);
                    }
                    Err(e) => eprintln!("Connection error: {}", e),
                }
            }
            return Ok(());
        }

        listener.set_nonblocking(true)?;

        loop {
            if let Some(rx) = shutdown.as_mut() {
                match rx.try_recv() {
                    Ok(()) | Err(TryRecvError::Disconnected) => break,
                    Err(TryRecvError::Empty) => {}
                }
            }

            match listener.accept() {
                Ok((stream, peer)) => self.spawn_worker(stream, Some(peer.ip())),
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(e) => {
                    eprintln!("Connection error: {}", e);
                    std::thread::sleep(Duration::from_millis(25));
                }
            }
        }

        Ok(())
    }

    fn spawn_worker(&self, stream: TcpStream, peer: Option<IpAddr>) {
        let handler = Arc::clone(&self.handler);
        let auth = self.auth.clone();
        let config = self.config.clone();
        let limiter = Arc::clone(&self.limiter);
        let auth_backoff = Arc::clone(&self.auth_backoff);
        let tls = self.tls.clone();
        let force_tls = self.force_tls;
        std::thread::spawn(move || {
            let peer_ip = peer
                .or_else(|| stream.peer_addr().ok().map(|addr| addr.ip()))
                .unwrap_or(IpAddr::from([127, 0, 0, 1]));
            if let Err(e) = handle_connection(
                stream,
                peer_ip,
                handler.as_ref(),
                auth.as_ref(),
                &config,
                &limiter,
                &auth_backoff,
                tls.as_ref(),
                force_tls,
            ) {
                eprintln!("Error handling connection: {}", e);
            }
        });
    }
}

/// Abstraction over plain TCP and TLS-encrypted RPC streams.
enum RpcStream {
    Plain(TcpStream),
    Tls(StreamOwned<ServerConnection, TcpStream>),
}

impl RpcStream {
    fn set_read_timeout(&self, duration: Option<Duration>) -> std::io::Result<()> {
        match self {
            RpcStream::Plain(stream) => stream.set_read_timeout(duration),
            RpcStream::Tls(stream) => stream.sock.set_read_timeout(duration),
        }
    }

    fn set_write_timeout(&self, duration: Option<Duration>) -> std::io::Result<()> {
        match self {
            RpcStream::Plain(stream) => stream.set_write_timeout(duration),
            RpcStream::Tls(stream) => stream.sock.set_write_timeout(duration),
        }
    }

    fn shutdown(&mut self) -> std::io::Result<()> {
        match self {
            RpcStream::Plain(stream) => stream.shutdown(Shutdown::Write),
            RpcStream::Tls(stream) => {
                stream.conn.send_close_notify();
                stream.sock.shutdown(Shutdown::Write)
            }
        }
    }
}

impl Read for RpcStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            RpcStream::Plain(stream) => stream.read(buf),
            RpcStream::Tls(stream) => stream.read(buf),
        }
    }
}

impl Write for RpcStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            RpcStream::Plain(stream) => stream.write(buf),
            RpcStream::Tls(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            RpcStream::Plain(stream) => stream.flush(),
            RpcStream::Tls(stream) => stream.flush(),
        }
    }
}

fn upgrade_to_tls(stream: TcpStream, tls_config: &TlsConfig) -> std::io::Result<RpcStream> {
    let server_config = tls_config.server_config();
    let connection = ServerConnection::new(server_config)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
    let mut tls_stream = StreamOwned::new(connection, stream);
    while tls_stream.conn.is_handshaking() {
        tls_stream.conn.complete_io(&mut tls_stream.sock)?;
    }
    Ok(RpcStream::Tls(tls_stream))
}

fn handle_connection<T: methods::RpcMethods>(
    stream: TcpStream,
    peer_ip: IpAddr,
    handler: &T,
    auth: Option<&RpcAuth>,
    config: &RpcConfig,
    limiter: &Arc<Mutex<HashMap<IpAddr, TokenBucket>>>,
    auth_backoff: &Arc<Mutex<HashMap<IpAddr, BackoffState>>>,
    tls: Option<&Arc<TlsConfig>>,
    force_tls: bool,
) -> std::io::Result<()> {
    let start = Instant::now();
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_millis(config.header_read_timeout_ms)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let mut channel = match (tls, force_tls) {
        (Some(tls_config), _) => upgrade_to_tls(stream, tls_config.as_ref())?,
        (None, true) => {
            // TLS required but not configured; drop connection immediately.
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "TLS-required connection attempted without TLS configuration",
            ));
        }
        (None, false) => RpcStream::Plain(stream),
    };

    channel.set_read_timeout(Some(Duration::from_millis(config.header_read_timeout_ms)))?;
    channel.set_write_timeout(Some(Duration::from_secs(5)))?;

    let mut buf_reader = BufReader::new(&mut channel);
    let mut total_header_bytes: usize = 0;
    let mut request_line = String::new();
    let mut headers: Vec<String> = Vec::new();

    loop {
        let mut line = String::new();
        let bytes = buf_reader.read_line(&mut line)?;
        if bytes == 0 {
            let stream = buf_reader.get_mut();
            send_bad_request(*stream)?;
            record_response(
                "INVALID",
                "invalid",
                StatusCode::BAD_REQUEST,
                0,
                start,
                peer_ip,
                false,
                false,
                false,
                false,
            );
            apply_cooldown(config);
            return Ok(());
        }

        total_header_bytes += bytes;
        if total_header_bytes > config.max_header_bytes {
            let stream = buf_reader.get_mut();
            send_header_too_large(*stream)?;
            record_response(
                "INVALID",
                "header",
                StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
                0,
                start,
                peer_ip,
                false,
                false,
                false,
                true,
            );
            apply_cooldown(config);
            return Ok(());
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if request_line.is_empty() {
            if trimmed.is_empty() {
                let stream = buf_reader.get_mut();
                send_bad_request(*stream)?;
                record_response(
                    "INVALID",
                    "invalid",
                    StatusCode::BAD_REQUEST,
                    0,
                    start,
                    peer_ip,
                    false,
                    false,
                    false,
                    false,
                );
                apply_cooldown(config);
                return Ok(());
            }
            request_line = trimmed.to_string();
            continue;
        }

        if trimmed.is_empty() {
            break;
        }

        headers.push(trimmed.to_string());
    }

    (**buf_reader.get_mut()).set_read_timeout(Some(Duration::from_secs(5)))?;

    if request_line.is_empty() {
        let stream = buf_reader.get_mut();
        send_bad_request(*stream)?;
        record_response(
            "INVALID",
            "invalid",
            StatusCode::BAD_REQUEST,
            0,
            start,
            peer_ip,
            false,
            false,
            false,
            false,
        );
        apply_cooldown(config);
        return Ok(());
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");
    let path_owned = path.to_string();

    let client_ip = resolve_client_ip(peer_ip, &headers, config);

    let is_health = method.eq_ignore_ascii_case("GET") && path == "/health";
    let is_metrics = method.eq_ignore_ascii_case("GET") && path == "/metrics";

    let content_length = headers
        .iter()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.trim().eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);

    if is_metrics {
        if !client_ip.is_loopback() {
            let stream = buf_reader.get_mut();
            send_forbidden(*stream)?;
            record_response(
                method,
                &path_owned,
                StatusCode::FORBIDDEN,
                0,
                start,
                client_ip,
                false,
                false,
                false,
                false,
            );
            apply_cooldown(config);
            return Ok(());
        }

        let body = render_metrics();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        {
            let stream = buf_reader.get_mut();
            stream.write_all(response.as_bytes())?;
            stream.flush()?;
            let _ = stream.shutdown();
        }
        record_response(
            method,
            &path_owned,
            StatusCode::OK,
            0,
            start,
            client_ip,
            false,
            false,
            false,
            false,
        );
        return Ok(());
    }

    if is_health {
        let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nok";
        {
            let stream = buf_reader.get_mut();
            stream.write_all(response.as_bytes())?;
            stream.flush()?;
            let _ = stream.shutdown();
        }
        record_response(
            method,
            &path_owned,
            StatusCode::OK,
            0,
            start,
            client_ip,
            false,
            false,
            false,
            false,
        );
        return Ok(());
    }

    if content_length == 0 {
        let stream = buf_reader.get_mut();
        send_bad_request(*stream)?;
        record_response(
            method,
            &path_owned,
            StatusCode::BAD_REQUEST,
            content_length,
            start,
            client_ip,
            false,
            false,
            false,
            false,
        );
        apply_cooldown(config);
        return Ok(());
    }

    if content_length > config.max_body_bytes {
        let stream = buf_reader.get_mut();
        send_payload_too_large(*stream)?;
        record_response(
            method,
            &path_owned,
            StatusCode::PAYLOAD_TOO_LARGE,
            content_length,
            start,
            client_ip,
            false,
            true,
            false,
            false,
        );
        apply_cooldown(config);
        return Ok(());
    }

    if !take_token(client_ip, limiter, config) {
        let stream = buf_reader.get_mut();
        send_too_many_requests(*stream)?;
        record_response(
            method,
            &path_owned,
            StatusCode::TOO_MANY_REQUESTS,
            content_length,
            start,
            client_ip,
            true,
            false,
            false,
            false,
        );
        apply_cooldown(config);
        return Ok(());
    }

    if let Some(auth_cfg) = auth {
        if !is_authorized(&headers, auth_cfg) {
            let stream = buf_reader.get_mut();
            send_unauthorized(*stream)?;
            record_response(
                method,
                &path_owned,
                StatusCode::UNAUTHORIZED,
                content_length,
                start,
                client_ip,
                false,
                false,
                true,
                false,
            );
            apply_auth_backoff(client_ip, auth_backoff);
            apply_cooldown(config);
            return Ok(());
        } else {
            reset_auth_backoff(client_ip, auth_backoff);
        }
    }

    (**buf_reader.get_mut()).set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut body = vec![0u8; content_length];
    buf_reader.read_exact(&mut body)?;

    // Release the buffered borrow so we can reuse the channel for writing.
    let stream = buf_reader.into_inner();

    let json_request = match serde_json::from_slice::<JsonRpcRequest>(&body) {
        Ok(req) => req,
        Err(e) => {
            let error_response = JsonRpcResponse::error(
                serde_json::Value::Null,
                error_codes::PARSE_ERROR,
                format!("Parse error: {e}"),
            );
            respond_json(stream, &error_response)?;
            record_response(
                method,
                &path_owned,
                StatusCode::OK,
                content_length,
                start,
                client_ip,
                false,
                false,
                false,
                false,
            );
            apply_cooldown(config);
            return Ok(());
        }
    };

    let json_response = if json_request.jsonrpc != "2.0" {
        JsonRpcResponse::error(
            json_request.id,
            error_codes::INVALID_REQUEST,
            "Invalid JSON-RPC version".to_string(),
        )
    } else {
        methods::dispatch_call(
            handler,
            &json_request.method,
            json_request.params,
            json_request.id,
        )
    };

    respond_json(stream, &json_response)?;
    record_response(
        method,
        &path_owned,
        StatusCode::OK,
        content_length,
        start,
        client_ip,
        false,
        false,
        false,
        false,
    );
    apply_cooldown(config);
    Ok(())
}

fn is_authorized(request_headers: &[String], auth: &RpcAuth) -> bool {
    let header = request_headers
        .iter()
        .find(|line| line.to_ascii_lowercase().starts_with("authorization:"));

    let Some(header) = header else {
        return false;
    };

    let mut parts = header.splitn(2, ':');
    parts.next(); // skip "Authorization"
    let value = parts.next().map(str::trim).unwrap_or_default();

    if !value.to_ascii_lowercase().starts_with("basic ") {
        return false;
    }

    let encoded = value[5..].trim();
    use base64::{engine::general_purpose::STANDARD, Engine};

    let decoded = match STANDARD.decode(encoded) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };

    let credential = match String::from_utf8(decoded) {
        Ok(s) => s,
        Err(_) => return false,
    };

    auth.matches(&credential)
}

fn respond_json(stream: &mut RpcStream, response: &JsonRpcResponse) -> std::io::Result<()> {
    let response_body = serde_json::to_string(response).unwrap();
    let http_response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response_body.len(),
        response_body
    );
    stream.write_all(http_response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown();
    Ok(())
}

fn send_bad_request(stream: &mut RpcStream) -> std::io::Result<()> {
    let response = concat!(
        "HTTP/1.1 400 Bad Request\r\n",
        "Content-Length: 0\r\n",
        "Connection: close\r\n",
        "\r\n"
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown();
    Ok(())
}

fn send_payload_too_large(stream: &mut RpcStream) -> std::io::Result<()> {
    let response = concat!(
        "HTTP/1.1 413 Payload Too Large\r\n",
        "Content-Length: 0\r\n",
        "Connection: close\r\n",
        "\r\n"
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown();
    Ok(())
}

fn send_header_too_large(stream: &mut RpcStream) -> std::io::Result<()> {
    let response = concat!(
        "HTTP/1.1 431 Request Header Fields Too Large\r\n",
        "Content-Length: 0\r\n",
        "Connection: close\r\n",
        "\r\n"
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown();
    Ok(())
}

fn send_forbidden(stream: &mut RpcStream) -> std::io::Result<()> {
    let response = concat!(
        "HTTP/1.1 403 Forbidden\r\n",
        "Content-Length: 0\r\n",
        "Connection: close\r\n",
        "\r\n"
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown();
    Ok(())
}

fn send_unauthorized(stream: &mut RpcStream) -> std::io::Result<()> {
    let response = concat!(
        "HTTP/1.1 401 Unauthorized\r\n",
        "WWW-Authenticate: Basic realm=\"BitQuan RPC\"\r\n",
        "Content-Length: 0\r\n",
        "Content-Type: text/plain\r\n",
        "Connection: close\r\n",
        "\r\n"
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown();
    Ok(())
}

fn send_too_many_requests(stream: &mut RpcStream) -> std::io::Result<()> {
    let response = concat!(
        "HTTP/1.1 429 Too Many Requests\r\n",
        "Content-Length: 0\r\n",
        "Connection: close\r\n",
        "\r\n"
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown();
    Ok(())
}

fn apply_cooldown(config: &RpcConfig) {
    if config.conn_cooldown_ms > 0 {
        std::thread::sleep(Duration::from_millis(config.conn_cooldown_ms));
    }
}

fn take_token(
    ip: IpAddr,
    limiter: &Arc<Mutex<HashMap<IpAddr, TokenBucket>>>,
    config: &RpcConfig,
) -> bool {
    let mut map = limiter.lock().unwrap();
    let bucket = map.entry(ip).or_insert_with(|| TokenBucket {
        tokens: config.rl_burst as f64,
        last: Instant::now(),
    });

    let now = Instant::now();
    let elapsed = now.duration_since(bucket.last).as_secs_f64();
    bucket.last = now;

    if config.rl_refill_per_sec > 0 {
        bucket.tokens =
            (bucket.tokens + elapsed * config.rl_refill_per_sec as f64).min(config.rl_burst as f64);
    }

    if bucket.tokens >= 1.0 {
        bucket.tokens -= 1.0;
        true
    } else {
        false
    }
}

struct TokenBucket {
    tokens: f64,
    last: Instant,
}

struct BackoffState {
    fails: u32,
    last: Instant,
}

fn resolve_client_ip(peer_ip: IpAddr, headers: &[String], config: &RpcConfig) -> IpAddr {
    if !config.trust_proxy {
        return peer_ip;
    }

    if !config
        .trusted_proxies
        .iter()
        .any(|cidr| cidr.contains(peer_ip))
    {
        return peer_ip;
    }

    for line in headers {
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("x-forwarded-for") {
                if let Some(first) = value.split(',').next() {
                    if let Ok(addr) = IpAddr::from_str(first.trim()) {
                        return addr;
                    }
                }
            }
        }
    }

    peer_ip
}

fn apply_auth_backoff(ip: IpAddr, backoff: &Arc<Mutex<HashMap<IpAddr, BackoffState>>>) {
    let mut map = backoff.lock().unwrap();
    let state = map.entry(ip).or_insert(BackoffState {
        fails: 0,
        last: Instant::now(),
    });
    state.fails = state.fails.saturating_add(1);
    state.last = Instant::now();
    let exponent = (state.fails - 1).min(4);
    let delay_ms = 100u64.saturating_mul(1u64 << exponent);
    drop(map);
    std::thread::sleep(Duration::from_millis(delay_ms));
}

fn reset_auth_backoff(ip: IpAddr, backoff: &Arc<Mutex<HashMap<IpAddr, BackoffState>>>) {
    let mut map = backoff.lock().unwrap();
    map.remove(&ip);
}

struct RpcMetrics {
    requests_total: AtomicU64,
    status_200: AtomicU64,
    status_400: AtomicU64,
    status_401: AtomicU64,
    status_403: AtomicU64,
    status_413: AtomicU64,
    status_429: AtomicU64,
    status_431: AtomicU64,
    status_other: AtomicU64,
    rl_drops_total: AtomicU64,
    auth_fail_total: AtomicU64,
    body_too_large_total: AtomicU64,
    header_limit_total: AtomicU64,
    latency_sum_ms: AtomicU64,
    latency_count: AtomicU64,
    latency_bucket_50: AtomicU64,
    latency_bucket_100: AtomicU64,
    latency_bucket_250: AtomicU64,
    latency_bucket_500: AtomicU64,
    latency_bucket_1000: AtomicU64,
    latency_bucket_inf: AtomicU64,
}

impl Default for RpcMetrics {
    fn default() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            status_200: AtomicU64::new(0),
            status_400: AtomicU64::new(0),
            status_401: AtomicU64::new(0),
            status_403: AtomicU64::new(0),
            status_413: AtomicU64::new(0),
            status_429: AtomicU64::new(0),
            status_431: AtomicU64::new(0),
            status_other: AtomicU64::new(0),
            rl_drops_total: AtomicU64::new(0),
            auth_fail_total: AtomicU64::new(0),
            body_too_large_total: AtomicU64::new(0),
            header_limit_total: AtomicU64::new(0),
            latency_sum_ms: AtomicU64::new(0),
            latency_count: AtomicU64::new(0),
            latency_bucket_50: AtomicU64::new(0),
            latency_bucket_100: AtomicU64::new(0),
            latency_bucket_250: AtomicU64::new(0),
            latency_bucket_500: AtomicU64::new(0),
            latency_bucket_1000: AtomicU64::new(0),
            latency_bucket_inf: AtomicU64::new(0),
        }
    }
}

impl RpcMetrics {
    fn record(
        &self,
        status: StatusCode,
        latency_ms: u64,
        rate_limited: bool,
        body_limit: bool,
        auth_fail: bool,
        header_limit: bool,
    ) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        match status {
            StatusCode::OK => self.status_200.fetch_add(1, Ordering::Relaxed),
            StatusCode::BAD_REQUEST => self.status_400.fetch_add(1, Ordering::Relaxed),
            StatusCode::UNAUTHORIZED => self.status_401.fetch_add(1, Ordering::Relaxed),
            StatusCode::FORBIDDEN => self.status_403.fetch_add(1, Ordering::Relaxed),
            StatusCode::PAYLOAD_TOO_LARGE => self.status_413.fetch_add(1, Ordering::Relaxed),
            StatusCode::TOO_MANY_REQUESTS => self.status_429.fetch_add(1, Ordering::Relaxed),
            StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE => {
                self.status_431.fetch_add(1, Ordering::Relaxed)
            }
            _ => self.status_other.fetch_add(1, Ordering::Relaxed),
        };

        if rate_limited {
            self.rl_drops_total.fetch_add(1, Ordering::Relaxed);
        }
        if body_limit {
            self.body_too_large_total.fetch_add(1, Ordering::Relaxed);
        }
        if auth_fail {
            self.auth_fail_total.fetch_add(1, Ordering::Relaxed);
        }
        if header_limit {
            self.header_limit_total.fetch_add(1, Ordering::Relaxed);
        }

        self.latency_sum_ms.fetch_add(latency_ms, Ordering::Relaxed);
        self.latency_count.fetch_add(1, Ordering::Relaxed);

        let bucket = if latency_ms <= 50 {
            &self.latency_bucket_50
        } else if latency_ms <= 100 {
            &self.latency_bucket_100
        } else if latency_ms <= 250 {
            &self.latency_bucket_250
        } else if latency_ms <= 500 {
            &self.latency_bucket_500
        } else if latency_ms <= 1000 {
            &self.latency_bucket_1000
        } else {
            &self.latency_bucket_inf
        };
        bucket.fetch_add(1, Ordering::Relaxed);
    }

    fn render(&self) -> String {
        let total = self.requests_total.load(Ordering::Relaxed);
        let status_line = |code: &str, value| {
            format!("rpc_requests_status_total{{code=\"{}\"}} {}\n", code, value)
        };
        let body = format!(
            "rpc_requests_total {}\n{}{}{}{}{}{}{}{}rpc_rl_drops_total {}\nrpc_body_413_total {}\nrpc_header_limit_total {}\nrpc_auth_401_total {}\nrpc_latency_ms_sum {}\nrpc_latency_ms_count {}\nrpc_latency_ms_bucket{{le=\"50\"}} {}\nrpc_latency_ms_bucket{{le=\"100\"}} {}\nrpc_latency_ms_bucket{{le=\"250\"}} {}\nrpc_latency_ms_bucket{{le=\"500\"}} {}\nrpc_latency_ms_bucket{{le=\"1000\"}} {}\nrpc_latency_ms_bucket{{le=\"+Inf\"}} {}\n",
            total,
            status_line("200", self.status_200.load(Ordering::Relaxed)),
            status_line("400", self.status_400.load(Ordering::Relaxed)),
            status_line("401", self.status_401.load(Ordering::Relaxed)),
            status_line("403", self.status_403.load(Ordering::Relaxed)),
            status_line("413", self.status_413.load(Ordering::Relaxed)),
            status_line("429", self.status_429.load(Ordering::Relaxed)),
            status_line("431", self.status_431.load(Ordering::Relaxed)),
            status_line("other", self.status_other.load(Ordering::Relaxed)),
            self.rl_drops_total.load(Ordering::Relaxed),
            self.body_too_large_total.load(Ordering::Relaxed),
            self.header_limit_total.load(Ordering::Relaxed),
            self.auth_fail_total.load(Ordering::Relaxed),
            self.latency_sum_ms.load(Ordering::Relaxed),
            self.latency_count.load(Ordering::Relaxed),
            self.latency_bucket_50.load(Ordering::Relaxed),
            self.latency_bucket_100.load(Ordering::Relaxed),
            self.latency_bucket_250.load(Ordering::Relaxed),
            self.latency_bucket_500.load(Ordering::Relaxed),
            self.latency_bucket_1000.load(Ordering::Relaxed),
            self.latency_bucket_inf.load(Ordering::Relaxed),
        );
        body
    }
}

static METRICS: Lazy<RpcMetrics> = Lazy::new(RpcMetrics::default);

fn render_metrics() -> String {
    METRICS.render()
}

fn record_response(
    method: &str,
    path: &str,
    status: StatusCode,
    content_length: usize,
    start: Instant,
    client_ip: IpAddr,
    rate_limited: bool,
    body_limit: bool,
    auth_fail: bool,
    header_limit: bool,
) {
    let latency_ms = start.elapsed().as_millis() as u64;
    METRICS.record(
        status,
        latency_ms,
        rate_limited,
        body_limit,
        auth_fail,
        header_limit,
    );

    if rate_limited || body_limit || auth_fail || header_limit {
        let reason = if rate_limited {
            "rate_limit"
        } else if body_limit {
            "body_limit"
        } else if header_limit {
            "header_limit"
        } else {
            "auth_failure"
        };

        let log = json!({
            "event": "rpc_guard",
            "reason": reason,
            "status": status.as_u16(),
            "method": method,
            "route": path,
            "ip": client_ip.to_string(),
            "bytes": content_length,
            "latency_ms": latency_ms,
        });
        println!("{}", log);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::methods::{BlockTemplate, BlockchainInfo, MiningInfo, RpcMethods, TxInfo};
    use crate::test_util::{basic_auth_header, init_test_tracing, spawn_test_server, wait_ready};
    use crate::{RpcConfig, RpcError};
    use anyhow::Result;
    use futures::future::try_join_all;
    use reqwest::StatusCode;
    use tokio::time::{sleep, timeout, Duration};

    struct TestHandler;

    impl RpcMethods for TestHandler {
        fn getblockcount(&self) -> Result<u64, RpcError> {
            Ok(100)
        }

        fn getblockchaininfo(&self) -> Result<BlockchainInfo, RpcError> {
            Ok(BlockchainInfo {
                chain: "test".to_string(),
                blocks: 100,
                bestblockhash: "test".to_string(),
                difficulty: 1.0,
                chainwork: "0".to_string(),
            })
        }

        fn getmininginfo(&self) -> Result<MiningInfo, RpcError> {
            Ok(MiningInfo {
                blocks: 100,
                difficulty: 1.0,
                networkhashps: 1000.0,
            })
        }

        fn getblocktemplate(&self) -> Result<BlockTemplate, RpcError> {
            Err(RpcError::InternalError("not implemented".to_string()))
        }

        fn submitblock(&self, _: String) -> Result<bool, RpcError> {
            Ok(true)
        }

        fn gettransaction(&self, _: String) -> Result<TxInfo, RpcError> {
            Err(RpcError::InternalError("not found".to_string()))
        }

        fn getbestblockhash(&self) -> Result<String, RpcError> {
            Ok("test".to_string())
        }

        fn getblockhash(&self, _: u64) -> Result<String, RpcError> {
            Ok("test".to_string())
        }

        fn getwork(&self) -> Result<crate::methods::WorkData, RpcError> {
            Ok(crate::methods::WorkData {
                data: "00000000".to_string(),
                target: "00000000ffff".to_string(),
            })
        }

        fn submitwork(&self, _: String) -> Result<bool, RpcError> {
            Ok(true)
        }
    }

    fn base_config() -> RpcConfig {
        let mut cfg = RpcConfig::default();
        cfg.conn_cooldown_ms = 0;
        cfg.trust_proxy = false;
        cfg.trusted_proxies.clear();
        cfg
    }

    #[test]
    fn test_server_creation() {
        let handler = TestHandler;
        let _server = RpcServer::new(handler, "127.0.0.1:0".to_string());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn health_not_rate_limited_under_load() -> Result<()> {
        init_test_tracing();
        let mut config = base_config();
        config.rl_burst = 1;
        config.rl_refill_per_sec = 0;
        let server = RpcServer::with_auth(TestHandler, "127.0.0.1:0".to_string(), None, config);
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let health_url = format!("{}/health", base_url);

        let mut tasks = Vec::with_capacity(50);
        for _ in 0..50 {
            let client = client.clone();
            let url = health_url.clone();
            tasks.push(async move {
                let resp = timeout(Duration::from_secs(5), client.get(url).send()).await??;
                anyhow::Result::<StatusCode>::Ok(resp.status())
            });
        }

        let statuses = try_join_all(tasks).await?;
        assert!(statuses.into_iter().all(|code| code == StatusCode::OK));

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn health_requires_no_auth_and_closes() -> Result<()> {
        init_test_tracing();
        let server =
            RpcServer::with_auth(TestHandler, "127.0.0.1:0".to_string(), None, base_config());
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;

        let resp = timeout(
            Duration::from_secs(5),
            client.get(format!("{}/health", base_url)).send(),
        )
        .await??;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.text().await?, "ok");

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rpc_body_within_limit_allows_200() -> Result<()> {
        init_test_tracing();
        let mut config = base_config();
        config.max_body_bytes = 131_072;
        let auth = RpcAuth::new("user", "pass");
        let server = RpcServer::with_auth(
            TestHandler,
            "127.0.0.1:0".to_string(),
            Some(auth.clone()),
            config,
        );
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let rpc_endpoint = format!("{}/rpc", base_url);

        let body = vec![b'a'; 64 * 1024];
        let resp = timeout(
            Duration::from_secs(5),
            client
                .post(&rpc_endpoint)
                .header("Content-Type", "application/json")
                .header(
                    "Authorization",
                    basic_auth_header(auth.username(), auth.password()),
                )
                .body(body)
                .send(),
        )
        .await??;
        assert_eq!(resp.status(), StatusCode::OK);

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rpc_max_body_returns_413() -> Result<()> {
        init_test_tracing();
        let mut config = base_config();
        config.max_body_bytes = 131_072;
        let server = RpcServer::with_auth(TestHandler, "127.0.0.1:0".to_string(), None, config);
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let rpc_endpoint = format!("{}/rpc", base_url);

        let oversized = vec![b'a'; 256 * 1024];
        let resp = timeout(
            Duration::from_secs(5),
            client
                .post(&rpc_endpoint)
                .header("Content-Type", "application/json")
                .body(oversized)
                .send(),
        )
        .await??;
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rpc_without_auth_is_401() -> Result<()> {
        init_test_tracing();
        let config = base_config();
        let auth = RpcAuth::new("user", "pass");
        let server = RpcServer::with_auth(
            TestHandler,
            "127.0.0.1:0".to_string(),
            Some(auth.clone()),
            config,
        );
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let rpc_endpoint = format!("{}/rpc", base_url);

        let body = r#"{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}"#;
        let resp = timeout(
            Duration::from_secs(5),
            client
                .post(&rpc_endpoint)
                .header("Content-Type", "application/json")
                .body(body)
                .send(),
        )
        .await??;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rpc_concurrency_smoke() -> Result<()> {
        init_test_tracing();
        let mut config = base_config();
        config.rl_burst = 20;
        let auth = RpcAuth::new("alice", "secret");
        let server = RpcServer::with_auth(
            TestHandler,
            "127.0.0.1:0".to_string(),
            Some(auth.clone()),
            config,
        );
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let rpc_endpoint = format!("{}/rpc", base_url);

        let auth_header = basic_auth_header(auth.username(), auth.password());
        let body = r#"{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}"#.to_string();

        let mut tasks = Vec::with_capacity(10);
        for _ in 0..10 {
            let client = client.clone();
            let url = rpc_endpoint.clone();
            let auth_header = auth_header.clone();
            let body = body.clone();
            tasks.push(async move {
                let resp = timeout(
                    Duration::from_secs(5),
                    client
                        .post(&url)
                        .header("Content-Type", "application/json")
                        .header("Authorization", &auth_header)
                        .body(body)
                        .send(),
                )
                .await??;
                anyhow::Result::<String>::Ok(resp.text().await?)
            });
        }

        let responses = try_join_all(tasks).await?;
        for text in responses {
            assert!(text.contains("100"));
        }

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rpc_rate_limit_429_when_exceeded() -> Result<()> {
        init_test_tracing();
        let mut config = base_config();
        config.rl_burst = 5;
        config.rl_refill_per_sec = 0;
        let auth = RpcAuth::new("user", "pass");
        let server = RpcServer::with_auth(
            TestHandler,
            "127.0.0.1:0".to_string(),
            Some(auth.clone()),
            config,
        );
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let rpc_endpoint = format!("{}/rpc", base_url);

        let body = r#"{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}"#;
        let mut statuses = Vec::new();
        for _ in 0..10 {
            let resp = timeout(
                Duration::from_secs(5),
                client
                    .post(&rpc_endpoint)
                    .header("Content-Type", "application/json")
                    .header(
                        "Authorization",
                        basic_auth_header(auth.username(), auth.password()),
                    )
                    .body(body)
                    .send(),
            )
            .await??;
            statuses.push(resp.status());
        }

        let ok = statuses.iter().filter(|&&s| s == StatusCode::OK).count();
        let limited = statuses
            .iter()
            .filter(|&&s| s == StatusCode::TOO_MANY_REQUESTS)
            .count();
        assert_eq!(ok, 5);
        assert_eq!(limited, 5);

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn rpc_rate_limit_recovers_after_time() -> Result<()> {
        init_test_tracing();
        let mut config = base_config();
        config.rl_burst = 2;
        config.rl_refill_per_sec = 4;
        let auth = RpcAuth::new("user", "pass");
        let server = RpcServer::with_auth(
            TestHandler,
            "127.0.0.1:0".to_string(),
            Some(auth.clone()),
            config,
        );
        let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

        wait_ready(&base_url).await?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let rpc_endpoint = format!("{}/rpc", base_url);

        let auth_header = basic_auth_header(auth.username(), auth.password());
        let body = r#"{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}"#;

        for _ in 0..2 {
            let resp = timeout(
                Duration::from_secs(5),
                client
                    .post(&rpc_endpoint)
                    .header("Content-Type", "application/json")
                    .header("Authorization", &auth_header)
                    .body(body)
                    .send(),
            )
            .await??;
            assert_eq!(resp.status(), StatusCode::OK);
        }

        let third = timeout(
            Duration::from_secs(5),
            client
                .post(&rpc_endpoint)
                .header("Content-Type", "application/json")
                .header("Authorization", &auth_header)
                .body(body)
                .send(),
        )
        .await??;
        assert_eq!(third.status(), StatusCode::TOO_MANY_REQUESTS);

        sleep(Duration::from_millis(300)).await;

        let fourth = timeout(
            Duration::from_secs(5),
            client
                .post(&rpc_endpoint)
                .header("Content-Type", "application/json")
                .header("Authorization", &auth_header)
                .body(body)
                .send(),
        )
        .await??;
        assert_eq!(fourth.status(), StatusCode::OK);

        let _ = shutdown_tx.send(());
        let _ = timeout(Duration::from_secs(5), handle).await??;
        Ok(())
    }
}
