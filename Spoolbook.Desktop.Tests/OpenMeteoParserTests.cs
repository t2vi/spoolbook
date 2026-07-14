using Spoolbook.Desktop.Features.Prints;
namespace Spoolbook.Desktop.Tests;

public class OpenMeteoParserTests
{
    private const string SampleJson = """
        {
          "hourly": {
            "time": ["2026-01-01T06:00", "2026-01-01T07:00", "2026-01-01T08:00", "2026-01-01T09:00", "2026-01-01T10:00"],
            "temperature_2m": [16.0, 18.0, 20.0, 22.0, 24.0],
            "relative_humidity_2m": [70, 65, 60, 55, 50]
          }
        }
        """;

    [Fact]
    public void ParseAmbient_AveragesHoursWithinWindow()
    {
        var (temp, humidity) = OpenMeteoParser.ParseAmbient(
            SampleJson,
            new DateTime(2026, 1, 1, 7, 0, 0, DateTimeKind.Utc),
            new DateTime(2026, 1, 1, 9, 0, 0, DateTimeKind.Utc));

        Assert.Equal(20.0m, temp);
        Assert.Equal(60.0m, humidity);
    }

    [Fact]
    public void ParseAmbient_NoHoursWithinWindow_ReturnsNull()
    {
        var (temp, humidity) = OpenMeteoParser.ParseAmbient(
            SampleJson,
            new DateTime(2026, 2, 1, 7, 0, 0, DateTimeKind.Utc),
            new DateTime(2026, 2, 1, 9, 0, 0, DateTimeKind.Utc));

        Assert.Null(temp);
        Assert.Null(humidity);
    }

    [Fact]
    public void ParseAmbient_InvalidJson_ReturnsNull()
    {
        var (temp, humidity) = OpenMeteoParser.ParseAmbient(
            "not json",
            new DateTime(2026, 1, 1, 7, 0, 0, DateTimeKind.Utc),
            new DateTime(2026, 1, 1, 9, 0, 0, DateTimeKind.Utc));

        Assert.Null(temp);
        Assert.Null(humidity);
    }
}
