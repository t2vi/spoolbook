using System.Globalization;
using System.Reflection;
namespace Spoolbook.Desktop.Features.Profiles;

// Reflection-based mapper for PrintProfile's ~125 dynamic settings fields — avoids
// hand-writing the same field list three times (model, input, service assignment).
public static class ProfileFieldMapper
{
    private static readonly HashSet<string> BaseFieldNames = new()
    {
        "Id", "FilamentId", "Filament", "SpoolId", "Spool", "Name", "NozzleTempC", "NozzleTempInitialC",
        "Source", "SourceSlicer", "RawSettingsJson", "SourcePresetPath", "VersionNumber", "VersionName", "IsCurrentVersion", "Notes", "CreatedAt"
    };

    public static readonly IReadOnlyList<PropertyInfo> DynamicProperties = typeof(PrintProfile)
        .GetProperties()
        .Where(p => p.CanWrite && !BaseFieldNames.Contains(p.Name))
        .ToList();

    public static void Apply(PrintProfile target, IReadOnlyDictionary<string, string> fields)
    {
        foreach (var prop in DynamicProperties)
        {
            if (!fields.TryGetValue(prop.Name, out var raw)) continue;
            prop.SetValue(target, Convert(raw, prop.PropertyType));
        }
    }

    public static Dictionary<string, string> ToFieldStrings(PrintProfile profile)
    {
        var result = new Dictionary<string, string>();
        foreach (var prop in DynamicProperties)
        {
            var value = prop.GetValue(profile);
            result[prop.Name] = value switch
            {
                null => "",
                bool b => b ? "true" : "false",
                IFormattable f => f.ToString(null, CultureInfo.InvariantCulture),
                _ => value.ToString() ?? ""
            };
        }
        return result;
    }

    private static object? Convert(string? raw, Type targetType)
    {
        var underlying = Nullable.GetUnderlyingType(targetType) ?? targetType;

        if (string.IsNullOrWhiteSpace(raw)) return null;

        if (underlying == typeof(string)) return raw;
        if (underlying == typeof(int)) return int.TryParse(raw, NumberStyles.Integer, CultureInfo.InvariantCulture, out var i) ? i : null;
        if (underlying == typeof(decimal)) return decimal.TryParse(raw, NumberStyles.Float, CultureInfo.InvariantCulture, out var d) ? d : null;
        if (underlying == typeof(bool)) return raw == "true" ? true : raw == "false" ? false : (object?)null;

        return null;
    }
}
