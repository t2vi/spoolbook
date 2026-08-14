using Spoolbook.Desktop.Features.BambuImport;
using Spoolbook.Desktop.Features.Profiles;

namespace Spoolbook.Web.Api;

public record ProfileFieldSpecResponse(string Name, List<ProfileFieldTab> Tabs);

public record ProfileInputRequest(
    string Name, Dictionary<string, string> Fields, ProfileSource? Source,
    SlicerType? SourceSlicer, string? RawSettingsJson, int? SpoolId)
{
    public ProfileInput ToInput() => new()
    {
        Name = Name, Fields = Fields, Source = Source, SourceSlicer = SourceSlicer,
        RawSettingsJson = RawSettingsJson, SpoolId = SpoolId
    };
}

public static class ProfileEndpoints
{
    public static void MapProfileEndpoints(this WebApplication app)
    {
        var group = app.MapGroup("/api/profiles");

        group.MapGet("", async (int filamentId, PrintProfileService profiles) =>
            await profiles.ListProfilesForFilamentAsync(filamentId));

        group.MapGet("/inventory", async (PrintProfileService _, ProfileInventoryService inventory) =>
            await inventory.ListAsync(new ProfileInventoryQuery()));

        // Serves both /profiles/new (no profileId — blank tabs) and /profiles/edit/{id} (tabs
        // pre-filled from the existing profile's fields) — one endpoint, same shape ProfileEdit
        // .razor's own OnInitializedAsync builds, so the tabbed form doesn't need two code paths.
        group.MapGet("/field-spec", async (int? profileId, PrintProfileService profiles) =>
        {
            if (profileId is null) return Results.Ok(new ProfileFieldSpecResponse("", ProfileFieldSpec.BuildGroups(null)));

            var existing = await profiles.GetProfileAsync(profileId.Value);
            if (existing is null) return Results.NotFound(new { error = "not_found" });

            var fields = ProfileFieldMapper.ToFieldStrings(existing);
            return Results.Ok(new ProfileFieldSpecResponse(existing.Name, ProfileFieldSpec.BuildGroups(fields)));
        });

        // Mirrors ProfileEdit.razor's OnThreeMfSelectedAsync: the browser only hands over bytes,
        // so save to a temp file for BambuFilamentImportService (which expects a local path),
        // then clean up — same save/import/delete sequence, just server-side instead of Blazor's
        // wwwroot-adjacent temp dir.
        group.MapPost("/import-3mf", async (HttpRequest req, BambuFilamentImportService importService) =>
        {
            if (!req.HasFormContentType) return Results.BadRequest(new { error = "Expected multipart form data." });
            var form = await req.ReadFormAsync();
            var file = form.Files.GetFile("file");
            if (file is null) return Results.BadRequest(new { error = "No file provided." });

            var tempPath = Path.Combine(Path.GetTempPath(), $"spoolbook-upload-{Guid.NewGuid():N}.3mf");
            try
            {
                await using (var stream = File.Create(tempPath))
                    await file.CopyToAsync(stream);

                var result = await importService.ImportFromThreeMfAsync(tempPath);
                return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
            }
            finally
            {
                if (File.Exists(tempPath)) File.Delete(tempPath);
            }
        }).RequireAuthorization();

        group.MapPost("", async (int filamentId, ProfileInputRequest req, PrintProfileService profiles) =>
        {
            var result = await profiles.CreateProfileAsync(filamentId, req.ToInput());
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        group.MapPut("/{id:int}", async (int id, ProfileInputRequest req, PrintProfileService profiles) =>
        {
            var result = await profiles.UpdateProfileAsync(id, req.ToInput());
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        group.MapDelete("/{id:int}", async (int id, PrintProfileService profiles) =>
        {
            var result = await profiles.DeleteProfileAsync(id);
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();
    }
}
