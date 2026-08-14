using Spoolbook.Desktop.Features.Prints;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Web.Services;

namespace Spoolbook.Web.Api;

public record ImportUrlRequest(string Url);
public record ResliceRequest(int ProfileId);
public record LinkVersionRequest(int PreviousVersionProjectId);

public static class ProjectEndpoints
{
    public static void MapProjectEndpoints(this WebApplication app)
    {
        var group = app.MapGroup("/api/projects");

        group.MapGet("", async (ProjectService projects) => await projects.ListAsync());

        group.MapGet("/{id:int}/plates", async (int id, ProjectService projects) =>
        {
            var project = (await projects.ListAsync()).FirstOrDefault(p => p.Id == id);
            return project is null ? Results.NotFound(new { error = "not_found" }) : Results.Ok(ProjectService.ReadPlates(project.FilePath));
        });

        group.MapPost("/upload", async (HttpRequest req, ProjectUploadService uploadService) =>
        {
            if (!req.HasFormContentType) return Results.BadRequest(new { error = "Expected multipart form data." });
            var form = await req.ReadFormAsync();
            var file = form.Files.GetFile("file");
            if (file is null) return Results.BadRequest(new ProjectResult { Ok = false, Error = "No file provided." });

            await using var stream = file.OpenReadStream();
            var result = await uploadService.SaveAndUpsertAsync(stream, file.FileName);
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        group.MapPost("/import-url", async (ImportUrlRequest req, ProjectUploadService uploadService) =>
        {
            var result = await uploadService.SaveFromUrlAsync(req.Url);
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        group.MapPost("/{id:int}/reslice", async (int id, ResliceRequest req, ProjectService projects, PrintProfileService profileService, ReslicingService reslicingService) =>
        {
            var project = (await projects.ListAsync()).FirstOrDefault(p => p.Id == id);
            var profile = await profileService.GetProfileAsync(req.ProfileId);
            if (project is null || profile is null) return Results.NotFound(new { error = "not_found" });

            var result = await reslicingService.ReslicingAsync(project, profile);
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        // Suggests an existing Project a fresh upload might be a new version of — caller still
        // confirms explicitly via /link-version below, this only pre-selects.
        group.MapGet("/version-candidate", async (string? meshHash, string fileName, int? excludeProjectId, ProjectService projects) =>
            await projects.FindVersionCandidateAsync(meshHash, fileName, excludeProjectId));

        group.MapPost("/{id:int}/link-version", async (int id, LinkVersionRequest req, ProjectService projects) =>
        {
            await projects.LinkAsNewVersionAsync(id, req.PreviousVersionProjectId);
            return Results.Ok(new { ok = true });
        }).RequireAuthorization();
    }
}
