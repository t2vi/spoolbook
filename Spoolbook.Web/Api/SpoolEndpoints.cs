using Spoolbook.Desktop.Features.Spools;

namespace Spoolbook.Web.Api;

public record CreateSpoolRequest(
    int FilamentId, string? LotCode, DateOnly? PurchasedAt, DateOnly? OpenedAt,
    DateOnly? EmptiedAt, int? WeightGrams, decimal? DiameterMm, string? Notes)
{
    public SpoolInput ToInput() => new()
    {
        LotCode = LotCode, PurchasedAt = PurchasedAt, OpenedAt = OpenedAt,
        EmptiedAt = EmptiedAt, WeightGrams = WeightGrams, DiameterMm = DiameterMm, Notes = Notes
    };
}

public static class SpoolEndpoints
{
    public static void MapSpoolEndpoints(this WebApplication app)
    {
        var group = app.MapGroup("/api/spools");

        group.MapGet("", async (SpoolService spools) => await spools.ListAllAsync());
        group.MapGet("/{id:int}", async (int id, SpoolService spools) =>
        {
            var spool = await spools.GetSpoolAsync(id);
            return spool is null ? Results.NotFound(new { error = "not_found" }) : Results.Ok(spool);
        });

        group.MapPost("", async (CreateSpoolRequest req, SpoolService spools) =>
        {
            var result = await spools.CreateSpoolAsync(req.FilamentId, req.ToInput());
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        group.MapPut("/{id:int}", async (int id, SpoolInput input, SpoolService spools) =>
        {
            var result = await spools.UpdateSpoolAsync(id, input);
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();

        group.MapDelete("/{id:int}", async (int id, SpoolService spools) =>
        {
            var result = await spools.DeleteSpoolAsync(id);
            return result.Ok ? Results.Ok(result) : Results.BadRequest(result);
        }).RequireAuthorization();
    }
}
