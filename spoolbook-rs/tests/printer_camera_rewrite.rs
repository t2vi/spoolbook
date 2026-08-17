use spoolbook_rs::printer_camera::{rewrite_request_line, rewrite_response_host};

// No C# reference tests exist for these (PrinterCameraService.cs has zero test coverage) — these
// scenarios come straight from the port's own doc comments, which describe real, confirmed
// firmware behavior (redirect Location handing back the printer's bare real IP, etc).

#[test]
fn rewrite_request_line_replaces_the_url_only_on_the_request_line() {
    let data = b"DESCRIBE rtsp://127.0.0.1:5000 RTSP/1.0\r\nCSeq: 2\r\n\r\n";
    let result = rewrite_request_line(data, "rtsp://127.0.0.1:5000", "rtsps://10.0.0.5:322");

    assert_eq!(result, b"DESCRIBE rtsps://10.0.0.5:322 RTSP/1.0\r\nCSeq: 2\r\n\r\n");
}

#[test]
fn rewrite_request_line_leaves_headers_after_the_request_line_untouched() {
    let data = b"SETUP rtsp://127.0.0.1:5000/track1 RTSP/1.0\r\nAuthorization: Digest foo\r\n\r\n";
    let result = rewrite_request_line(data, "rtsp://127.0.0.1:5000", "rtsps://10.0.0.5:322");

    let text = String::from_utf8(result).unwrap();
    assert!(text.contains("Authorization: Digest foo"), "{text}");
    assert!(text.starts_with("SETUP rtsps://10.0.0.5:322/track1 RTSP/1.0"), "{text}");
}

#[test]
fn rewrite_request_line_passes_through_data_with_no_request_line() {
    // Binary RTP data, or anything that isn't an RTSP control line.
    let data = b"\x24\x00\x01\x02\x03";
    assert_eq!(rewrite_request_line(data, "rtsp://x", "rtsps://y"), data);
}

#[test]
fn rewrite_response_host_rewrites_host_port_in_a_redirect_location_header() {
    let data = b"RTSP/1.0 301 Moved Permanently\r\nLocation: rtsps://10.0.0.5:322/streaming/live/1\r\n\r\n";
    let result = rewrite_response_host(data, "10.0.0.5", 322, 54321);

    let text = String::from_utf8(result).unwrap();
    assert_eq!(text, "RTSP/1.0 301 Moved Permanently\r\nLocation: rtsp://127.0.0.1:54321/streaming/live/1\r\n\r\n");
}

#[test]
fn rewrite_response_host_rewrites_bare_host_without_a_port() {
    let data = b"RTSP/1.0 200 OK\r\nContent-Base: rtsps://10.0.0.5/\r\n\r\n";
    let result = rewrite_response_host(data, "10.0.0.5", 322, 54321);

    let text = String::from_utf8(result).unwrap();
    assert_eq!(text, "RTSP/1.0 200 OK\r\nContent-Base: rtsp://127.0.0.1:54321/\r\n\r\n");
}

#[test]
fn rewrite_response_host_never_touches_the_body_after_the_header_blank_line() {
    // The SDP body legitimately contains the printer's real address ("o=" line) and must reach
    // ffmpeg byte-for-byte — Content-Length was computed over the original body.
    let body = "v=0\r\no=- 0 0 IN IP4 10.0.0.5\r\ns=stream\r\n";
    let data = format!("RTSP/1.0 200 OK\r\nContent-Type: application/sdp\r\n\r\n{body}");
    let result = rewrite_response_host(data.as_bytes(), "10.0.0.5", 322, 54321);

    let text = String::from_utf8(result).unwrap();
    assert!(text.ends_with(body), "{text}");
}

#[test]
fn rewrite_response_host_passes_through_non_rtsp_data_unchanged() {
    // Binary interleaved RTP data (RFC 2326 §10.12) never starts with "RTSP/1.0".
    let data = b"\x24\x00\x01\x02\x03\x04\x05\x06";
    assert_eq!(rewrite_response_host(data, "10.0.0.5", 322, 54321), data);
}
