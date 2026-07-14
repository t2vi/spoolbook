using System.Text.Json;
using Spoolbook.Desktop.Features.BambuImport;
namespace Spoolbook.Desktop.Tests;

public class BambuFilamentImportServiceTests : IDisposable
{
    private readonly string _root;
    private readonly BambuFilamentImportService _service;

    public BambuFilamentImportServiceTests()
    {
        _root = Path.Combine(Path.GetTempPath(), "spoolbook-push-test-" + Guid.NewGuid());
        Directory.CreateDirectory(_root);
        var resolver = new BambuPresetResolver(Path.Combine(_root, "user"), Path.Combine(_root, "system"));
        _service = new BambuFilamentImportService(resolver);
    }

    public void Dispose() => Directory.Delete(_root, recursive: true);

    private string WriteLeaf(string json)
    {
        var path = Path.Combine(_root, "leaf.json");
        File.WriteAllText(path, json);
        return path;
    }

    [Fact]
    public async Task PushToFileAsync_UpdatesManagedKeyValue_PreservesOtherKeysAndInherits()
    {
        var path = WriteLeaf("""
        {
            "name": "Test Preset",
            "inherits": "fdm_filament_pla",
            "filament_settings_id": ["Test Preset"],
            "nozzle_temperature": ["220"],
            "some_unmanaged_key": ["untouched"]
        }
        """);

        var result = await _service.PushToFileAsync(path, new Dictionary<string, string> { ["NozzleTempC"] = "230" });

        Assert.True(result.Ok);
        using var doc = JsonDocument.Parse(await File.ReadAllTextAsync(path));
        Assert.Equal("230", doc.RootElement.GetProperty("nozzle_temperature")[0].GetString());
        Assert.Equal("fdm_filament_pla", doc.RootElement.GetProperty("inherits").GetString());
        Assert.Equal("untouched", doc.RootElement.GetProperty("some_unmanaged_key")[0].GetString());
    }

    [Fact]
    public async Task PushToFileAsync_AddsKeyWhenNotPresentInLeafFile()
    {
        var path = WriteLeaf("""{"name": "Test Preset", "inherits": ""}""");

        var result = await _service.PushToFileAsync(path, new Dictionary<string, string> { ["ShrinkPct"] = "99%" });

        Assert.True(result.Ok);
        using var doc = JsonDocument.Parse(await File.ReadAllTextAsync(path));
        Assert.Equal("99%", doc.RootElement.GetProperty("filament_shrink")[0].GetString());
    }

    [Fact]
    public async Task PushToFileAsync_ConvertsBoolFieldsToOneOrZero()
    {
        var path = WriteLeaf("""{"name": "Test Preset", "inherits": ""}""");

        var result = await _service.PushToFileAsync(path, new Dictionary<string, string> { ["Soluble"] = "true" });

        Assert.True(result.Ok);
        using var doc = JsonDocument.Parse(await File.ReadAllTextAsync(path));
        Assert.Equal("1", doc.RootElement.GetProperty("filament_soluble")[0].GetString());
    }

    [Fact]
    public async Task PushToFileAsync_IgnoresUnmappedOrBlankFields()
    {
        var path = WriteLeaf("""{"name": "Test Preset", "inherits": ""}""");

        var result = await _service.PushToFileAsync(path, new Dictionary<string, string>
        {
            ["NotARealField"] = "whatever",
            ["ShrinkPct"] = ""
        });

        Assert.True(result.Ok);
        using var doc = JsonDocument.Parse(await File.ReadAllTextAsync(path));
        Assert.False(doc.RootElement.TryGetProperty("filament_shrink", out _));
    }

    [Fact]
    public async Task PushToFileAsync_ReturnsErrorForInvalidJson()
    {
        var path = WriteLeaf("not valid json");

        var result = await _service.PushToFileAsync(path, new Dictionary<string, string> { ["NozzleTempC"] = "230" });

        Assert.False(result.Ok);
        Assert.Equal("invalid_json", result.Error);
    }
}
