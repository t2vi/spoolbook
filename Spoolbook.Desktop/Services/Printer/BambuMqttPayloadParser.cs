using System.Text.Json;
using Spoolbook.Desktop.Features.Prints;

namespace Spoolbook.Desktop.Services.Printer;

public class BambuTelemetryMessage
{
    public required string GcodeState { get; init; }
    public string? TaskId { get; init; }
    public required ReadingInput Reading { get; init; }
}

// Parses Bambu Lab's LAN MQTT "report" topic payload (device/{serial}/report) — pure JSON-in,
// DTO-out, no network/DB, so it's exercised directly by tests against captured real payloads.
// The actual MQTT connection lives in Spoolbook.Web (docs/adr/0017), which is the in-process
// host now per the homelab pivot (docs/adr/0018).
public static class BambuMqttPayloadParser
{
    private static readonly HashSet<string> ActiveStates =
        new(StringComparer.OrdinalIgnoreCase) { "RUNNING", "PAUSE", "PREPARE" };

    public static bool IsActiveState(string gcodeState) => ActiveStates.Contains(gcodeState);

    // Bambu's broker also emits partial/delta messages that omit gcode_state entirely — treated
    // as not a usable status snapshot (returns null) rather than guessing at missing state.
    public static BambuTelemetryMessage? Parse(string json)
    {
        try
        {
            using var doc = JsonDocument.Parse(json);
            if (!doc.RootElement.TryGetProperty("print", out var print)) return null;
            if (!TryGetString(print, "gcode_state", out var gcodeState)) return null;

            string? amsSlot = null;
            if (print.TryGetProperty("ams", out var ams) && TryGetString(ams, "tray_now", out var trayNow))
                amsSlot = trayNow;

            return new BambuTelemetryMessage
            {
                GcodeState = gcodeState,
                TaskId = TryGetString(print, "task_id", out var taskId) ? taskId : null,
                Reading = new ReadingInput
                {
                    NozzleTempC = TryGetDecimal(print, "nozzle_temper"),
                    BedTempC = TryGetDecimal(print, "bed_temper"),
                    ChamberTempC = TryGetDecimal(print, "chamber_temper"),
                    AmsSlot = amsSlot,
                    ProgressPct = TryGetInt(print, "mc_percent")
                }
            };
        }
        catch (JsonException)
        {
            return null;
        }
    }

    private static bool TryGetString(JsonElement obj, string name, out string value)
    {
        if (obj.TryGetProperty(name, out var el) && el.ValueKind == JsonValueKind.String)
        {
            value = el.GetString()!;
            return true;
        }
        value = "";
        return false;
    }

    private static decimal? TryGetDecimal(JsonElement obj, string name) =>
        obj.TryGetProperty(name, out var el) && el.ValueKind == JsonValueKind.Number ? el.GetDecimal() : null;

    private static int? TryGetInt(JsonElement obj, string name) =>
        obj.TryGetProperty(name, out var el) && el.ValueKind == JsonValueKind.Number ? el.GetInt32() : null;
}
