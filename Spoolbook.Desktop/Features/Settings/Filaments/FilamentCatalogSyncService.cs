namespace Spoolbook.Desktop.Features.Settings.Filaments;

public class FilamentSyncResult
{
    public bool Ok { get; init; }
    public string? Error { get; init; }
    public List<FilamentInput> Entries { get; init; } = [];
}

// Catalog is scraped daily and published as a static file by a separate repo
// (github.com/t2vi/spoolbook-filament-sync). Fetching it directly, the same way
// OpenMeteoWeatherService fetches ambient weather, keeps this app server-free — no service of
// our own to deploy or authenticate against, matching the app's local-only, single-user design.
public class FilamentCatalogSyncService
{
    public const string CatalogUrl =
        "https://raw.githubusercontent.com/t2vi/spoolbook-filament-sync/main/data/filament-catalog.json";

    private readonly HttpClient _http = new();

    // The default source failing is a real error (nothing to import); a user-added extra
    // source failing just means that one source is skipped this run — one broken URL
    // shouldn't block the sync of everything else.
    public async Task<FilamentSyncResult> FetchAsync(IEnumerable<string>? additionalSourceUrls = null)
    {
        List<FilamentInput> entries;
        try
        {
            var json = await _http.GetStringAsync(CatalogUrl);
            entries = FilamentCatalogParser.Parse(json);
        }
        catch (HttpRequestException ex)
        {
            return new FilamentSyncResult { Ok = false, Error = ex.Message };
        }

        foreach (var url in additionalSourceUrls ?? [])
        {
            try
            {
                var json = await _http.GetStringAsync(url);
                entries.AddRange(FilamentCatalogParser.Parse(json));
            }
            catch (HttpRequestException)
            {
                // skip — see comment above
            }
        }

        return new FilamentSyncResult { Ok = true, Entries = entries };
    }
}
