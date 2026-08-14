using System.Text.Json;
using Microsoft.AspNetCore.Http.Json;
using Microsoft.Extensions.Options;
using Spoolbook.Desktop.Features.Prints;
using Spoolbook.Desktop.Features.Settings.Printers;
using Spoolbook.Desktop.Services.Printer;
using Spoolbook.Web.Services;

namespace Spoolbook.Web.Api;

public record PrinterConnectionTestRequest(string IpAddress, string AccessCode);
public record PrinterControlRequest(string Command);
public record StartPrintRequest(int ProjectId, string PlaterId, int SpoolId, int ProfileId, bool UseAms, int AmsSlot);
public record PrinterLiveSnapshot(bool Connected, List<AmsUnitReading> AmsUnits, string CameraStatus, string? CameraError, string? GcodeState);

// JSON API mirror of the Printers Razor pages + PrinterCard/PrintModal — first slice of the
// Blazor Server -> SvelteKit migration. Thin wrappers over the same services those components
// already call; no new business logic.
public static class PrinterEndpoints
{
    public static void MapPrinterEndpoints(this WebApplication app)
    {
        var group = app.MapGroup("/api/printers");

        group.MapGet("", async (PrinterService printers) => await printers.ListAsync());

        group.MapPost("", async (PrinterInput input, PrinterService printers) =>
        {
            var result = await printers.CreateAsync(input);
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        group.MapPut("/{id:int}", async (int id, PrinterInput input, PrinterService printers) =>
        {
            var result = await printers.UpdateAsync(id, input);
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        group.MapDelete("/{id:int}", async (int id, PrinterService printers) =>
        {
            var result = await printers.DeleteAsync(id);
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        group.MapPost("/test", async (PrinterConnectionTestRequest req, PrinterConnectionTestService test) =>
            await test.TestAsync(req.IpAddress, req.AccessCode)).RequireAuthorization();

        // SSE: direct lift of PrinterCard.razor's poll loop (2s cadence, same two data sources),
        // same streaming shape as the existing /printers/{id}/camera MJPEG loop below — just a
        // new content-type, not a new pattern in this codebase.
        group.MapGet("/{id:int}/live", async (int id, HttpContext ctx, PrinterLiveStatusStore liveStatusStore, PrinterCameraService cameraService, IOptions<JsonOptions> jsonOptions) =>
        {
            ctx.Response.ContentType = "text/event-stream";
            ctx.Response.Headers.CacheControl = "no-cache";

            // Reuse the app's configured (camelCase) JSON options — a bare JsonSerializer.Serialize
            // call here would fall back to PascalCase and disagree with every other JSON endpoint.
            var serializerOptions = jsonOptions.Value.SerializerOptions;

            using var timer = new PeriodicTimer(TimeSpan.FromSeconds(2));
            try
            {
                do
                {
                    var snapshot = new PrinterLiveSnapshot(
                        liveStatusStore.GetConnectedClient(id) is not null,
                        liveStatusStore.GetAmsUnits(id),
                        cameraService.GetStatus(id).ToString(),
                        cameraService.GetLastError(id),
                        liveStatusStore.GetGcodeState(id));

                    await ctx.Response.WriteAsync($"data: {JsonSerializer.Serialize(snapshot, serializerOptions)}\n\n", ctx.RequestAborted);
                    await ctx.Response.Body.FlushAsync(ctx.RequestAborted);
                } while (await timer.WaitForNextTickAsync(ctx.RequestAborted));
            }
            catch (OperationCanceledException)
            {
                // Client navigated away / closed the tab — expected teardown, not an error.
            }
        });

        // Only clears the failed state back to NotStarted (see PrinterCameraService.Retry) — the
        // browser's <img> tag reconnecting (cache-busted query string) is what actually restarts
        // the pipeline, same division of responsibility as PrinterCard.razor's RetryCamera().
        group.MapPost("/{id:int}/camera/retry", (int id, PrinterCameraService cameraService) =>
        {
            cameraService.Retry(id);
            return Results.Ok(new PrinterControlResult { Ok = true });
        });

        group.MapPost("/{id:int}/control", async (int id, PrinterControlRequest req, PrinterService printers, PrinterControlService control) =>
        {
            var printer = (await printers.ListAsync()).FirstOrDefault(p => p.Id == id);
            if (printer?.SerialNumber is null) return Results.NotFound(new { error = "not_found" });

            var result = req.Command switch
            {
                "pause" => await control.PauseAsync(id, printer.SerialNumber),
                "resume" => await control.ResumeAsync(id, printer.SerialNumber),
                "stop" => await control.StopAsync(id, printer.SerialNumber),
                _ => new PrinterControlResult { Ok = false, Error = "Unknown command." }
            };
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        // Combines PrinterPrintService.StartPrintAsync + PrintService.CreateInProgressAsync —
        // the same two-call sequence PrintModal.razor's SendAsync() does today — into one round
        // trip for a decoupled client.
        group.MapPost("/{id:int}/print", async (
            int id, StartPrintRequest req, PrinterService printers, ProjectService projects,
            PrinterPrintService printerPrintService, PrintService printRecordService) =>
        {
            var printer = (await printers.ListAsync()).FirstOrDefault(p => p.Id == id);
            if (printer?.IpAddress is null || printer.AccessCode is null || printer.SerialNumber is null)
                return Results.BadRequest(new PrinterControlResult { Ok = false, Error = "Printer is missing connection details." });

            var project = (await projects.ListAsync()).FirstOrDefault(p => p.Id == req.ProjectId);
            if (project is null) return Results.NotFound(new { error = "not_found" });

            var startResult = await printerPrintService.StartPrintAsync(
                id, printer.IpAddress, printer.AccessCode, printer.SerialNumber,
                project.FilePath, project.FileName, $"plate_{req.PlaterId}.gcode",
                req.UseAms, req.AmsSlot, printer.Model);

            if (!startResult.Ok) return Results.BadRequest(startResult);

            await printRecordService.CreateInProgressAsync(req.ProfileId, req.SpoolId, id, project.Id, req.PlaterId, DateTime.UtcNow);
            return Results.Ok(new PrinterControlResult { Ok = true });
        }).RequireAuthorization();
    }
}
