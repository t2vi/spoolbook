using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Spools;
namespace Spoolbook.Desktop.Tests;

public class SpoolInventoryServiceTests
{
    private async Task<(int bambuId, int prusaId)> SeedAsync(FilamentService filamentService, SpoolService spoolService)
    {
        var bambu = await filamentService.CreateAsync(new FilamentInput { Brand = "Bambu Lab", Material = "PLA", Color = "Black" });
        var prusa = await filamentService.CreateAsync(new FilamentInput { Brand = "Test Prusament", Material = "PETG", Color = "Test Galaxy Black" });

        await spoolService.CreateSpoolAsync(bambu.Entry!.Id, new SpoolInput { LotCode = "A", PurchasedAt = new DateOnly(2026, 1, 1) });
        await spoolService.CreateSpoolAsync(bambu.Entry!.Id, new SpoolInput { LotCode = "B", PurchasedAt = new DateOnly(2026, 2, 1), OpenedAt = new DateOnly(2026, 2, 5) });
        await spoolService.CreateSpoolAsync(prusa.Entry!.Id, new SpoolInput { LotCode = "C", PurchasedAt = new DateOnly(2026, 3, 1), OpenedAt = new DateOnly(2026, 3, 5), EmptiedAt = new DateOnly(2026, 3, 10) });

        return (bambu.Entry.Id, prusa.Entry.Id);
    }

    [Fact]
    public async Task List_ReturnsAllSpoolsAcrossFilamentsByDefault()
    {
        using var db = TestDbFactory.Create();
        await SeedAsync(new FilamentService(db), new SpoolService(db));
        var service = new SpoolInventoryService(db);

        var result = await service.ListAsync(new SpoolInventoryQuery());

        Assert.Equal(3, result.Total);
        Assert.Equal(3, result.Spools.Count);
    }

    [Fact]
    public async Task List_FiltersByBrand()
    {
        using var db = TestDbFactory.Create();
        await SeedAsync(new FilamentService(db), new SpoolService(db));
        var service = new SpoolInventoryService(db);

        var result = await service.ListAsync(new SpoolInventoryQuery { Brand = "Test Prusament" });

        Assert.Single(result.Spools);
        Assert.Equal("C", result.Spools[0].LotCode);
    }

    [Fact]
    public async Task List_FiltersByStatus()
    {
        using var db = TestDbFactory.Create();
        await SeedAsync(new FilamentService(db), new SpoolService(db));
        var service = new SpoolInventoryService(db);

        var unopened = await service.ListAsync(new SpoolInventoryQuery { Status = SpoolStatus.Unopened });
        var opened = await service.ListAsync(new SpoolInventoryQuery { Status = SpoolStatus.Opened });
        var empty = await service.ListAsync(new SpoolInventoryQuery { Status = SpoolStatus.Empty });

        Assert.Equal(new[] { "A" }, unopened.Spools.Select(s => s.LotCode));
        Assert.Equal(new[] { "B" }, opened.Spools.Select(s => s.LotCode));
        Assert.Equal(new[] { "C" }, empty.Spools.Select(s => s.LotCode));
    }

    [Fact]
    public async Task List_SortsByPurchasedAtWithNullsLastRegardlessOfDirection()
    {
        using var db = TestDbFactory.Create();
        var filamentService = new FilamentService(db);
        var spoolService = new SpoolService(db);
        var filament = await filamentService.CreateAsync(new FilamentInput { Brand = "B", Material = "M", Color = "C" });
        await spoolService.CreateSpoolAsync(filament.Entry!.Id, new SpoolInput { LotCode = "no-date" });
        await spoolService.CreateSpoolAsync(filament.Entry!.Id, new SpoolInput { LotCode = "dated", PurchasedAt = new DateOnly(2026, 1, 1) });
        var service = new SpoolInventoryService(db);

        var asc = await service.ListAsync(new SpoolInventoryQuery { Sort = SpoolSortColumn.PurchasedAt, Order = SortOrder.Asc });
        var desc = await service.ListAsync(new SpoolInventoryQuery { Sort = SpoolSortColumn.PurchasedAt, Order = SortOrder.Desc });

        Assert.Equal("no-date", asc.Spools.Last().LotCode);
        Assert.Equal("no-date", desc.Spools.Last().LotCode);
    }

    [Fact]
    public async Task List_Paginates()
    {
        using var db = TestDbFactory.Create();
        await SeedAsync(new FilamentService(db), new SpoolService(db));
        var service = new SpoolInventoryService(db);

        var page1 = await service.ListAsync(new SpoolInventoryQuery { Page = 1, PageSize = 2 });
        var page2 = await service.ListAsync(new SpoolInventoryQuery { Page = 2, PageSize = 2 });

        Assert.Equal(2, page1.Spools.Count);
        Assert.Single(page2.Spools);
        Assert.Equal(3, page1.Total);
        Assert.Equal(2, page1.TotalPages);
    }
}
