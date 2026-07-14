using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;
using Spoolbook.Desktop.Features.Dashboard;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Settings.General;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Features.Prints;
namespace Spoolbook.Desktop.Tests;

public class DashboardMetricsServiceTests
{
    [Fact]
    public async Task GetMetricsAsync_CountsFilaments()
    {
        using var db = TestDbFactory.Create();
        var seededCount = await db.Filaments.CountAsync();
        var filamentService = new FilamentService(db);
        await filamentService.CreateAsync(new FilamentInput { Brand = "Test Brand", Material = "PLA", Color = "Black" });
        var service = new DashboardMetricsService(db, new AppSettingsService(db));

        var metrics = await service.GetMetricsAsync();

        Assert.Equal(seededCount + 1, metrics.FilamentCount);
    }

    [Fact]
    public async Task GetMetricsAsync_ReturnsNullLastSyncWhenNeverSynced()
    {
        using var db = TestDbFactory.Create();
        var service = new DashboardMetricsService(db, new AppSettingsService(db));

        var metrics = await service.GetMetricsAsync();

        Assert.Null(metrics.LastFilamentSyncAt);
    }

    [Fact]
    public async Task GetMetricsAsync_ReflectsRecordedSyncTime()
    {
        using var db = TestDbFactory.Create();
        var appSettingsService = new AppSettingsService(db);
        await appSettingsService.RecordFilamentSyncAsync();
        var service = new DashboardMetricsService(db, appSettingsService);

        var metrics = await service.GetMetricsAsync();

        Assert.NotNull(metrics.LastFilamentSyncAt);
    }

    [Fact]
    public async Task GetMetricsAsync_GroupsFilamentsByBrandDescending()
    {
        using var db = TestDbFactory.Create();
        var filamentService = new FilamentService(db);
        await filamentService.CreateAsync(new FilamentInput { Brand = "Solo Brand", Material = "PLA", Color = "Test Color A" });
        await filamentService.CreateAsync(new FilamentInput { Brand = "Solo Brand", Material = "PETG", Color = "Test Color B" });
        var service = new DashboardMetricsService(db, new AppSettingsService(db));

        var metrics = await service.GetMetricsAsync();

        var soloBrand = metrics.FilamentsByBrand.Single(c => c.Label == "Solo Brand");
        Assert.Equal(2, soloBrand.Count);
        Assert.True(metrics.FilamentsByBrand[0].Count >= metrics.FilamentsByBrand[^1].Count);
    }

    [Fact]
    public async Task GetMetricsAsync_BreaksDownSpoolsByStatus()
    {
        using var db = TestDbFactory.Create();
        var filamentService = new FilamentService(db);
        var spoolService = new SpoolService(db);
        var filament = await filamentService.CreateAsync(new FilamentInput { Brand = "Test Brand", Material = "PLA", Color = "Black" });
        await spoolService.CreateSpoolAsync(filament.Entry!.Id, new SpoolInput());
        await spoolService.CreateSpoolAsync(filament.Entry.Id, new SpoolInput { OpenedAt = new DateOnly(2026, 1, 1) });
        await spoolService.CreateSpoolAsync(filament.Entry.Id, new SpoolInput { OpenedAt = new DateOnly(2026, 1, 1), EmptiedAt = new DateOnly(2026, 1, 2) });
        var service = new DashboardMetricsService(db, new AppSettingsService(db));

        var metrics = await service.GetMetricsAsync();

        Assert.Equal(1, metrics.SpoolsByStatus.Single(c => c.Label == "Unopened").Count);
        Assert.Equal(1, metrics.SpoolsByStatus.Single(c => c.Label == "Opened").Count);
        Assert.Equal(1, metrics.SpoolsByStatus.Single(c => c.Label == "Empty").Count);
    }

    [Fact]
    public async Task GetMetricsAsync_BreaksDownPrintsByStatusIncludingZeroCounts()
    {
        using var db = TestDbFactory.Create();
        var filamentService = new FilamentService(db);
        var spoolService = new SpoolService(db);
        var profileService = new PrintProfileService(db);
        var filament = await filamentService.CreateAsync(new FilamentInput { Brand = "Test Brand", Material = "PLA", Color = "Black" });
        var spool = await spoolService.CreateSpoolAsync(filament.Entry!.Id, new SpoolInput());
        var profile = await profileService.CreateProfileAsync(filament.Entry.Id, new ProfileInput { Name = "Standard", NozzleTempC = "230" });
        var printService = new PrintService(db, new FakeWeatherService());
        await printService.CreateAsync(profile.Profile!.Id, spool.Spool!.Id, new PrintInput
        {
            Printer = "P2S", StartedAt = new DateTime(2026, 1, 1), EndedAt = new DateTime(2026, 1, 1, 2, 0, 0), Status = PrintStatus.Success
        });
        var service = new DashboardMetricsService(db, new AppSettingsService(db));

        var metrics = await service.GetMetricsAsync();

        Assert.Equal(1, metrics.PrintsByStatus.Single(c => c.Label == "Success").Count);
        Assert.Equal(0, metrics.PrintsByStatus.Single(c => c.Label == "Failed").Count);
        Assert.Equal(0, metrics.PrintsByStatus.Single(c => c.Label == "Partial").Count);
    }
}
