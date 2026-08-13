using Spoolbook.Desktop.Services.Printer;
namespace Spoolbook.Desktop.Tests;

public class BambuMqttPayloadParserTests
{
    // Trimmed to the fields the parser actually reads, but real key names/types/values
    // captured from a live Bambu P2S "report" MQTT message during an active print.
    private const string RunningStatusJson = """
    {
        "print": {
            "gcode_state": "RUNNING",
            "task_id": "1725",
            "nozzle_temper": 240.0,
            "nozzle_target_temper": 240.0,
            "bed_temper": 70.0,
            "bed_target_temper": 70.0,
            "mc_percent": 8,
            "layer_num": 21,
            "total_layer_num": 908,
            "ams": {
                "tray_now": "0"
            }
        }
    }
    """;

    [Fact]
    public void Parse_ExtractsCoreFields_FromRunningStatus()
    {
        var result = BambuMqttPayloadParser.Parse(RunningStatusJson);

        Assert.NotNull(result);
        Assert.Equal("RUNNING", result!.GcodeState);
        Assert.Equal("1725", result.TaskId);
        Assert.Equal(240.0m, result.Reading.NozzleTempC);
        Assert.Equal(70.0m, result.Reading.BedTempC);
        Assert.Equal("0", result.Reading.AmsSlot);
        Assert.Equal(8, result.Reading.ProgressPct);
    }

    // Community-documented Bambu AMS schema (not yet verified against a captured payload from
    // this user's own P2S, unlike the payload above) — confirm field names against a live
    // capture before shipping AMS UI on top of this.
    private const string RunningStatusWithFullAmsJson = """
    {
        "print": {
            "gcode_state": "RUNNING",
            "task_id": "1725",
            "nozzle_temper": 240.0,
            "bed_temper": 70.0,
            "mc_percent": 8,
            "ams": {
                "tray_now": "0",
                "ams": [
                    {
                        "id": "0",
                        "humidity": "5",
                        "tray": [
                            { "id": "0", "tray_type": "PLA", "tray_color": "FFFFFFFF", "remain": 72 },
                            { "id": "1", "tray_type": "PETG", "tray_color": "1A1A1AFF", "remain": 40 },
                            { "id": "2", "tray_type": "", "tray_color": "", "remain": -1 },
                            { "id": "3", "tray_type": "ABS", "tray_color": "FF0000FF", "remain": 15 }
                        ]
                    }
                ]
            }
        }
    }
    """;

    [Fact]
    public void Parse_ExtractsFullAmsInventory_WhenPresent()
    {
        var result = BambuMqttPayloadParser.Parse(RunningStatusWithFullAmsJson);

        Assert.NotNull(result);
        var unit = Assert.Single(result!.AmsUnits);
        Assert.Equal("0", unit.UnitId);
        Assert.Equal(5, unit.HumidityLevel);
        Assert.Equal(4, unit.Trays.Count);

        Assert.Equal("PLA", unit.Trays[0].MaterialType);
        Assert.Equal("FFFFFFFF", unit.Trays[0].ColorHex);
        Assert.Equal(72, unit.Trays[0].RemainPercent);

        Assert.Null(unit.Trays[2].MaterialType);
        Assert.Null(unit.Trays[2].ColorHex);
        Assert.Null(unit.Trays[2].RemainPercent);
    }

    [Fact]
    public void Parse_ReturnsEmptyAmsUnits_WhenAmsArrayAbsent()
    {
        var result = BambuMqttPayloadParser.Parse(RunningStatusJson);

        Assert.NotNull(result);
        Assert.Empty(result!.AmsUnits);
    }

    [Fact]
    public void Parse_LeavesChamberTempNull_WhenFieldAbsent()
    {
        // P2S's real payloads never included chamber_temper in observed captures — the field
        // is nullable throughout, this just confirms absence doesn't throw.
        var result = BambuMqttPayloadParser.Parse(RunningStatusJson);

        Assert.Null(result!.Reading.ChamberTempC);
    }

    [Fact]
    public void Parse_ReturnsNull_WhenNoPrintKey()
    {
        var result = BambuMqttPayloadParser.Parse("""{ "system": { "sequence_id": "1" } }""");

        Assert.Null(result);
    }

    [Fact]
    public void Parse_ReturnsNull_WhenGcodeStateMissing()
    {
        // Bambu's broker also emits partial/delta messages that omit gcode_state — treated as
        // not a usable status snapshot rather than guessing a state.
        var result = BambuMqttPayloadParser.Parse("""{ "print": { "nozzle_temper": 210.0 } }""");

        Assert.Null(result);
    }

    [Fact]
    public void Parse_ReturnsNull_ForMalformedJson()
    {
        var result = BambuMqttPayloadParser.Parse("not json");

        Assert.Null(result);
    }

    [Theory]
    [InlineData("RUNNING", true)]
    [InlineData("PAUSE", true)]
    [InlineData("PREPARE", true)]
    [InlineData("running", true)]
    [InlineData("IDLE", false)]
    [InlineData("FINISH", false)]
    [InlineData("FAILED", false)]
    public void IsActiveState_ClassifiesGcodeState(string state, bool expected)
    {
        Assert.Equal(expected, BambuMqttPayloadParser.IsActiveState(state));
    }
}
