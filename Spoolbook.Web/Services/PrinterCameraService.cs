using System.Collections.Concurrent;
using System.Diagnostics;
using System.Net;
using System.Net.Security;
using System.Net.Sockets;
using System.Runtime.CompilerServices;
using System.Text;
using System.Threading.Channels;
using Spoolbook.Desktop.Services.Printer;

namespace Spoolbook.Web.Services;

public enum CameraStreamStatus { NotStarted, Connecting, Streaming, Unavailable }

// Relays a printer's RTSPS camera feed to the browser as MJPEG (docs/adr/0024). Singleton,
// one broadcaster per printer — Bambu firmware allows exactly one camera connection at a
// time, so every viewer (tab/device) shares the same upstream ffmpeg process instead of each
// opening its own. RTSP-only (X1/X2/H2/P2 family, confirmed against a real P2S) — the
// port-6000 chamber-image protocol (A1/P1) is deliberately not implemented (YAGNI, no
// printer of that family in this system).
public class PrinterCameraService
{
    private readonly ILogger<PrinterCameraService> _logger;
    private readonly ConcurrentDictionary<int, CameraBroadcaster> _broadcasters = new();

    public PrinterCameraService(ILogger<PrinterCameraService> logger) => _logger = logger;

    public CameraStreamStatus GetStatus(int printerId) =>
        _broadcasters.TryGetValue(printerId, out var b) ? b.Status : CameraStreamStatus.NotStarted;

    public string? GetLastError(int printerId) =>
        _broadcasters.TryGetValue(printerId, out var b) ? b.LastError : null;

    public void Retry(int printerId)
    {
        if (_broadcasters.TryGetValue(printerId, out var b))
            b.Retry();
    }

    public async IAsyncEnumerable<byte[]> SubscribeAsync(
        int printerId, string ipAddress, string accessCode, [EnumeratorCancellation] CancellationToken ct)
    {
        var broadcaster = GetOrCreate(printerId, ipAddress, accessCode);
        var channel = broadcaster.Subscribe();
        try
        {
            await foreach (var frame in channel.Reader.ReadAllAsync(ct))
                yield return frame;
        }
        finally
        {
            broadcaster.Unsubscribe(channel);
        }
    }

    private CameraBroadcaster GetOrCreate(int printerId, string ipAddress, string accessCode) =>
        _broadcasters.GetOrAdd(printerId, _ => new CameraBroadcaster(ipAddress, accessCode, _logger));
}

internal class CameraBroadcaster
{
    private const int Port = 322;
    private const int MaxReconnects = 10;
    private static readonly TimeSpan ReconnectDelay = TimeSpan.FromMilliseconds(500);
    private static readonly TimeSpan StopGraceDelay = TimeSpan.FromSeconds(10);

    private readonly string _ipAddress;
    private readonly string _accessCode;
    private readonly ILogger _logger;
    private readonly object _lock = new();
    private readonly List<Channel<byte[]>> _subscribers = [];

    private CancellationTokenSource? _pipelineCts;
    private CancellationTokenSource? _stopGraceCts;

    public CameraStreamStatus Status { get; private set; } = CameraStreamStatus.NotStarted;
    public string? LastError { get; private set; }

    public CameraBroadcaster(string ipAddress, string accessCode, ILogger logger)
    {
        _ipAddress = ipAddress;
        _accessCode = accessCode;
        _logger = logger;
    }

    public Channel<byte[]> Subscribe()
    {
        var channel = Channel.CreateBounded<byte[]>(new BoundedChannelOptions(4) { FullMode = BoundedChannelFullMode.DropOldest });
        lock (_lock)
        {
            _subscribers.Add(channel);
            _stopGraceCts?.Cancel(); // a new viewer arrived during the shutdown grace window
            if (Status is CameraStreamStatus.NotStarted)
                StartPipelineLocked();
        }
        return channel;
    }

    public void Unsubscribe(Channel<byte[]> channel)
    {
        lock (_lock)
        {
            _subscribers.Remove(channel);
            channel.Writer.TryComplete();
            if (_subscribers.Count == 0)
                ScheduleStopLocked();
        }
    }

    // Just clears the failed state back to NotStarted — there's no subscriber (channel) to
    // hand frames to until the browser's <img> tag actually reconnects, so the real "start"
    // trigger is Subscribe(), same as the very first view. The UI is expected to re-render
    // the <img> (cache-busted) right after calling this.
    public void Retry()
    {
        lock (_lock)
        {
            if (Status == CameraStreamStatus.Unavailable)
            {
                Status = CameraStreamStatus.NotStarted;
                LastError = null;
            }
        }
    }

    private void StartPipelineLocked()
    {
        Status = CameraStreamStatus.Connecting;
        LastError = null;
        _pipelineCts = new CancellationTokenSource();
        _ = RunPipelineAsync(_pipelineCts.Token);
    }

    private void ScheduleStopLocked()
    {
        _stopGraceCts = new CancellationTokenSource();
        var graceToken = _stopGraceCts.Token;
        var pipelineCts = _pipelineCts;
        _ = Task.Run(async () =>
        {
            try { await Task.Delay(StopGraceDelay, graceToken); }
            catch (OperationCanceledException) { return; }

            lock (_lock)
            {
                if (_subscribers.Count > 0) return;
                pipelineCts?.Cancel();
            }
        });
    }

    private void Broadcast(byte[] frame)
    {
        lock (_lock)
        {
            foreach (var sub in _subscribers)
                sub.Writer.TryWrite(frame);
        }
    }

    private async Task RunPipelineAsync(CancellationToken ct)
    {
        var reconnects = 0;
        while (!ct.IsCancellationRequested)
        {
            TcpListener? proxyListener = null;
            Process? ffmpeg = null;
            Task<string>? stderrTask = null;
            var gotAnyFrame = false;

            try
            {
                (var proxyPort, proxyListener) = StartTlsProxy(_ipAddress, Port, ct);
                _logger.LogDebug("Camera TLS proxy on 127.0.0.1:{Port} -> {Ip}:{Target}", proxyPort, _ipAddress, Port);
                ffmpeg = StartFfmpeg(proxyPort);
                _logger.LogDebug("Camera ffmpeg started, pid={Pid}", ffmpeg.Id);
                stderrTask = DrainStderrAsync(ffmpeg);

                var extractor = new JpegFrameExtractor();
                var buffer = new byte[8192];
                var stdout = ffmpeg.StandardOutput.BaseStream;
                while (true)
                {
                    var n = await stdout.ReadAsync(buffer, ct);
                    if (n == 0) break; // ffmpeg exited — fall through to reconnect

                    var frames = extractor.Feed(buffer.AsSpan(0, n));
                    if (frames.Count > 0)
                    {
                        if (!gotAnyFrame) Status = CameraStreamStatus.Streaming;
                        gotAnyFrame = true;
                        reconnects = 0; // this attempt produced real video — reset the give-up budget
                        foreach (var frame in frames) Broadcast(frame);
                    }
                }
            }
            catch (OperationCanceledException)
            {
                TryKill(ffmpeg);
                proxyListener?.Stop();
                break;
            }
            catch (Exception ex)
            {
                _logger.LogWarning(ex, "Camera pipeline error for {Ip}", _ipAddress);
            }

            var stderrText = stderrTask is null ? "" : await SafeAwaitAsync(stderrTask);
            TryKill(ffmpeg);
            proxyListener?.Stop();

            if (ct.IsCancellationRequested) break;

            reconnects++;
            // Never producing a frame gives up fast (likely unreachable/wrong credentials);
            // a stream that was working and then dropped gets more budget — P2S is documented
            // to drop RTSP sessions after a few seconds as routine behavior, not failure.
            var giveUpAfter = gotAnyFrame ? MaxReconnects : 3;
            if (reconnects >= giveUpAfter)
            {
                lock (_lock)
                {
                    Status = CameraStreamStatus.Unavailable;
                    LastError = string.IsNullOrWhiteSpace(stderrText)
                        ? "Camera stream failed — printer may be off or camera disabled."
                        : $"Camera stream failed: {stderrText.Trim()}";
                }
                return;
            }

            try { await Task.Delay(ReconnectDelay, ct); }
            catch (OperationCanceledException) { break; }
        }

        lock (_lock)
        {
            if (Status != CameraStreamStatus.Unavailable)
                Status = CameraStreamStatus.NotStarted;
        }
    }

    private Process StartFfmpeg(int proxyPort)
    {
        var url = $"rtsp://bblp:{_accessCode}@127.0.0.1:{proxyPort}/streaming/live/1";
        var psi = new ProcessStartInfo("ffmpeg")
        {
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
        };
        // P2S-tuned (confirmed against maziggy/bambuddy's camera_profiles.py, #1395): the
        // default fast-startup probesize/analyzeduration (32 bytes / 0) can't lock onto this
        // firmware's slow keyframe pacing, and its RTP timestamps don't advance — without
        // -use_wallclock_as_timestamps, ffmpeg's default CFR conversion freezes after frame 1.
        foreach (var arg in new[]
        {
            "-rtsp_transport", "tcp",
            "-rtsp_flags", "prefer_tcp",
            "-timeout", "30000000",
            "-buffer_size", "1024000",
            "-max_delay", "500000",
            "-probesize", "1000000",
            "-analyzeduration", "500000",
            "-fflags", "nobuffer",
            "-flags", "low_delay",
            "-use_wallclock_as_timestamps", "1",
            "-i", url,
            "-f", "mjpeg",
            "-q:v", "5",
            "-r", "10",
            "-an",
            "-",
        })
            psi.ArgumentList.Add(arg);

        return Process.Start(psi) ?? throw new InvalidOperationException("Failed to start ffmpeg");
    }

    private static async Task<string> DrainStderrAsync(Process process)
    {
        try
        {
            var text = await process.StandardError.ReadToEndAsync();
            var lines = text.Split('\n', StringSplitOptions.RemoveEmptyEntries);
            return string.Join(" | ", lines[^Math.Min(lines.Length, 20)..]);
        }
        catch { return ""; /* process gone */ }
    }

    private static async Task<string> SafeAwaitAsync(Task<string> task)
    {
        try { return await task; }
        catch { return ""; }
    }

    private static void TryKill(Process? process)
    {
        if (process is null) return;
        try { if (!process.HasExited) process.Kill(entireProcessTree: true); } catch { /* already gone */ }
        try { process.Dispose(); } catch { /* already disposed */ }
    }

    // Bambu's RTSPS data channel needs OpenSSL/.NET's SslStream (fine here, unlike ffmpeg's
    // own GnuTLS-linked builds on Debian, which reject this printer's TLS renegotiation and
    // drop the stream after a few seconds) — this proxy terminates TLS itself and hands
    // ffmpeg a plain rtsp:// localhost connection. Pin TLS 1.2: the same P2S firmware family
    // that needed this in the print-start FTPS fix (see docs/adr around the print-start
    // debugging) has a TLS 1.3 session-handling quirk here too.
    private static (int port, TcpListener listener) StartTlsProxy(string targetHost, int targetPort, CancellationToken ct)
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        var proxyPort = ((IPEndPoint)listener.LocalEndpoint).Port;
        _ = AcceptLoopAsync(listener, targetHost, targetPort, proxyPort, ct);
        return (proxyPort, listener);
    }

    private static async Task AcceptLoopAsync(TcpListener listener, string targetHost, int targetPort, int proxyPort, CancellationToken ct)
    {
        while (!ct.IsCancellationRequested)
        {
            TcpClient client;
            try { client = await listener.AcceptTcpClientAsync(ct); }
            catch { break; } // listener stopped/cancelled
            _ = HandleProxyClientAsync(client, targetHost, targetPort, proxyPort, ct);
        }
    }

    private static async Task HandleProxyClientAsync(TcpClient client, string targetHost, int targetPort, int proxyPort, CancellationToken ct)
    {
        using var _client = client;
        TcpClient? upstream = null;
        try
        {
            upstream = new TcpClient();
            await upstream.ConnectAsync(targetHost, targetPort, ct);
            await using var sslStream = new SslStream(upstream.GetStream(), leaveInnerStreamOpen: false, (_, _, _, _) => true);
            await sslStream.AuthenticateAsClientAsync(new SslClientAuthenticationOptions
            {
                TargetHost = targetHost,
            }, ct);

            var clientStream = client.GetStream();
            // Must include the scheme, not just host:port — the printer serves RTSPS-only and
            // 301-redirects any request-line whose URL scheme says plain "rtsp://" (what
            // ffmpeg's proxy-facing URL uses, since ffmpeg itself never sees TLS). Matches
            // bambuddy's rewrite_rtsp_request_url, whose real_url is likewise "rtsps://host:port".
            var proxyUrl = $"rtsp://127.0.0.1:{proxyPort}";
            var realUrl = $"rtsps://{targetHost}:{targetPort}";

            var toServer = ForwardAsync(clientStream, sslStream, data => RewriteRequestLine(data, proxyUrl, realUrl), ct);
            var toClient = ForwardAsync(sslStream, clientStream, data => RewriteResponseHost(data, targetHost, targetPort, proxyPort), ct);
            await Task.WhenAny(toServer, toClient);
        }
        catch
        {
            // Either side dropped — nothing further to clean up beyond the disposals below.
        }
        finally
        {
            upstream?.Dispose();
        }
    }

    private static async Task ForwardAsync(Stream src, Stream dst, Func<byte[], byte[]> transform, CancellationToken ct)
    {
        var buffer = new byte[65536];
        try
        {
            while (true)
            {
                var n = await src.ReadAsync(buffer, ct);
                if (n == 0) break;
                await dst.WriteAsync(transform(buffer[..n]), ct);
            }
        }
        catch
        {
            // Peer dropped mid-forward — normal on stream teardown.
        }
    }

    // RTSP request lines have the form "METHOD <url> RTSP/1.0\r\n" — rewriting only that
    // line (not a blind whole-buffer replace) leaves any Authorization header intact,
    // matching bambuddy's rewrite_rtsp_request_url. Safe to treat this direction as ASCII
    // text: ffmpeg-as-client only ever sends RTSP control lines here, never binary RTP data
    // (that flows server→client, handled separately by RewriteResponseHost).
    private static byte[] RewriteRequestLine(byte[] data, string proxyUrl, string realUrl)
    {
        var text = Encoding.ASCII.GetString(data);
        if (!text.Contains(" RTSP/1.0", StringComparison.Ordinal))
            return data;

        var lines = text.Split("\r\n");
        for (var i = 0; i < lines.Length; i++)
        {
            if (lines[i].EndsWith(" RTSP/1.0", StringComparison.Ordinal))
            {
                lines[i] = lines[i].Replace(proxyUrl, realUrl, StringComparison.Ordinal);
                break;
            }
        }
        return Encoding.ASCII.GetBytes(string.Join("\r\n", lines));
    }

    // RTSP responses ("RTSP/1.0 200 OK" status line) can embed the printer's own real address
    // in headers like Content-Base or a redirect Location — confirmed live: a 301 response
    // handed ffmpeg the printer's bare real IP, and ffmpeg followed it with a brand-new
    // *unproxied* connection that then failed ffmpeg's own stricter TLS validation against
    // the printer's self-signed cert (the exact failure this proxy exists to avoid). Rewrite
    // both "host:port" and bare "host" (a redirect can omit the port) to the proxy's address
    // so any address ffmpeg learns from the server always routes back through this proxy.
    // Also downgrade the "rtsps://" scheme to "rtsp://": the redirect keeps the printer's own
    // (encrypted) scheme, but this proxy's ffmpeg-facing listener is plain TCP — TLS is only
    // terminated on the upstream (real-printer) side — so a same-scheme redirect back to the
    // proxy address makes ffmpeg attempt a second TLS handshake against a listener that isn't
    // speaking TLS, which just hangs with no error. Binary interleaved RTP data (RFC 2326
    // §10.12, marked with a leading '$') never starts with "RTSP/1.0", so none of this ever
    // touches video payload — only genuine text responses.
    private static byte[] RewriteResponseHost(byte[] data, string targetHost, int targetPort, int proxyPort)
    {
        if (data.Length < 8 || Encoding.ASCII.GetString(data, 0, 8) != "RTSP/1.0")
            return data;

        // Only the header block is rewritten — a DESCRIBE response's SDP body can legitimately
        // contain the printer's real address (e.g. the "o=" line) and must reach ffmpeg
        // byte-for-byte, since Content-Length is computed over the *original* body and any
        // length-changing rewrite there desyncs ffmpeg's framing of the response.
        var text = Encoding.ASCII.GetString(data);
        var headerEnd = text.IndexOf("\r\n\r\n", StringComparison.Ordinal);
        var headerLen = headerEnd == -1 ? text.Length : headerEnd + 4;

        var proxyHostPort = $"127.0.0.1:{proxyPort}";
        var header = text[..headerLen]
            .Replace($"{targetHost}:{targetPort}", proxyHostPort, StringComparison.Ordinal)
            .Replace(targetHost, proxyHostPort, StringComparison.Ordinal)
            .Replace("rtsps://", "rtsp://", StringComparison.Ordinal);
        return Encoding.ASCII.GetBytes(header + text[headerLen..]);
    }
}
