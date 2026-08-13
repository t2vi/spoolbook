using System.Text.Json;
using Spoolbook.Desktop.Features.Prints;

namespace Spoolbook.Desktop.Services.Printer;

public class BambuTelemetryMessage
{
    public required string GcodeState { get; init; }
    public string? TaskId { get; init; }
    public required ReadingInput Reading { get; init; }
    public List<AmsUnitReading> AmsUnits { get; init; } = [];
}

// Live-only AMS snapshot — deliberately not persisted onto Reading/PrinterReading (docs/adr/0022):
// the point is a live status view, not a per-tray time series in print history.
public class AmsUnitReading
{
    public required string UnitId { get; init; }
    public int? HumidityLevel { get; init; }
    public List<AmsTrayReading> Trays { get; init; } = [];
}

public class AmsTrayReading
{
    public required string SlotId { get; init; }
    public string? MaterialType { get; init; }
    public string? ColorHex { get; init; }
    public int? RemainPercent { get; init; }
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
            var amsUnits = new List<AmsUnitReading>();
            if (print.TryGetProperty("ams", out var ams))
            {
                if (TryGetString(ams, "tray_now", out var trayNow))
                    amsSlot = trayNow;

                if (ams.TryGetProperty("ams", out var amsArray) && amsArray.ValueKind == JsonValueKind.Array)
                {
                    foreach (var unitEl in amsArray.EnumerateArray())
                        amsUnits.Add(ParseAmsUnit(unitEl));
                }
            }

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
                },
                AmsUnits = amsUnits
            };
        }
        catch (JsonException)
        {
            return null;
        }
    }

    private static AmsUnitReading ParseAmsUnit(JsonElement unitEl)
    {
        var trays = new List<AmsTrayReading>();
        if (unitEl.TryGetProperty("tray", out var trayArray) && trayArray.ValueKind == JsonValueKind.Array)
        {
            foreach (var trayEl in trayArray.EnumerateArray())
            {
                var materialType = TryGetString(trayEl, "tray_type", out var mt) && mt.Length > 0 ? mt : null;
                var colorHex = TryGetString(trayEl, "tray_color", out var ch) && ch.Length > 0 ? ch : null;
                var remain = TryGetInt(trayEl, "remain");

                trays.Add(new AmsTrayReading
                {
                    SlotId = TryGetString(trayEl, "id", out var trayId) ? trayId : "",
                    MaterialType = materialType,
                    ColorHex = colorHex,
                    // -1 is Bambu's "unknown" sentinel (untagged spool / no RFID read yet)
                    RemainPercent = remain is null or -1 ? null : remain
                });
            }
        }

        return new AmsUnitReading
        {
            UnitId = TryGetString(unitEl, "id", out var unitId) ? unitId : "",
            // Bambu reports humidity as a string digit, not a number, unlike most other fields here
            HumidityLevel = TryGetString(unitEl, "humidity", out var humidity) && int.TryParse(humidity, out var humidityLevel) ? humidityLevel : null,
            Trays = trays
        };
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
