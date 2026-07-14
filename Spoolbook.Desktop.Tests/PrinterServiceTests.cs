using Spoolbook.Desktop.Data;
using Spoolbook.Desktop.Features.Settings.Printers;
namespace Spoolbook.Desktop.Tests;

public class PrinterServiceTests
{
    [Fact]
    public async Task CreateAsync_AddsNewPrinter()
    {
        using var db = TestDbFactory.Create();
        var service = new PrinterService(db);

        var result = await service.CreateAsync(new PrinterInput { Name = "Garage P2S", Model = "Bambu Lab P2S" });

        Assert.True(result.Ok);
        Assert.Equal("Garage P2S", result.Printer!.Name);
        Assert.Equal("Bambu Lab P2S", result.Printer.Model);
    }

    [Fact]
    public async Task CreateAsync_ModelIsOptional()
    {
        using var db = TestDbFactory.Create();
        var service = new PrinterService(db);

        var result = await service.CreateAsync(new PrinterInput { Name = "Garage P2S" });

        Assert.True(result.Ok);
        Assert.Null(result.Printer!.Model);
    }

    [Fact]
    public async Task CreateAsync_RejectsDuplicateName()
    {
        using var db = TestDbFactory.Create();
        var service = new PrinterService(db);
        await service.CreateAsync(new PrinterInput { Name = "Garage P2S" });

        var result = await service.CreateAsync(new PrinterInput { Name = "Garage P2S", Model = "Different model" });

        Assert.False(result.Ok);
        Assert.Equal("duplicate", result.Error);
    }

    [Fact]
    public async Task ListAsync_ReturnsSortedByName()
    {
        using var db = TestDbFactory.Create();
        var service = new PrinterService(db);
        await service.CreateAsync(new PrinterInput { Name = "Zeta Printer" });
        await service.CreateAsync(new PrinterInput { Name = "Alpha Printer" });

        var printers = await service.ListAsync();

        Assert.Equal("Alpha Printer", printers[0].Name);
        Assert.Equal("Zeta Printer", printers[1].Name);
    }

    [Fact]
    public async Task UpdateAsync_ChangesNameAndModel()
    {
        using var db = TestDbFactory.Create();
        var service = new PrinterService(db);
        var created = await service.CreateAsync(new PrinterInput { Name = "Garage P2S" });

        var result = await service.UpdateAsync(created.Printer!.Id, new PrinterInput { Name = "Basement P2S", Model = "Bambu Lab P2S" });

        Assert.True(result.Ok);
        Assert.Equal("Basement P2S", result.Printer!.Name);
        Assert.Equal("Bambu Lab P2S", result.Printer.Model);
    }

    [Fact]
    public async Task UpdateAsync_RejectsDuplicateName()
    {
        using var db = TestDbFactory.Create();
        var service = new PrinterService(db);
        await service.CreateAsync(new PrinterInput { Name = "Garage P2S" });
        var second = await service.CreateAsync(new PrinterInput { Name = "Basement P2S" });

        var result = await service.UpdateAsync(second.Printer!.Id, new PrinterInput { Name = "Garage P2S" });

        Assert.False(result.Ok);
        Assert.Equal("duplicate", result.Error);
    }

    [Fact]
    public async Task DeleteAsync_RemovesPrinter()
    {
        using var db = TestDbFactory.Create();
        var service = new PrinterService(db);
        var created = await service.CreateAsync(new PrinterInput { Name = "Garage P2S" });

        var result = await service.DeleteAsync(created.Printer!.Id);

        Assert.True(result.Ok);
        Assert.Empty(await service.ListAsync());
    }

    [Fact]
    public async Task DeleteAsync_BlockedWhilePrintsExist()
    {
        using var db = TestDbFactory.Create();
        var service = new PrinterService(db);
        var printer = await service.CreateAsync(new PrinterInput { Name = "Garage P2S" });

        var filamentService = new Spoolbook.Desktop.Features.Settings.Filaments.FilamentService(db);
        var filament = await filamentService.CreateAsync(new Spoolbook.Desktop.Features.Settings.Filaments.FilamentInput { Brand = "Test Brand", Material = "PLA", Color = "Black" });
        var spoolService = new Spoolbook.Desktop.Features.Spools.SpoolService(db);
        var spool = await spoolService.CreateSpoolAsync(filament.Entry!.Id, new Spoolbook.Desktop.Features.Spools.SpoolInput());
        var profileService = new Spoolbook.Desktop.Features.Profiles.PrintProfileService(db);
        var profile = await profileService.CreateProfileAsync(filament.Entry.Id, new Spoolbook.Desktop.Features.Profiles.ProfileInput { Name = "Standard", NozzleTempC = "230" });
        var printService = new Spoolbook.Desktop.Features.Prints.PrintService(db, new FakeWeatherService());
        await printService.CreateAsync(profile.Profile!.Id, spool.Spool!.Id, printer.Printer!.Id, new Spoolbook.Desktop.Features.Prints.PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0),
            EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = Spoolbook.Desktop.Features.Prints.PrintStatus.Success
        });

        var result = await service.DeleteAsync(printer.Printer.Id);

        Assert.False(result.Ok);
        Assert.Equal("has_prints", result.Error);
    }
}
