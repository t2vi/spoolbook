using Spoolbook.Desktop.Features.Prints;

namespace Spoolbook.Web.Api;

public record PrintInputRequest(
    DateTime StartedAt, DateTime EndedAt, PrintStatus Status, string? Notes, int? AmsHumidityPct,
    decimal? ActualRoomTempC, bool? CleanBuildPlate, int? ProjectId, string? ProjectPlaterId,
    List<FailureMode> FailureModes)
{
    public PrintInput ToInput() => new()
    {
        StartedAt = StartedAt, EndedAt = EndedAt, Status = Status, Notes = Notes,
        AmsHumidityPct = AmsHumidityPct, ActualRoomTempC = ActualRoomTempC, CleanBuildPlate = CleanBuildPlate,
        ProjectId = ProjectId, ProjectPlaterId = ProjectPlaterId, FailureModes = FailureModes
    };
}

public record CreatePrintRequest(int ProfileId, int SpoolId, int PrinterId, PrintInputRequest Input);
public record UpdatePrintRequest(int PrinterId, PrintInputRequest Input);
public record AttachJobRequest(int JobId);

public static class PrintEndpoints
{
    public static void MapPrintEndpoints(this WebApplication app)
    {
        var group = app.MapGroup("/api/prints");

        // PrintService.ListAsync() has no filter args (confirmed by reading the file) — Razor
        // filters client-side today for the "recent prints" panel; do the same here rather than
        // adding a query-object convention to the service for 5 rows. Full paginated search
        // (the Prints list page) is the separate /inventory endpoint below.
        group.MapGet("", async (int? printerId, PrintService prints) =>
        {
            var all = await prints.ListAsync();
            return printerId is null ? all : all.Where(p => p.PrinterId == printerId).Take(5).ToList();
        });

        group.MapGet("/inventory", async (PrintStatus? status, int? printerId, int page, int pageSize, PrintInventoryService inventory) =>
            await inventory.ListAsync(new PrintInventoryQuery
            {
                Status = status,
                PrinterId = printerId,
                Page = page == 0 ? 1 : page,
                PageSize = pageSize == 0 ? 20 : pageSize
            }));

        group.MapGet("/recommend-profile", async (int projectId, decimal? currentTempC, PrintService prints) =>
            await prints.RecommendProfileForProjectAsync(projectId, currentTempC));

        // Only meaningful when logging a NEW print (Id absent in the Blazor form) — an existing
        // Print's Job was already decided at creation time, matching RefreshJobMatchAsync's own
        // Id.HasValue guard.
        group.MapGet("/job-match", async (int printerId, DateTime startedAt, PrinterTelemetryService telemetry) =>
            await telemetry.FindMatchForPrintAsync(printerId, startedAt));

        group.MapGet("/{id:int}", async (int id, PrintService prints) =>
        {
            var print = await prints.GetAsync(id);
            return print is null ? Results.NotFound(new { error = "not_found" }) : Results.Ok(print);
        });

        group.MapPost("", async (CreatePrintRequest req, PrintService prints) =>
        {
            var result = await prints.CreateAsync(req.ProfileId, req.SpoolId, req.PrinterId, req.Input.ToInput());
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        group.MapPut("/{id:int}", async (int id, UpdatePrintRequest req, PrintService prints) =>
        {
            var result = await prints.UpdateAsync(id, req.PrinterId, req.Input.ToInput());
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        group.MapDelete("/{id:int}", async (int id, PrintService prints) =>
        {
            var result = await prints.DeleteAsync(id);
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        group.MapPost("/{id:int}/attach-job", async (int id, AttachJobRequest req, PrinterTelemetryService telemetry) =>
        {
            await telemetry.AttachJobToPrintAsync(req.JobId, id);
            return Results.Ok(new { ok = true });
        }).RequireAuthorization();
    }
}
