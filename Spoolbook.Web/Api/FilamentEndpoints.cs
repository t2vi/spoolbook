using Spoolbook.Desktop.Features.Settings.Colors;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Settings.General;

namespace Spoolbook.Web.Api;

public static class FilamentEndpoints
{
    public static void MapFilamentEndpoints(this WebApplication app)
    {
        var group = app.MapGroup("/api/filaments");

        group.MapGet("", async (string? brand, string? material, int page, int pageSize, FilamentService filaments) =>
            await filaments.SearchAsync(new FilamentQuery
            {
                Brand = string.IsNullOrWhiteSpace(brand) ? null : brand,
                Material = string.IsNullOrWhiteSpace(material) ? null : material,
                Page = page == 0 ? 1 : page,
                PageSize = pageSize == 0 ? 20 : pageSize
            }));

        // Unpaged — for pickers (e.g. the Spool form's Filament dropdown) that need every row,
        // not a page of them. FilamentService itself has no GetAsync(id) either, so the edit
        // page finds-by-id from this same list client-side, same pattern as printers.
        group.MapGet("/all", async (FilamentService filaments) => await filaments.ListAsync());

        group.MapPost("", async (FilamentInput input, FilamentService filaments) =>
        {
            var result = await filaments.CreateAsync(input);
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        group.MapPut("/{id:int}", async (int id, FilamentInput input, FilamentService filaments) =>
        {
            var result = await filaments.UpdateAsync(id, input);
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        group.MapDelete("/{id:int}", async (int id, FilamentService filaments) =>
        {
            var result = await filaments.DeleteAsync(id);
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        // Mirrors FilamentList.razor's SyncCatalogAsync: fetch the published catalog, import new
        // entries, record the sync timestamp — one endpoint instead of three round trips.
        group.MapPost("/sync", async (AppSettingsService appSettings, FilamentService filaments) =>
        {
            var additionalSources = await appSettings.GetAdditionalFilamentSourceUrlsAsync();
            var syncResult = await new FilamentCatalogSyncService().FetchAsync(additionalSources);
            if (!syncResult.Ok)
                return Results.BadRequest(new { ok = false, error = syncResult.Error });

            var summary = await filaments.ImportManyAsync(syncResult.Entries);
            await appSettings.RecordFilamentSyncAsync();
            return Results.Ok(new { ok = true, summary.Added, summary.Skipped });
        }).RequireAuthorization();

        app.MapGet("/api/filament-colors", async (FilamentColorService colors) => await colors.ListAsync());
    }
}
