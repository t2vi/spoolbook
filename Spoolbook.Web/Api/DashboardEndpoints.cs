using Spoolbook.Desktop.Features.Dashboard;
using Spoolbook.Desktop.Features.Profiles;

namespace Spoolbook.Web.Api;

public record DashboardSnapshot(DashboardMetrics Metrics, int ProfileCount);

public static class DashboardEndpoints
{
    public static void MapDashboardEndpoints(this WebApplication app) =>
        // Combines DashboardMetricsService + a profile count (Home.razor's own two-call
        // sequence: GetMetricsAsync + ListAsync{Page:1,PageSize:1}.Total) into one round trip.
        app.MapGet("/api/dashboard", async (DashboardMetricsService metricsService, ProfileInventoryService profileInventory) =>
        {
            var metrics = await metricsService.GetMetricsAsync();
            var profiles = await profileInventory.ListAsync(new ProfileInventoryQuery { Page = 1, PageSize = 1 });
            return new DashboardSnapshot(metrics, profiles.Total);
        });
}
