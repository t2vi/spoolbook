using System.Text.Json;
namespace Spoolbook.Desktop.Features.Settings.Filaments;

public static class FilamentCatalogParser
{
    public static List<FilamentInput> Parse(string json) =>
        JsonSerializer.Deserialize<List<FilamentInput>>(json, new JsonSerializerOptions { PropertyNameCaseInsensitive = true }) ?? [];
}
