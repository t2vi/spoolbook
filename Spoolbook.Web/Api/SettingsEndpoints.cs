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

        // Bambu*Dir fields are desktop-only (local filesystem paths on whichever machine runs
        // the Avalonia app) — preserved as-is so saving from here can't wipe a desktop user's
        // configured paths, since both apps share the same AppSettings row.
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
