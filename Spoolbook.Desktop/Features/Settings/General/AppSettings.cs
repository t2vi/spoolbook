namespace Spoolbook.Desktop.Features.Settings.General;

public class AppSettings
{
    public int Id { get; set; }
    public string? BambuUserPresetsDir { get; set; }
    public string? BambuSystemProfilesDir { get; set; }
    public DateTime? LastFilamentSyncAt { get; set; }
    // Newline-separated list of additional filament-catalog URLs, each expected to serve the
    // same JSON shape as FilamentCatalogSyncService.CatalogUrl.
    public string? AdditionalFilamentSourceUrls { get; set; }
}
