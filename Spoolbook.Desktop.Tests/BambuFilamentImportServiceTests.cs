using System.IO.Compression;
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

    // Mirrors a real Bambu Studio .3mf: a zip with Metadata/project_settings.config holding the
    // fully flattened, already-resolved settings (no "inherits" chain — Bambu bakes it in at slice time).
    private string WriteThreeMf(string projectSettingsJson, string entryName = "Metadata/project_settings.config")
    {
        var path = Path.Combine(_root, "project.3mf");
        using (var archive = ZipFile.Open(path, ZipArchiveMode.Create))
        {
            var entry = archive.CreateEntry(entryName);
            using var writer = new StreamWriter(entry.Open());
            writer.Write(projectSettingsJson);
        }
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

        var result = await _service.PushToFileAsync(path, new Dictionary<string, string> { ["NozzleTempC"] = "230" });

        Assert.True(result.Ok);
        using var doc = JsonDocument.Parse(await File.ReadAllTextAsync(path));
        Assert.Equal("230", doc.RootElement.GetProperty("nozzle_temperature")[0].GetString());
    }

    // ShrinkPct is stored (and edited in the UI) without a "%" — the field's unit label already
    // shows one — but Bambu's own raw JSON format expects it embedded back in, e.g. "99%".
    [Fact]
    public async Task PushToFileAsync_ReAppendsPercentSuffixForShrinkPct()
    {
        var path = WriteLeaf("""{"name": "Test Preset", "inherits": ""}""");

        var result = await _service.PushToFileAsync(path, new Dictionary<string, string> { ["ShrinkPct"] = "99" });

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
    public async Task ImportAsync_StripsPercentSuffixForShrinkPct()
    {
        var path = WriteLeaf("""
        {
            "name": "Test Preset",
            "filament_settings_id": ["Test Preset"],
            "filament_shrink": ["99%"]
        }
        """);

        var result = await _service.ImportAsync(path);

        Assert.True(result.Ok);
        Assert.Equal("99", result.Fields!["ShrinkPct"]);
    }

    [Fact]
    public async Task PushToFileAsync_ReturnsErrorForInvalidJson()
    {
        var path = WriteLeaf("not valid json");

        var result = await _service.PushToFileAsync(path, new Dictionary<string, string> { ["NozzleTempC"] = "230" });

        Assert.False(result.Ok);
        Assert.Equal("invalid_json", result.Error);
    }

    [Fact]
    public async Task ImportFromThreeMfAsync_ReadsFlattenedProjectSettings_NoInheritsWalking()
    {
        var path = WriteThreeMf("""
        {
            "nozzle_temperature": ["245", "240"],
            "filament_retraction_length": ["1", "0.4"],
            "hot_plate_temp_initial_layer": ["70"]
        }
        """);

        var result = await _service.ImportFromThreeMfAsync(path);

        Assert.True(result.Ok);
        Assert.Equal("245", result.Fields!["NozzleTempC"]);
        Assert.Equal("1", result.Fields!["RetractionMm"]);
        Assert.Equal("70", result.Fields!["HotPlateTempInitialC"]);
    }

    [Fact]
    public async Task ImportFromThreeMfAsync_ReturnsErrorWhenProjectSettingsConfigMissing()
    {
        var path = Path.Combine(_root, "empty.3mf");
        using (ZipFile.Open(path, ZipArchiveMode.Create)) { }

        var result = await _service.ImportFromThreeMfAsync(path);

        Assert.False(result.Ok);
        Assert.Equal("no_project_settings", result.Error);
    }

    [Fact]
    public async Task ImportFromThreeMfAsync_ReturnsErrorForCorruptZip()
    {
        var path = Path.Combine(_root, "corrupt.3mf");
        File.WriteAllText(path, "not a zip");

        var result = await _service.ImportFromThreeMfAsync(path);

        Assert.False(result.Ok);
        Assert.Equal("invalid_3mf", result.Error);
    }

    [Fact]
    public async Task ImportFromThreeMfAsync_AppliesSameBoolAndPercentConversionsAsRawPresetImport()
    {
        var path = WriteThreeMf("""
        {
            "filament_shrink": ["99%"],
            "filament_soluble": ["1"]
        }
        """);

        var result = await _service.ImportFromThreeMfAsync(path);

        Assert.True(result.Ok);
        Assert.Equal("99", result.Fields!["ShrinkPct"]);
        Assert.Equal("true", result.Fields!["Soluble"]);
    }
}
