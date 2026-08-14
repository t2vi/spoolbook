using System.Text.Json.Nodes;
using Spoolbook.Desktop.Features.Profiles;
namespace Spoolbook.Desktop.Tests;

// Covers ProfileConfigPatcher — patches a real Project's Metadata/project_settings.config with a
// PrintProfile's fields for the re-slice-before-print feature (docs — 2026-08-14 grill session).
// Fixture (Fixtures/project_settings_sample.json) is a trimmed real export, not synthetic data.
public class ProfileConfigPatcherTests
{
    private static string LoadFixture() =>
        File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "Fixtures", "project_settings_sample.json"));

    private static PrintProfile MinimalProfile() => new() { Name = "Test", FilamentId = 1, NozzleTempC = 240 };

    [Fact]
    public void Patch_SetsScalarFieldMappedToArrayKey_AtIndexZeroOnly()
    {
        var profile = MinimalProfile();
        profile.NozzleTempC = 250;

        var result = ProfileConfigPatcher.Patch(LoadFixture(), profile);

        var node = JsonNode.Parse(result)!.AsObject();
        var arr = node["nozzle_temperature"]!.AsArray();
        Assert.Equal("250", arr[0]!.GetValue<string>());
        Assert.Equal("240", arr[1]!.GetValue<string>()); // second filament's value untouched
    }

    [Fact]
    public void Patch_PreservesShorterArraysAndTwoElementArrays_AtIndexZero()
    {
        var profile = MinimalProfile();
        profile.NozzleTempInitialC = 260;

        var result = ProfileConfigPatcher.Patch(LoadFixture(), profile);

        var arr = JsonNode.Parse(result)!["nozzle_temperature_initial_layer"]!.AsArray();
        Assert.Equal("260", arr[0]!.GetValue<string>());
        Assert.Equal("245", arr[1]!.GetValue<string>());
    }

    [Fact]
    public void Patch_ConvertsBoolToZeroOne_NotTrueFalse()
    {
        var profile = MinimalProfile();
        profile.LongRetractionsWhenCut = true;

        var result = ProfileConfigPatcher.Patch(LoadFixture(), profile);

        var arr = JsonNode.Parse(result)!["long_retractions_when_cut"]!.AsArray();
        Assert.Equal("1", arr[0]!.GetValue<string>());
    }

    [Fact]
    public void Patch_LeavesNullFields_OriginalValueUntouched()
    {
        var profile = MinimalProfile(); // HotPlateTempC left null

        var result = ProfileConfigPatcher.Patch(LoadFixture(), profile);

        var arr = JsonNode.Parse(result)!["hot_plate_temp"]!.AsArray();
        Assert.Equal("70", arr[0]!.GetValue<string>()); // fixture's original value, unchanged
    }

    [Fact]
    public void Patch_LeavesKeysNotOwnedByPrintProfile_Untouched()
    {
        var profile = MinimalProfile();
        profile.NozzleTempC = 999;

        var result = ProfileConfigPatcher.Patch(LoadFixture(), profile);

        var node = JsonNode.Parse(result)!.AsObject();
        Assert.Equal("0.2", node["layer_height"]!.GetValue<string>());
        Assert.Equal("15%", node["sparse_infill_density"]!.GetValue<string>());
        Assert.Equal("Bambu Lab P2S", node["printer_model"]!.GetValue<string>());
    }

    [Fact]
    public void Patch_SetsScalarNonArrayKey()
    {
        // required_nozzle_HRC is array-valued in the real config; sanity-check a plain int?
        // field round-trips correctly through ToConfigString's IFormattable path.
        var profile = MinimalProfile();
        profile.RequiredNozzleHrc = 5;

        var result = ProfileConfigPatcher.Patch(LoadFixture(), profile);

        var arr = JsonNode.Parse(result)!["required_nozzle_HRC"]!.AsArray();
        Assert.Equal("5", arr[0]!.GetValue<string>());
    }
}
