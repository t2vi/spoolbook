using System.Text.Json;
namespace Spoolbook.Desktop.Features.BambuImport;

// Walks a Bambu Studio filament preset's `inherits` chain to a fully-resolved flat
// settings dictionary — child values win, missing keys fall through to the parent.
// Ancestors live either as user presets (matched by internal "name" field) or as
// system presets (looked up via a vendor manifest's filament_list -> sub_path).
public class BambuPresetResolver
{
    private readonly string _userPresetsDir;
    private readonly string _systemProfilesDir;

    public BambuPresetResolver(string userPresetsDir, string systemProfilesDir)
    {
        _userPresetsDir = userPresetsDir;
        _systemProfilesDir = systemProfilesDir;
    }

    public async Task<Dictionary<string, JsonElement>> ResolveAsync(string leafJson)
    {
        var merged = new Dictionary<string, JsonElement>();
        var visited = new HashSet<string>();
        var currentJson = leafJson;
        string? vendor = null;

        while (true)
        {
            using var doc = JsonDocument.Parse(currentJson);
            var root = doc.RootElement;

            foreach (var prop in root.EnumerateObject())
                merged.TryAdd(prop.Name, prop.Value.Clone());

            if (!root.TryGetProperty("inherits", out var inheritsProp)) break;
            var inheritsName = inheritsProp.GetString();
            if (string.IsNullOrEmpty(inheritsName) || !visited.Add(inheritsName)) break;

            var found = await FindPresetByNameAsync(inheritsName, vendor);
            if (found is null) break;
            currentJson = found.Value.Json;
            vendor = found.Value.Vendor ?? vendor;
        }

        return merged;
    }

    public List<ImportedPreset> ListUserFilamentPresets()
    {
        var results = new List<ImportedPreset>();
        if (!Directory.Exists(_userPresetsDir)) return results;

        foreach (var file in Directory.GetFiles(_userPresetsDir, "*.json"))
        {
            var name = TryGetName(File.ReadAllText(file));
            if (name is not null)
                results.Add(new ImportedPreset { Name = name, FilePath = file });
        }

        return results.OrderBy(p => p.Name).ToList();
    }

    public List<ImportedPreset> ListSystemFilamentPresets()
    {
        var results = new List<ImportedPreset>();
        if (!Directory.Exists(_systemProfilesDir)) return results;

        foreach (var vendorFile in Directory.GetFiles(_systemProfilesDir, "*.json"))
        {
            var vendorName = Path.GetFileNameWithoutExtension(vendorFile);
            using var vendorDoc = JsonDocument.Parse(File.ReadAllText(vendorFile));
            if (!vendorDoc.RootElement.TryGetProperty("filament_list", out var list)) continue;

            foreach (var entry in list.EnumerateArray())
            {
                if (!entry.TryGetProperty("name", out var n) || !entry.TryGetProperty("sub_path", out var subPathProp)) continue;

                var fullPath = Path.Combine(_systemProfilesDir, vendorName, subPathProp.GetString()!);
                if (File.Exists(fullPath))
                    results.Add(new ImportedPreset { Name = n.GetString()!, FilePath = fullPath });
            }
        }

        return results.OrderBy(p => p.Name).ToList();
    }

    // Every vendor folder ships its own copy of shared template presets (e.g.
    // "fdm_filament_common", "fdm_filament_pla"), so a name lookup is ambiguous across
    // vendors. Once we know which vendor a chain belongs to (from resolving its first
    // system-level ancestor), stick to that vendor's own copy for the rest of the chain —
    // otherwise whichever vendor folder happens to sort first wins, silently pulling in
    // the wrong vendor's values.
    private async Task<(string Json, string? Vendor)?> FindPresetByNameAsync(string name, string? preferredVendor)
    {
        if (Directory.Exists(_userPresetsDir))
        {
            foreach (var file in Directory.GetFiles(_userPresetsDir, "*.json"))
            {
                var text = await File.ReadAllTextAsync(file);
                if (TryGetName(text) == name) return (text, null);
            }
        }

        if (Directory.Exists(_systemProfilesDir))
        {
            var vendorFiles = Directory.GetFiles(_systemProfilesDir, "*.json");
            IEnumerable<string> ordered = preferredVendor is null
                ? vendorFiles
                : vendorFiles.OrderByDescending(f => Path.GetFileNameWithoutExtension(f) == preferredVendor);

            foreach (var vendorFile in ordered)
            {
                var vendorName = Path.GetFileNameWithoutExtension(vendorFile);
                var vendorText = await File.ReadAllTextAsync(vendorFile);
                using var vendorDoc = JsonDocument.Parse(vendorText);
                if (!vendorDoc.RootElement.TryGetProperty("filament_list", out var list)) continue;

                foreach (var entry in list.EnumerateArray())
                {
                    if (!entry.TryGetProperty("name", out var n) || n.GetString() != name) continue;
                    if (!entry.TryGetProperty("sub_path", out var subPathProp)) continue;

                    // sub_path is relative to the vendor's own subfolder (e.g. BBL/), not the profiles root.
                    var fullPath = Path.Combine(_systemProfilesDir, vendorName, subPathProp.GetString()!);
                    if (File.Exists(fullPath)) return (await File.ReadAllTextAsync(fullPath), vendorName);
                }
            }
        }

        return null;
    }

    private static string? TryGetName(string json)
    {
        try
        {
            using var doc = JsonDocument.Parse(json);
            return doc.RootElement.TryGetProperty("name", out var n) ? n.GetString() : null;
        }
        catch (JsonException)
        {
            return null;
        }
    }
}
