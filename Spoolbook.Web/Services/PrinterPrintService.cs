using System.IO.Compression;
using System.Security.Cryptography;
using System.Text.Json;
using FluentFTP;
using FluentFTP.GnuTLS;
using FluentFTP.GnuTLS.Enums;
using MQTTnet;

namespace Spoolbook.Web.Services;

// Sends an already-sliced .3mf to the printer and starts it — the "print from spoolbook"
// action (docs/adr/0022). Two steps, both undocumented Bambu LAN-mode protocol, reverse-
// engineered by the community (matches BambuMqttPayloadParser's posture): FTPS upload of the
// file (port 990, implicit TLS, same bblp/access-code credential as MQTT), then a "project_file"
// command on the same live MQTT connection PrinterControlService reuses. Confirmed end-to-end
// against a real P2S 2026-08-14 (real task_id, RUNNING state, nozzle/bed heating to target).
public class PrinterPrintService
{
    private readonly PrinterLiveStatusStore _liveStatusStore;
    private readonly ILogger<PrinterPrintService> _logger;

    public PrinterPrintService(PrinterLiveStatusStore liveStatusStore, ILogger<PrinterPrintService> logger)
    {
        _liveStatusStore = liveStatusStore;
        _logger = logger;
    }

    public async Task<PrinterControlResult> StartPrintAsync(
        int printerId, string ipAddress, string accessCode, string serialNumber,
        string localFilePath, string displayFileName, string plateGcodeFileName, bool useAms = true,
        int amsSlot = 0, string? printerModel = null)
    {
        // Web uploads are stored on disk under a content-hash filename (ProjectUploadService,
        // ADR-0020) — that's fine for spoolbook's own storage, but sending it to the printer as
        // the on-device filename got rejected: "Unsupported file path or name" (Bambu error
        // 0500-4002 000314), confirmed against a real P2S. Use the human display name instead.
        var remoteFileName = SanitizeForPrinterFilename(displayFileName);
        // Confirmed against a real .3mf export: the printer validates this against the plate's
        // gcode specifically (matching the archive's own Metadata/plate_N.gcode.md5 sidecar),
        // not the whole .3mf file's checksum — whole-file md5 got "unable to parse the 3mf file"
        // (0502-402D) even with a correctly gcode-embedded export.
        var md5 = ComputeGcodeMd5(localFilePath, plateGcodeFileName);
        if (md5 is null)
            return new PrinterControlResult { Ok = false, Error = $"Couldn't find Metadata/{plateGcodeFileName} inside the .3mf — is this a sliced export, not just a saved project?" };

        try
        {
            using var ftp = new AsyncFtpClient(ipAddress, "bblp", accessCode, 990);
            ftp.Config.EncryptionMode = FtpEncryptionMode.Implicit;
            ftp.Config.ValidateAnyCertificate = true;
            // .NET's SslStream can't do TLS session reuse across the control/data channel, which
            // this printer's FTP server (vsftpd) requires — every upload was returning a silent
            // FtpStatus.Failed (data channel closed as "Broken pipe" mid-handshake) with no
            // exception, so the print command was being sent for a 0-byte file every time.
            // GnuTLS supports session reuse (same fix FileZilla and bambuddy use); confirmed
            // against a real P2S. See github.com/robinrodricks/FluentFTP/issues/1283.
            // Two more real, independently-confirmed quirks needed on top of the library swap:
            // GnuTLS's default ALPN offer gets a hard TLS alert (120, "no_application_protocol")
            // from this printer's firmware — disable ALPN entirely. And per bambuddy's
            // ftp_profiles.py (P2S firmware 01.02.00.00, #1401), TLS 1.3's async session-ticket
            // model breaks the data-channel session-reuse handshake on this printer's old vsFTPd
            // build — exclude TLS 1.3 so it negotiates 1.2.
            ftp.Config.CustomStream = typeof(GnuTlsStream);
            ftp.Config.CustomStreamConfig = new GnuConfig
            {
                SecurityOptions = [new GnuOption(GnuOperator.Exclude, GnuCommand.Protocol_Tls13)],
                SetALPNControlConnection = string.Empty,
                SetALPNDataConnection = string.Empty,
            };
            await ftp.AutoConnect();
            var uploadStatus = await ftp.UploadFile(localFilePath, $"/{remoteFileName}", FtpRemoteExists.Overwrite);
            await ftp.Disconnect();
            if (uploadStatus != FtpStatus.Success)
                return new PrinterControlResult { Ok = false, Error = $"File upload to printer failed ({uploadStatus})." };
            _logger.LogInformation("Uploaded {RemoteFileName} to printer via FTPS", remoteFileName);
        }
        catch (Exception ex)
        {
            var detail = ex.InnerException?.Message ?? ex.Message;
            return new PrinterControlResult { Ok = false, Error = $"File upload to printer failed: {detail}" };
        }

        var client = _liveStatusStore.GetConnectedClient(printerId);
        if (client is null)
            return new PrinterControlResult { Ok = false, Error = "Printer isn't connected — telemetry link is down or still reconnecting." };

        // Mirrors maziggy/bambuddy's start_print (backend/app/services/bambu_mqtt.py) — a mature,
        // production, issue-tracked implementation that special-cases exactly this hardware
        // (P2S/N7). Matching it field-for-field after OpenBambuAPI-doc-level guessing repeatedly
        // got a command accepted (err_code 0) but still failing later at [0502-402D] once AMS
        // validation passed.
        var submissionId = (DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() % 2_147_483_647) is var s && s == 0 ? "1" : s.ToString();
        var flatAmsMapping = useAms ? new[] { amsSlot } : [];
        // Global tray ID = ams_id*4 + slot_id (bambuddy's "regular AMS tray" case — spoolbook only
        // targets a single onboard AMS unit, not AMS-HT/external-spool/multi-nozzle setups).
        var amsMapping2 = useAms ? flatAmsMapping.Select(t => new { ams_id = t / 4, slot_id = t % 4 }) : [];
        // P2S doesn't support vibration calibration like X1/P1 — bambuddy forces this off
        // specifically for P2S/N7, confirmed against this printer's own printer_model_id "N7".
        var isP2S = printerModel?.Contains("P2S", StringComparison.OrdinalIgnoreCase) == true;

        var payload = JsonSerializer.Serialize(new
        {
            print = new
            {
                sequence_id = "20000",
                command = "project_file",
                param = $"Metadata/{plateGcodeFileName}",
                url = $"ftp:///{remoteFileName}",
                file = remoteFileName,
                md5,
                bed_type = "auto",
                timelapse = false,
                // bed_leveling stays a plain bool (true only when forced "on"); auto_bed_leveling
                // carries the tri-state (0=off/1=on/2=auto) — the two-field shape BambuStudio
                // actually sends. Spoolbook always requests "auto".
                bed_leveling = false,
                auto_bed_leveling = 2,
                flow_cali = false,
                vibration_cali = !isP2S,
                layer_inspect = false,
                use_ams = useAms,
                cfg = "0",
                extrude_cali_flag = 2,
                extrude_cali_manual_mode = 0,
                // Single-nozzle only (spoolbook has no dual-nozzle printer support) — always 0.
                nozzle_offset_cali = 0,
                subtask_name = Path.GetFileNameWithoutExtension(remoteFileName),
                profile_id = "0",
                // A fresh non-zero id per submission, not hardcoded "0" — hardcoded 0 makes
                // third-party MQTT observers see reprints as a continuation of the same job.
                project_id = submissionId,
                subtask_id = submissionId,
                task_id = submissionId,
                ams_mapping = flatAmsMapping,
                ams_mapping2 = amsMapping2
            }
        });

        var message = new MqttApplicationMessageBuilder()
            .WithTopic($"device/{serialNumber}/request")
            .WithPayload(payload)
            .Build();

        try
        {
            await client.PublishAsync(message);
            return new PrinterControlResult { Ok = true };
        }
        catch (Exception ex)
        {
            return new PrinterControlResult { Ok = false, Error = ex.Message };
        }
    }

    private static string? ComputeGcodeMd5(string localFilePath, string plateGcodeFileName)
    {
        using var archive = ZipFile.OpenRead(localFilePath);
        var entry = archive.GetEntry($"Metadata/{plateGcodeFileName}");
        if (entry is null) return null;

        using var stream = entry.Open();
        return Convert.ToHexString(MD5.HashData(stream)).ToLowerInvariant();
    }

    // Strips characters outside a conservative safe set and caps length — the exact rule the
    // printer's firmware enforces isn't documented, so this stays deliberately narrow rather
    // than guessing at what else might be "unsupported."
    private static string SanitizeForPrinterFilename(string displayFileName)
    {
        var name = Path.GetFileNameWithoutExtension(displayFileName);
        var safe = new string(name.Where(c => char.IsLetterOrDigit(c) || c is '_' or '-').ToArray());
        if (safe.Length == 0) safe = "print";
        if (safe.Length > 60) safe = safe[..60];
        return $"{safe}.3mf";
    }
}
