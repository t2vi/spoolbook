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
