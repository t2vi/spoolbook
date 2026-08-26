// Port of PrinterCameraService.cs — relays a printer's RTSPS camera feed to the browser as
// MJPEG (docs/adr/0024). One broadcaster per printer (Bambu firmware allows exactly one camera
// connection at a time), shared by every viewer. RTSP-only (X1/X2/H2/P2 family) — the port-6000
// chamber-image protocol (A1/P1) is deliberately not implemented (no printer of that family in
// this system).
//
// UNVERIFIED AGAINST REAL HARDWARE (and against a real `ffmpeg` binary) — same posture as
// printer_mqtt.rs and send_print.rs's FTPS upload: the C# original has zero test coverage for
// this file either (only JpegFrameExtractor and, here, the RTSP rewrite functions are unit
// tested — the pipeline's state machine around subscribe/unsubscribe/retry is also tested,
// since none of that needs a real printer or ffmpeg).
use crate::jpeg_frame_extractor::JpegFrameExtractor;
use axum::body::{Body, Bytes};
use base64::Engine;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Extension, Router};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Command;
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;

const CAMERA_PORT: u16 = 322;
const MAX_RECONNECTS: u32 = 10;
const RECONNECT_DELAY: Duration = Duration::from_millis(500);
const STOP_GRACE_DELAY: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CameraStreamStatus {
    #[default]
    NotStarted,
    Connecting,
    Streaming,
    Unavailable,
}

impl CameraStreamStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotStarted => "NotStarted",
            Self::Connecting => "Connecting",
            Self::Streaming => "Streaming",
            Self::Unavailable => "Unavailable",
        }
    }
}

pub type CameraRegistry = Arc<Mutex<HashMap<i64, Arc<CameraBroadcaster>>>>;

pub fn new_registry() -> CameraRegistry {
    Arc::new(Mutex::new(HashMap::new()))
}

pub async fn status_of(registry: &CameraRegistry, printer_id: i64) -> CameraStreamStatus {
    match registry.lock().await.get(&printer_id) {
        Some(b) => b.status().await,
        None => CameraStreamStatus::NotStarted,
    }
}

pub async fn last_error_of(registry: &CameraRegistry, printer_id: i64) -> Option<String> {
    match registry.lock().await.get(&printer_id) {
        Some(b) => b.last_error().await,
        None => None,
    }
}

pub async fn retry(registry: &CameraRegistry, printer_id: i64) {
    if let Some(b) = registry.lock().await.get(&printer_id) {
        b.retry().await;
    }
}

async fn get_or_create(registry: &CameraRegistry, printer_id: i64, ip_address: String, access_code: String) -> Arc<CameraBroadcaster> {
    registry.lock().await.entry(printer_id).or_insert_with(|| CameraBroadcaster::new(ip_address, access_code)).clone()
}

struct BroadcasterState {
    status: CameraStreamStatus,
    last_error: Option<String>,
    subscribers: HashMap<u64, mpsc::Sender<Vec<u8>>>,
    next_subscriber_id: u64,
    pipeline_cancel: Option<CancellationToken>,
    stop_grace_cancel: Option<CancellationToken>,
}

impl Default for BroadcasterState {
    fn default() -> Self {
        Self {
            status: CameraStreamStatus::NotStarted,
            last_error: None,
            subscribers: HashMap::new(),
            next_subscriber_id: 0,
            pipeline_cancel: None,
            stop_grace_cancel: None,
        }
    }
}

pub struct CameraBroadcaster {
    ip_address: String,
    access_code: String,
    state: Mutex<BroadcasterState>,
}

// RAII handle: dropping it (browser tab closed, response body cancelled) unsubscribes, so the
// broadcaster notices the viewer is gone without needing an explicit disconnect call from the
// HTTP layer. Also implements Stream directly (over the underlying channel) so it can be handed
// straight to axum::body::Body::from_stream.
pub struct CameraSubscription {
    broadcaster: Arc<CameraBroadcaster>,
    id: u64,
    receiver: mpsc::Receiver<Vec<u8>>,
}

impl Drop for CameraSubscription {
    fn drop(&mut self) {
        let broadcaster = self.broadcaster.clone();
        let id = self.id;
        tokio::spawn(async move { broadcaster.unsubscribe(id).await });
    }
}

impl tokio_stream::Stream for CameraSubscription {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this.receiver.poll_recv(cx) {
            Poll::Ready(Some(frame)) => {
                let mut chunk = format!("--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n", frame.len()).into_bytes();
                chunk.extend_from_slice(&frame);
                chunk.extend_from_slice(b"\r\n");
                Poll::Ready(Some(Ok(Bytes::from(chunk))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl CameraBroadcaster {
    fn new(ip_address: String, access_code: String) -> Arc<Self> {
        Arc::new(Self { ip_address, access_code, state: Mutex::new(BroadcasterState::default()) })
    }

    pub async fn status(&self) -> CameraStreamStatus {
        self.state.lock().await.status
    }

    pub async fn last_error(&self) -> Option<String> {
        self.state.lock().await.last_error.clone()
    }

    pub async fn retry(&self) {
        let mut state = self.state.lock().await;
        if state.status == CameraStreamStatus::Unavailable {
            state.status = CameraStreamStatus::NotStarted;
            state.last_error = None;
        }
    }

    pub async fn subscribe(self: &Arc<Self>) -> CameraSubscription {
        // Bounded to 4, matching the C# original's BoundedChannelFullMode.DropOldest — this
        // uses try_send (drops the *newest* frame on backpressure) rather than evicting the
        // oldest already-buffered one, since mpsc has no eviction API. A live low-fps MJPEG
        // feed trailing by up to 4 frames either way is not a visible difference.
        let (tx, rx) = mpsc::channel(4);
        let id;
        {
            let mut state = self.state.lock().await;
            id = state.next_subscriber_id;
            state.next_subscriber_id += 1;
            state.subscribers.insert(id, tx);
            if let Some(grace) = state.stop_grace_cancel.take() {
                grace.cancel();
            }
            if state.status == CameraStreamStatus::NotStarted {
                self.start_pipeline(&mut state);
            }
        }
        CameraSubscription { broadcaster: self.clone(), id, receiver: rx }
    }

    async fn unsubscribe(self: &Arc<Self>, id: u64) {
        let mut state = self.state.lock().await;
        state.subscribers.remove(&id);
        if state.subscribers.is_empty() {
            self.schedule_stop(&mut state);
        }
    }

    fn start_pipeline(self: &Arc<Self>, state: &mut BroadcasterState) {
        state.status = CameraStreamStatus::Connecting;
        state.last_error = None;
        let cancel = CancellationToken::new();
        state.pipeline_cancel = Some(cancel.clone());
        let this = self.clone();
        tokio::spawn(async move { this.run_pipeline(cancel).await });
    }

    fn schedule_stop(self: &Arc<Self>, state: &mut BroadcasterState) {
        let grace = CancellationToken::new();
        state.stop_grace_cancel = Some(grace.clone());
        let pipeline_cancel = state.pipeline_cancel.clone();
        let this = self.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = tokio::time::sleep(STOP_GRACE_DELAY) => {}
                _ = grace.cancelled() => return,
            }
            let state = this.state.lock().await;
            if state.subscribers.is_empty() {
                if let Some(c) = pipeline_cancel {
                    c.cancel();
                }
            }
        });
    }

    async fn broadcast(&self, frame: &[u8]) {
        let state = self.state.lock().await;
        for tx in state.subscribers.values() {
            let _ = tx.try_send(frame.to_vec());
        }
    }

    async fn set_streaming_if_not_already(&self) {
        let mut state = self.state.lock().await;
        if state.status != CameraStreamStatus::Streaming {
            state.status = CameraStreamStatus::Streaming;
        }
    }

    async fn give_up(&self, message: String) {
        let mut state = self.state.lock().await;
        state.status = CameraStreamStatus::Unavailable;
        state.last_error = Some(message);
    }

    async fn reset_to_not_started_unless_unavailable(&self) {
        let mut state = self.state.lock().await;
        if state.status != CameraStreamStatus::Unavailable {
            state.status = CameraStreamStatus::NotStarted;
        }
    }

    async fn run_pipeline(self: Arc<Self>, cancel: CancellationToken) {
        let mut reconnects: u32 = 0;

        loop {
            if cancel.is_cancelled() {
                break;
            }

            let mut got_any_frame = false;
            let attempt = tokio::select! {
                _ = cancel.cancelled() => break,
                result = self.run_one_attempt(&cancel, &mut got_any_frame) => result,
            };

            if cancel.is_cancelled() {
                break;
            }

            reconnects += 1;
            // Never producing a frame gives up fast (likely unreachable/wrong credentials); a
            // stream that was working and then dropped gets more budget — P2S is documented to
            // drop RTSP sessions after a few seconds as routine behavior, not failure.
            let give_up_after = if got_any_frame { MAX_RECONNECTS } else { 3 };
            if reconnects >= give_up_after {
                let message = match attempt {
                    Some(stderr) if !stderr.trim().is_empty() => format!("Camera stream failed: {}", stderr.trim()),
                    _ => "Camera stream failed — printer may be off or camera disabled.".to_string(),
                };
                self.give_up(message).await;
                return;
            }

            tokio::select! {
                _ = tokio::time::sleep(RECONNECT_DELAY) => {}
                _ = cancel.cancelled() => break,
            }
        }

        self.reset_to_not_started_unless_unavailable().await;
    }

    // Runs one connect-proxy / spawn-ffmpeg / read-frames attempt. Returns ffmpeg's tail stderr
    // (for the eventual give-up error message) once the attempt ends, however it ends.
    async fn run_one_attempt(self: &Arc<Self>, cancel: &CancellationToken, got_any_frame: &mut bool) -> Option<String> {
        let proxy_port = match start_tls_proxy(self.ip_address.clone(), CAMERA_PORT, cancel.clone()).await {
            Ok(port) => port,
            Err(_) => return None,
        };

        let url = format!("rtsp://bblp:{}@127.0.0.1:{proxy_port}/streaming/live/1", self.access_code);
        let mut child = match spawn_ffmpeg(&url) {
            Ok(child) => child,
            Err(_) => return None,
        };

        let mut stdout = child.stdout.take()?;
        let mut stderr = child.stderr.take()?;
        let stderr_task = tokio::spawn(async move { drain_stderr_tail(&mut stderr).await });

        let mut extractor = JpegFrameExtractor::new();
        let mut buffer = [0u8; 8192];
        loop {
            let read = tokio::select! {
                _ = cancel.cancelled() => break,
                result = stdout.read(&mut buffer) => result,
            };
            let n = match read {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };

            let frames = extractor.feed(&buffer[..n]);
            if !frames.is_empty() {
                if !*got_any_frame {
                    self.set_streaming_if_not_already().await;
                }
                *got_any_frame = true;
                for frame in &frames {
                    self.broadcast(frame).await;
                }
            }
        }

        let _ = child.kill().await;
        stderr_task.await.ok().flatten()
    }
}

fn spawn_ffmpeg(url: &str) -> std::io::Result<tokio::process::Child> {
    // P2S-tuned: the default fast-startup probesize/analyzeduration can't lock onto this
    // firmware's slow keyframe pacing, and its RTP timestamps don't advance — without
    // -use_wallclock_as_timestamps, ffmpeg's default CFR conversion freezes after frame 1.
    Command::new("ffmpeg")
        .args([
            "-rtsp_transport",
            "tcp",
            "-rtsp_flags",
            "prefer_tcp",
            "-timeout",
            "30000000",
            "-buffer_size",
            "1024000",
            "-max_delay",
            "500000",
            "-probesize",
            "1000000",
            "-analyzeduration",
            "500000",
            "-fflags",
            "nobuffer",
            "-flags",
            "low_delay",
            "-use_wallclock_as_timestamps",
            "1",
            "-i",
            url,
            "-f",
            "mjpeg",
            "-q:v",
            "5",
            "-r",
            "10",
            "-an",
            "-",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
}

async fn drain_stderr_tail(stderr: &mut tokio::process::ChildStderr) -> Option<String> {
    let mut text = String::new();
    stderr.read_to_string(&mut text).await.ok()?;
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    let tail = &lines[lines.len().saturating_sub(20)..];
    Some(tail.join(" | "))
}

// Bambu's RTSPS data channel needs a real TLS client (ffmpeg's own GnuTLS-linked Debian builds
// reject this printer's TLS renegotiation and drop the stream after a few seconds) — this proxy
// terminates TLS itself and hands ffmpeg a plain rtsp:// localhost connection. TLS 1.2 pinned:
// the same P2S firmware family that needed this in the print-start FTPS fix has a TLS 1.3
// session-handling quirk here too (see tls12_no_verify_config's doc comment).
async fn start_tls_proxy(target_host: String, target_port: u16, cancel: CancellationToken) -> std::io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let proxy_port = listener.local_addr()?.port();
    tokio::spawn(accept_loop(listener, target_host, target_port, proxy_port, cancel));
    Ok(proxy_port)
}

async fn accept_loop(listener: TcpListener, target_host: String, target_port: u16, proxy_port: u16, cancel: CancellationToken) {
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            accepted = listener.accept() => {
                let Ok((client, _)) = accepted else { break };
                tokio::spawn(handle_proxy_client(client, target_host.clone(), target_port, proxy_port));
            }
        }
    }
}

async fn handle_proxy_client(mut client: TcpStream, target_host: String, target_port: u16, proxy_port: u16) {
    let Ok(upstream) = TcpStream::connect((target_host.as_str(), target_port)).await else { return };
    let connector = tokio_rustls::TlsConnector::from(Arc::new(crate::printer_mqtt::tls12_no_verify_config()));
    let Ok(server_name) = rustls::pki_types::ServerName::try_from(target_host.clone()) else { return };
    let Ok(tls_stream) = connector.connect(server_name, upstream).await else { return };

    let (mut tls_read, mut tls_write) = tokio::io::split(tls_stream);
    let (mut client_read, mut client_write) = client.split();

    // Must include the scheme, not just host:port — the printer serves RTSPS-only and
    // 301-redirects any request-line whose URL scheme says plain "rtsp://" (what ffmpeg's
    // proxy-facing URL uses, since ffmpeg itself never sees TLS).
    let proxy_url = format!("rtsp://127.0.0.1:{proxy_port}");
    let real_url = format!("rtsps://{target_host}:{target_port}");
    let to_server = forward(&mut client_read, &mut tls_write, |data| rewrite_request_line(data, &proxy_url, &real_url));
    let to_client = forward(&mut tls_read, &mut client_write, |data| rewrite_response_host(data, &target_host, target_port, proxy_port));

    tokio::select! {
        _ = to_server => {}
        _ = to_client => {}
    }
}

async fn forward<R, W, F>(src: &mut R, dst: &mut W, transform: F)
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
    F: Fn(&[u8]) -> Vec<u8>,
{
    let mut buffer = [0u8; 65536];
    loop {
        let n = match src.read(&mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        if dst.write_all(&transform(&buffer[..n])).await.is_err() {
            break;
        }
    }
}

// RTSP request lines have the form "METHOD <url> RTSP/1.0\r\n" — rewriting only that line (not
// a blind whole-buffer replace) leaves any Authorization header intact. Safe to treat this
// direction as ASCII text: ffmpeg-as-client only ever sends RTSP control lines here, never
// binary RTP data (that flows server->client, handled separately by rewrite_response_host).
pub fn rewrite_request_line(data: &[u8], proxy_url: &str, real_url: &str) -> Vec<u8> {
    let Ok(text) = std::str::from_utf8(data) else { return data.to_vec() };
    if !text.contains(" RTSP/1.0") {
        return data.to_vec();
    }

    let mut lines: Vec<String> = text.split("\r\n").map(str::to_string).collect();
    for line in lines.iter_mut() {
        if line.ends_with(" RTSP/1.0") {
            *line = line.replace(proxy_url, real_url);
            break;
        }
    }
    lines.join("\r\n").into_bytes()
}

// RTSP responses ("RTSP/1.0 200 OK" status line) can embed the printer's own real address in
// headers like Content-Base or a redirect Location — a 301 response handing ffmpeg the printer's
// bare real IP makes it open a brand-new *unproxied* connection that then fails ffmpeg's own
// stricter TLS validation against the printer's self-signed cert (the exact failure this proxy
// exists to avoid). Rewrite both "host:port" and bare "host" (a redirect can omit the port), and
// downgrade "rtsps://" to "rtsp://" (this proxy's ffmpeg-facing listener is plain TCP — a
// same-scheme redirect back to the proxy address makes ffmpeg attempt a second TLS handshake
// against a listener that isn't speaking TLS, which just hangs). Only the header block is
// rewritten — a DESCRIBE response's SDP body can legitimately contain the printer's real address
// and must reach ffmpeg byte-for-byte (Content-Length is computed over the *original* body).
// Binary interleaved RTP data (RFC 2326 §10.12, a leading '$') never starts with "RTSP/1.0", so
// none of this ever touches video payload.
pub fn rewrite_response_host(data: &[u8], target_host: &str, target_port: u16, proxy_port: u16) -> Vec<u8> {
    if data.len() < 8 || &data[..8] != b"RTSP/1.0" {
        return data.to_vec();
    }

    let separator = b"\r\n\r\n";
    let header_len = data.windows(separator.len()).position(|w| w == separator).map_or(data.len(), |i| i + separator.len());

    let Ok(header) = std::str::from_utf8(&data[..header_len]) else { return data.to_vec() };

    let proxy_host_port = format!("127.0.0.1:{proxy_port}");
    let target_host_port = format!("{target_host}:{target_port}");
    let rewritten = header.replace(&target_host_port, &proxy_host_port).replace(target_host, &proxy_host_port).replace("rtsps://", "rtsp://");

    let mut out = rewritten.into_bytes();
    out.extend_from_slice(&data[header_len..]); // body preserved byte-for-byte, never re-encoded
    out
}

// End-of-print bed photo (issues/121): a headless, internal-only subscription — no browser
// watching — that waits for the pipeline's first decoded frame, then drops (Drop's own
// unsubscribe cleans up, same as a browser tab closing). subscribe() lazily starts the pipeline
// if nothing else is already streaming, so this works whether or not the live camera view
// happened to be open at print-end. Bypasses CameraSubscription's Stream impl (which wraps each
// frame in HTTP multipart framing for the browser) — reads the raw JPEG straight off the
// broadcast channel instead.
const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(8);

async fn capture_still(registry: &CameraRegistry, printer_id: i64, ip_address: String, access_code: String) -> Option<Vec<u8>> {
    let broadcaster = get_or_create(registry, printer_id, ip_address, access_code).await;
    let mut subscription = broadcaster.subscribe().await;
    tokio::time::timeout(SNAPSHOT_TIMEOUT, subscription.receiver.recv()).await.ok().flatten()
}

// Called from printer_telemetry::end_job right after the ambient weather fetch — same
// fire-and-forget posture (never panics, never propagates an error to the caller): a printer
// that's already powered off/unreachable right after finishing (routine for a P2S, per
// run_pipeline's own comment) just leaves bed_photo_base64 null, no retry.
pub async fn capture_and_store(pool: &SqlitePool, registry: &CameraRegistry, print_id: i64, printer_id: i64) {
    let Some(printer) = crate::printers::get_by_id(pool, printer_id).await else { return };
    let (Some(ip_address), Some(access_code)) = (printer.ip_address, printer.access_code) else { return };

    match capture_still(registry, printer_id, ip_address, access_code).await {
        Some(frame) => {
            let encoded = base64::engine::general_purpose::STANDARD.encode(&frame);
            sqlx::query("UPDATE prints SET bed_photo_base64 = ?1 WHERE id = ?2")
                .bind(encoded)
                .bind(print_id)
                .execute(pool)
                .await
                .expect("update failed");
        }
        None => eprintln!("bed photo capture failed or timed out for print {print_id} (printer {printer_id})"),
    }
}

pub fn router() -> Router<SqlitePool> {
    Router::new().route("/printers/{id}/camera", get(stream)).route("/api/printers/{id}/camera/retry", axum::routing::post(retry_endpoint))
}

async fn stream(
    _editor: crate::auth::Editor,
    State(pool): State<SqlitePool>,
    Extension(registry): Extension<CameraRegistry>,
    Path(id): Path<i64>,
) -> Response {
    let Some(printer) = crate::printers::get_by_id(&pool, id).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let (Some(ip_address), Some(access_code)) = (printer.ip_address, printer.access_code) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let broadcaster = get_or_create(&registry, id, ip_address, access_code).await;
    let subscription = broadcaster.subscribe().await;

    (
        [(header::CONTENT_TYPE, "multipart/x-mixed-replace; boundary=frame"), (header::CACHE_CONTROL, "no-cache")],
        Body::from_stream(subscription),
    )
        .into_response()
}

// Just clears the failed state back to NotStarted — there's no subscriber to hand frames to
// until the browser's <img> tag actually reconnects, so the real "start" trigger is
// stream()/subscribe(), same as the very first view. The UI is expected to re-render the <img>
// (cache-busted) right after calling this.
async fn retry_endpoint(Extension(registry): Extension<CameraRegistry>, Path(id): Path<i64>) -> axum::Json<serde_json::Value> {
    retry(&registry, id).await;
    axum::Json(serde_json::json!({ "ok": true }))
}
