using Spoolbook.Desktop.Data;
using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Settings.Printers;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Features.Prints;
namespace Spoolbook.Desktop.Tests;

public class PrintInventoryServiceTests
{
    private static async Task<(int bambuSpoolId, int prusaSpoolId, int profileId, int printerId)> SeedAsync(SpoolbookDbContext db)
    {
        var filamentService = new FilamentService(db);
        var spoolService = new SpoolService(db);
        var profileService = new PrintProfileService(db);
        var printerService = new PrinterService(db);

        var bambu = await filamentService.CreateAsync(new FilamentInput { Brand = "Bambu Lab", Material = "PLA", Color = "Black" });
        var prusa = await filamentService.CreateAsync(new FilamentInput { Brand = "Test Prusament", Material = "PETG", Color = "Test Galaxy Black" });

        var bambuSpool = await spoolService.CreateSpoolAsync(bambu.Entry!.Id, new SpoolInput());
        var prusaSpool = await spoolService.CreateSpoolAsync(prusa.Entry!.Id, new SpoolInput());

        var profile = await profileService.CreateProfileAsync(bambu.Entry.Id, new ProfileInput { Name = "Standard", NozzleTempC = "230" });
        var printer = await printerService.CreateAsync(new PrinterInput { Name = "P2S" });

        return (bambuSpool.Spool!.Id, prusaSpool.Spool!.Id, profile.Profile!.Id, printer.Printer!.Id);
    }

    [Fact]
    public async Task List_ReturnsAllPrintsByDefault()
    {
        using var db = TestDbFactory.Create();
        var (bambuSpoolId, prusaSpoolId, profileId, printerId) = await SeedAsync(db);
        var printService = new PrintService(db, new FakeWeatherService());
        await printService.CreateAsync(profileId, bambuSpoolId, printerId, new PrintInput { StartedAt = new DateTime(2026, 1, 1), EndedAt = new DateTime(2026, 1, 1, 2, 0, 0), Status = PrintStatus.Success });
        await printService.CreateAsync(profileId, prusaSpoolId, printerId, new PrintInput { StartedAt = new DateTime(2026, 1, 2), EndedAt = new DateTime(2026, 1, 2, 2, 0, 0), Status = PrintStatus.Failed });
        var service = new PrintInventoryService(db);

        var result = await service.ListAsync(new PrintInventoryQuery());

        Assert.Equal(2, result.Total);
        Assert.Equal(2, result.Prints.Count);
    }

    [Fact]
    public async Task List_FiltersByBrand()
    {
        using var db = TestDbFactory.Create();
        var (bambuSpoolId, prusaSpoolId, profileId, printerId) = await SeedAsync(db);
        var printService = new PrintService(db, new FakeWeatherService());
        await printService.CreateAsync(profileId, bambuSpoolId, printerId, new PrintInput { StartedAt = new DateTime(2026, 1, 1), EndedAt = new DateTime(2026, 1, 1, 2, 0, 0), Status = PrintStatus.Success });
        await printService.CreateAsync(profileId, prusaSpoolId, printerId, new PrintInput { StartedAt = new DateTime(2026, 1, 2), EndedAt = new DateTime(2026, 1, 2, 2, 0, 0), Status = PrintStatus.Failed });
        var service = new PrintInventoryService(db);

        var result = await service.ListAsync(new PrintInventoryQuery { Brand = "Test Prusament" });

        Assert.Single(result.Prints);
        Assert.Equal(PrintStatus.Failed, result.Prints[0].Status);
    }

    [Fact]
    public async Task List_FiltersByStatus()
    {
        using var db = TestDbFactory.Create();
        var (bambuSpoolId, prusaSpoolId, profileId, printerId) = await SeedAsync(db);
        var printService = new PrintService(db, new FakeWeatherService());
        await printService.CreateAsync(profileId, bambuSpoolId, printerId, new PrintInput { StartedAt = new DateTime(2026, 1, 1), EndedAt = new DateTime(2026, 1, 1, 2, 0, 0), Status = PrintStatus.Success });
        await printService.CreateAsync(profileId, prusaSpoolId, printerId, new PrintInput { StartedAt = new DateTime(2026, 1, 2), EndedAt = new DateTime(2026, 1, 2, 2, 0, 0), Status = PrintStatus.Failed });
        var service = new PrintInventoryService(db);

        var result = await service.ListAsync(new PrintInventoryQuery { Status = PrintStatus.Failed });

        Assert.Single(result.Prints);
    }

    [Fact]
    public async Task List_SortsByStartedAtDescendingByDefault()
    {
        using var db = TestDbFactory.Create();
        var (bambuSpoolId, _, profileId, printerId) = await SeedAsync(db);
        var printService = new PrintService(db, new FakeWeatherService());
        await printService.CreateAsync(profileId, bambuSpoolId, printerId, new PrintInput { StartedAt = new DateTime(2026, 1, 1), EndedAt = new DateTime(2026, 1, 1, 2, 0, 0), Status = PrintStatus.Success });
        await printService.CreateAsync(profileId, bambuSpoolId, printerId, new PrintInput { StartedAt = new DateTime(2026, 1, 5), EndedAt = new DateTime(2026, 1, 5, 2, 0, 0), Status = PrintStatus.Success });
        var service = new PrintInventoryService(db);

        var result = await service.ListAsync(new PrintInventoryQuery());

        Assert.Equal(new DateTime(2026, 1, 5), result.Prints[0].StartedAt);
    }

    [Fact]
    public async Task List_Paginates()
    {
        using var db = TestDbFactory.Create();
        var (bambuSpoolId, prusaSpoolId, profileId, printerId) = await SeedAsync(db);
        var printService = new PrintService(db, new FakeWeatherService());
        await printService.CreateAsync(profileId, bambuSpoolId, printerId, new PrintInput { StartedAt = new DateTime(2026, 1, 1), EndedAt = new DateTime(2026, 1, 1, 2, 0, 0), Status = PrintStatus.Success });
        await printService.CreateAsync(profileId, prusaSpoolId, printerId, new PrintInput { StartedAt = new DateTime(2026, 1, 2), EndedAt = new DateTime(2026, 1, 2, 2, 0, 0), Status = PrintStatus.Failed });
        var service = new PrintInventoryService(db);

        var page1 = await service.ListAsync(new PrintInventoryQuery { Page = 1, PageSize = 1 });
        var page2 = await service.ListAsync(new PrintInventoryQuery { Page = 2, PageSize = 1 });

        Assert.Single(page1.Prints);
        Assert.Single(page2.Prints);
        Assert.Equal(2, page1.Total);
        Assert.Equal(2, page1.TotalPages);
    }
}
