using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;
using Spoolbook.Desktop.Features.Prints;
using Spoolbook.Desktop.Features.Settings.General;
namespace Spoolbook.Desktop.Features.Dashboard;

public record CategoryCount(string Label, int Count);

public class DashboardMetrics
{
    public int FilamentCount { get; init; }
    public DateTime? LastFilamentSyncAt { get; init; }
    public List<CategoryCount> FilamentsByBrand { get; init; } = [];
    public List<CategoryCount> FilamentsByMaterial { get; init; } = [];
    public List<CategoryCount> SpoolsByStatus { get; init; } = [];
    public List<CategoryCount> PrintsByStatus { get; init; } = [];
}

public class DashboardMetricsService
{
    private readonly SpoolbookDbContext _db;
    private readonly AppSettingsService _appSettingsService;

    public DashboardMetricsService(SpoolbookDbContext db, AppSettingsService appSettingsService)
    {
        _db = db;
        _appSettingsService = appSettingsService;
    }

    public async Task<DashboardMetrics> GetMetricsAsync()
    {
        var filamentCount = await _db.Filaments.CountAsync();

        var byBrand = (await _db.Filaments
            .GroupBy(f => f.Brand)
            .Select(g => new { Label = g.Key, Count = g.Count() })
            .ToListAsync())
            .OrderByDescending(c => c.Count)
            .Select(c => new CategoryCount(c.Label, c.Count))
            .ToList();

        var byMaterial = (await _db.Filaments
            .GroupBy(f => f.Material)
            .Select(g => new { Label = g.Key, Count = g.Count() })
            .ToListAsync())
            .OrderByDescending(c => c.Count)
            .Select(c => new CategoryCount(c.Label, c.Count))
            .ToList();

        var spools = await _db.Spools.Select(s => new { s.OpenedAt, s.EmptiedAt }).ToListAsync();
        var spoolsByStatus = new List<CategoryCount>
        {
            new("Unopened", spools.Count(s => s.OpenedAt is null)),
            new("Opened", spools.Count(s => s.OpenedAt is not null && s.EmptiedAt is null)),
            new("Empty", spools.Count(s => s.EmptiedAt is not null))
        };

        var printCountsByStatus = await _db.Prints
            .GroupBy(p => p.Status)
            .Select(g => new { Status = g.Key, Count = g.Count() })
            .ToListAsync();
        var printsByStatus = Enum.GetValues<PrintStatus>()
            .Select(status => new CategoryCount(status.ToString(), printCountsByStatus.FirstOrDefault(p => p.Status == status)?.Count ?? 0))
            .ToList();

        var appSettings = await _appSettingsService.GetAsync();

        return new DashboardMetrics
        {
            FilamentCount = filamentCount,
            LastFilamentSyncAt = appSettings.LastFilamentSyncAt,
            FilamentsByBrand = byBrand,
            FilamentsByMaterial = byMaterial,
            SpoolsByStatus = spoolsByStatus,
            PrintsByStatus = printsByStatus
        };
    }
}
