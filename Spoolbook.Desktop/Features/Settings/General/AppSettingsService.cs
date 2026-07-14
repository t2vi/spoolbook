using Spoolbook.Desktop.Data;
namespace Spoolbook.Desktop.Features.Settings.General;

public class AppSettingsInput
{
    public string? BambuUserPresetsDir { get; set; }
    public string? BambuSystemProfilesDir { get; set; }
    public string? AdditionalFilamentSourceUrls { get; set; }
}

public class AppSettingsService
{
    private readonly SpoolbookDbContext _db;

    public AppSettingsService(SpoolbookDbContext db)
    {
        _db = db;
    }

    public async Task<AppSettings> GetAsync()
    {
        var settings = await _db.AppSettings.FindAsync(1);
        if (settings is not null) return settings;

        settings = new AppSettings { Id = 1 };
        _db.AppSettings.Add(settings);
        await _db.SaveChangesAsync();
        return settings;
    }

    public async Task SaveAsync(AppSettingsInput input)
    {
        var settings = await GetAsync();
        settings.BambuUserPresetsDir = string.IsNullOrWhiteSpace(input.BambuUserPresetsDir) ? null : input.BambuUserPresetsDir;
        settings.BambuSystemProfilesDir = string.IsNullOrWhiteSpace(input.BambuSystemProfilesDir) ? null : input.BambuSystemProfilesDir;
        settings.AdditionalFilamentSourceUrls = string.IsNullOrWhiteSpace(input.AdditionalFilamentSourceUrls) ? null : input.AdditionalFilamentSourceUrls;
        await _db.SaveChangesAsync();
    }

    public async Task<IReadOnlyList<string>> GetAdditionalFilamentSourceUrlsAsync()
    {
        var settings = await GetAsync();
        return (settings.AdditionalFilamentSourceUrls ?? "")
            .Split('\n', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
            .ToList();
    }

    public async Task RecordFilamentSyncAsync()
    {
        var settings = await GetAsync();
        settings.LastFilamentSyncAt = DateTime.UtcNow;
        await _db.SaveChangesAsync();
    }
}
