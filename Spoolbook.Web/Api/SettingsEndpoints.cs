using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Settings.General;

namespace Spoolbook.Web.Api;

public record SettingsResponse(string? AdditionalFilamentSourceUrls, DateTime? LastFilamentSyncAt, string CatalogUrl);
public record SaveSettingsRequest(string? AdditionalFilamentSourceUrls);

public static class SettingsEndpoints
{
    public static void MapSettingsEndpoints(this WebApplication app)
    {
        var group = app.MapGroup("/api/settings");

        group.MapGet("", async (AppSettingsService appSettings) =>
        {
            var settings = await appSettings.GetAsync();
            return new SettingsResponse(settings.AdditionalFilamentSourceUrls, settings.LastFilamentSyncAt, FilamentCatalogSyncService.CatalogUrl);
        });

        // Bambu*Dir fields are local filesystem paths (Bambu Studio's own preset directories on
        // whichever machine runs Spoolbook.Web) — preserved as-is here so saving general settings
        // from this endpoint can't wipe them, since they share the same AppSettings row.
        group.MapPost("", async (SaveSettingsRequest req, AppSettingsService appSettings) =>
        {
            var current = await appSettings.GetAsync();
            await appSettings.SaveAsync(new AppSettingsInput
            {
                BambuUserPresetsDir = current.BambuUserPresetsDir,
                BambuSystemProfilesDir = current.BambuSystemProfilesDir,
                AdditionalFilamentSourceUrls = req.AdditionalFilamentSourceUrls
            });
            return Results.Ok(new { ok = true });
        }).RequireAuthorization();
    }
}
