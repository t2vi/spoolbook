using Spoolbook.Desktop.Data;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Settings.Printers;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Features.Prints;
namespace Spoolbook.Desktop.Tests;

public class FakeWeatherService : IWeatherService
{
    public (decimal? TempC, decimal? HumidityPct) Result { get; set; } = (22.5m, 60m);

    public Task<(decimal? TempC, decimal? HumidityPct)> GetAmbientAsync(DateTime startedAt, DateTime endedAt) =>
        Task.FromResult(Result);
}

public class PrintServiceTests
{
    private static async Task<(int ProfileId, int SpoolId, int PrinterId)> SeedAsync(SpoolbookDbContext db)
    {
        var filamentService = new FilamentService(db);
        var filament = await filamentService.CreateAsync(new FilamentInput { Brand = "Bambu Lab", Material = "PLA", Color = "Black" });
        var spoolService = new SpoolService(db);
        var spool = await spoolService.CreateSpoolAsync(filament.Entry!.Id, new SpoolInput());
        var profileService = new PrintProfileService(db);
        var profile = await profileService.CreateProfileAsync(filament.Entry.Id, new ProfileInput { Name = "Standard", NozzleTempC = "230" });
        var printerService = new PrinterService(db);
        var printer = await printerService.CreateAsync(new PrinterInput { Name = "Bambu Lab P2S" });
        return (profile.Profile!.Id, spool.Spool!.Id, printer.Printer!.Id);
    }

    [Fact]
    public async Task CreateAsync_StoresPrintReferencingProfileAndSpool()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var service = new PrintService(db, new FakeWeatherService());

        var result = await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0),
            EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Success
        });

        Assert.True(result.Ok);
        Assert.Equal(profileId, result.Print!.ProfileId);
        Assert.Equal(spoolId, result.Print.SpoolId);
        Assert.Equal(printerId, result.Print.PrinterId);
        Assert.Equal(PrintStatus.Success, result.Print.Status);
    }

    [Fact]
    public async Task CreateAsync_FetchesAmbientWeatherAndSetsSource()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var weather = new FakeWeatherService { Result = (18.4m, 55m) };
        var service = new PrintService(db, weather);

        var result = await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0),
            EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Success
        });

        Assert.Equal(18.4m, result.Print!.AmbientTempC);
        Assert.Equal(55m, result.Print.AmbientHumidityPct);
        Assert.Equal(AmbientSource.WeatherApi, result.Print.AmbientSource);
    }

    [Fact]
    public async Task CreateAsync_WeatherFetchFails_LeavesAmbientNull()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var weather = new FakeWeatherService { Result = (null, null) };
        var service = new PrintService(db, weather);

        var result = await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0),
            EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Success
        });

        Assert.Null(result.Print!.AmbientTempC);
        Assert.Null(result.Print.AmbientHumidityPct);
        Assert.Null(result.Print.AmbientSource);
    }

    [Fact]
    public async Task CreateAsync_StoresAmsHumidityAndNotes()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var service = new PrintService(db, new FakeWeatherService());

        var result = await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0),
            EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Partial,
            Notes = "Warped corner",
            AmsHumidityPct = 12,
            ActualRoomTempC = 19.5m,
            CleanBuildPlate = false
        });

        Assert.Equal("Warped corner", result.Print!.Notes);
        Assert.Equal(12, result.Print.AmsHumidityPct);
        Assert.Equal(PrintStatus.Partial, result.Print.Status);
        Assert.Equal(19.5m, result.Print.ActualRoomTempC);
        Assert.False(result.Print.CleanBuildPlate);
    }

    [Fact]
    public async Task ListAsync_ReturnsPrintsNewestFirst()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var service = new PrintService(db, new FakeWeatherService());
        await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0), Status = PrintStatus.Success
        });
        await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 2, 8, 0, 0), EndedAt = new DateTime(2026, 1, 2, 10, 0, 0), Status = PrintStatus.Failed
        });

        var prints = await service.ListAsync();

        Assert.Equal(2, prints.Count);
        Assert.Equal(PrintStatus.Failed, prints[0].Status);
    }

    [Fact]
    public async Task GetAsync_IncludesProfileAndSpoolWithFilament()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var service = new PrintService(db, new FakeWeatherService());
        var created = await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0), Status = PrintStatus.Success
        });

        var loaded = await service.GetAsync(created.Print!.Id);

        Assert.NotNull(loaded!.Profile);
        Assert.NotNull(loaded.Spool);
        Assert.NotNull(loaded.Spool!.Filament);
        Assert.NotNull(loaded.Printer);
    }

    [Fact]
    public async Task DeleteAsync_RemovesPrint()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var service = new PrintService(db, new FakeWeatherService());
        var created = await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0), Status = PrintStatus.Success
        });

        var result = await service.DeleteAsync(created.Print!.Id);

        Assert.True(result.Ok);
        Assert.Empty(await service.ListAsync());
    }
}
