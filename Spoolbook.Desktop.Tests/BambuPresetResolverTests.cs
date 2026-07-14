using System.Text.Json;
using Spoolbook.Desktop.Features.BambuImport;
namespace Spoolbook.Desktop.Tests;

public class BambuPresetResolverTests : IDisposable
{
    private readonly string _root;
    private readonly string _userDir;
    private readonly string _systemDir;

    public BambuPresetResolverTests()
    {
        _root = Path.Combine(Path.GetTempPath(), "spoolbook-test-" + Guid.NewGuid());
        _userDir = Path.Combine(_root, "user", "filament");
        _systemDir = Path.Combine(_root, "system");
        Directory.CreateDirectory(_userDir);
        Directory.CreateDirectory(_systemDir);
    }

    public void Dispose() => Directory.Delete(_root, recursive: true);

    private static string GetValue(Dictionary<string, JsonElement> merged, string key) =>
        merged[key].EnumerateArray().First().GetString()!;

    [Fact]
    public async Task Resolve_ReturnsLeafFieldsWhenSelfContained()
    {
        var leaf = """{"name":"Base","inherits":"","nozzle_temperature":["230"]}""";

        var resolver = new BambuPresetResolver(_userDir, _systemDir);
        var result = await resolver.ResolveAsync(leaf);

        Assert.Equal("230", GetValue(result, "nozzle_temperature"));
    }

    [Fact]
    public async Task Resolve_MergesFromUserParentPreset_ChildWins()
    {
        File.WriteAllText(Path.Combine(_userDir, "parent.json"), """
            {"name":"Parent","inherits":"","nozzle_temperature":["200"],"hot_plate_temp":["55"]}
            """);
        var leaf = """{"name":"Child","inherits":"Parent","nozzle_temperature":["230"]}""";

        var resolver = new BambuPresetResolver(_userDir, _systemDir);
        var result = await resolver.ResolveAsync(leaf);

        Assert.Equal("230", GetValue(result, "nozzle_temperature")); // child wins
        Assert.Equal("55", GetValue(result, "hot_plate_temp")); // inherited from parent
    }

    [Fact]
    public async Task Resolve_WalksMultipleLevels_UserThenSystem()
    {
        // System vendor manifest maps a preset name -> file, mirroring BBL.json's filament_list.
        Directory.CreateDirectory(Path.Combine(_systemDir, "BBL", "filament"));
        File.WriteAllText(Path.Combine(_systemDir, "BBL.json"), """
            {"filament_list":[{"name":"SystemBase","sub_path":"filament/SystemBase.json"}]}
            """);
        File.WriteAllText(Path.Combine(_systemDir, "BBL", "filament", "SystemBase.json"), """
            {"name":"SystemBase","inherits":"","fan_max_speed":["100"]}
            """);
        File.WriteAllText(Path.Combine(_userDir, "mid.json"), """
            {"name":"Mid","inherits":"SystemBase","hot_plate_temp":["55"]}
            """);
        var leaf = """{"name":"Leaf","inherits":"Mid","nozzle_temperature":["230"]}""";

        var resolver = new BambuPresetResolver(_userDir, _systemDir);
        var result = await resolver.ResolveAsync(leaf);

        Assert.Equal("230", GetValue(result, "nozzle_temperature"));
        Assert.Equal("55", GetValue(result, "hot_plate_temp"));
        Assert.Equal("100", GetValue(result, "fan_max_speed"));
    }

    [Fact]
    public async Task Resolve_PrefersSameVendorWhenAncestorNameIsAmbiguousAcrossVendors()
    {
        // Two vendors both ship a preset literally named "Shared" (mirrors real Bambu Studio,
        // where every vendor folder has its own "fdm_filament_common"/"fdm_filament_pla" template).
        // "AAVendor" sorts before "ZZVendor" alphabetically/by directory listing, so a naive
        // scan-all-vendors-take-first-match lookup would wrongly grab AAVendor's copy even when
        // the chain actually belongs to ZZVendor.
        Directory.CreateDirectory(Path.Combine(_systemDir, "AAVendor", "filament"));
        Directory.CreateDirectory(Path.Combine(_systemDir, "ZZVendor", "filament"));

        File.WriteAllText(Path.Combine(_systemDir, "AAVendor.json"), """
            {"filament_list":[{"name":"Shared","sub_path":"filament/Shared.json"}]}
            """);
        File.WriteAllText(Path.Combine(_systemDir, "AAVendor", "filament", "Shared.json"), """
            {"name":"Shared","filament_shrink":["WRONG-VENDOR"]}
            """);

        File.WriteAllText(Path.Combine(_systemDir, "ZZVendor.json"), """
            {"filament_list":[
                {"name":"RootZ","sub_path":"filament/RootZ.json"},
                {"name":"Shared","sub_path":"filament/Shared.json"}
            ]}
            """);
        File.WriteAllText(Path.Combine(_systemDir, "ZZVendor", "filament", "RootZ.json"), """
            {"name":"RootZ","inherits":"Shared","nozzle_temperature":["230"]}
            """);
        File.WriteAllText(Path.Combine(_systemDir, "ZZVendor", "filament", "Shared.json"), """
            {"name":"Shared","filament_shrink":["100%"]}
            """);

        var leaf = """{"name":"Leaf","inherits":"RootZ","nozzle_temperature":["230"]}""";

        var resolver = new BambuPresetResolver(_userDir, _systemDir);
        var result = await resolver.ResolveAsync(leaf);

        Assert.Equal("100%", GetValue(result, "filament_shrink"));
    }

    [Fact]
    public async Task Resolve_StopsGracefullyWhenAncestorNotFound()
    {
        var leaf = """{"name":"Leaf","inherits":"Missing","nozzle_temperature":["230"]}""";

        var resolver = new BambuPresetResolver(_userDir, _systemDir);
        var result = await resolver.ResolveAsync(leaf);

        Assert.Equal("230", GetValue(result, "nozzle_temperature"));
        Assert.False(result.ContainsKey("hot_plate_temp"));
    }

    [Fact]
    public async Task Resolve_StopsOnCircularInherits()
    {
        File.WriteAllText(Path.Combine(_userDir, "a.json"), """{"name":"A","inherits":"B"}""");
        File.WriteAllText(Path.Combine(_userDir, "b.json"), """{"name":"B","inherits":"A"}""");
        var leaf = """{"name":"Leaf","inherits":"A","nozzle_temperature":["230"]}""";

        var resolver = new BambuPresetResolver(_userDir, _systemDir);
        var result = await resolver.ResolveAsync(leaf);

        Assert.Equal("230", GetValue(result, "nozzle_temperature"));
    }

    [Fact]
    public void ListSystemFilamentPresets_ReturnsNamesAndPathsFromVendorManifests()
    {
        Directory.CreateDirectory(Path.Combine(_systemDir, "BBL", "filament"));
        File.WriteAllText(Path.Combine(_systemDir, "BBL.json"), """
            {"filament_list":[
                {"name":"Bambu PLA Basic","sub_path":"filament/Bambu PLA Basic.json"},
                {"name":"Bambu PETG Basic","sub_path":"filament/Bambu PETG Basic.json"}
            ]}
            """);
        File.WriteAllText(Path.Combine(_systemDir, "BBL", "filament", "Bambu PLA Basic.json"), """{"name":"Bambu PLA Basic"}""");
        File.WriteAllText(Path.Combine(_systemDir, "BBL", "filament", "Bambu PETG Basic.json"), """{"name":"Bambu PETG Basic"}""");

        var resolver = new BambuPresetResolver(_userDir, _systemDir);
        var presets = resolver.ListSystemFilamentPresets();

        Assert.Equal(new[] { "Bambu PETG Basic", "Bambu PLA Basic" }, presets.Select(p => p.Name));
        Assert.All(presets, p => Assert.True(File.Exists(p.FilePath)));
    }

    [Fact]
    public void ListUserFilamentPresets_ReturnsNamesAndPathsFromUserDir()
    {
        File.WriteAllText(Path.Combine(_userDir, "yellow.json"), """{"name":"Bambu PLA Basic - FRC - Yellow","inherits":"Bambu PLA Basic"}""");
        File.WriteAllText(Path.Combine(_userDir, "black.json"), """{"name":"Bambu PLA Basic - FRC - Black","inherits":"Bambu PLA Basic"}""");

        var resolver = new BambuPresetResolver(_userDir, _systemDir);
        var presets = resolver.ListUserFilamentPresets();

        Assert.Equal(
            new[] { "Bambu PLA Basic - FRC - Black", "Bambu PLA Basic - FRC - Yellow" },
            presets.Select(p => p.Name));
        Assert.All(presets, p => Assert.True(File.Exists(p.FilePath)));
    }
}
