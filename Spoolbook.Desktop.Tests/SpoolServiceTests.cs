using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Features.Prints;
namespace Spoolbook.Desktop.Tests;

public class SpoolServiceTests
{
    private static async Task<int> SeedFilamentAsync(FilamentService filamentService)
    {
        var result = await filamentService.CreateAsync(new FilamentInput
        {
            Brand = "Bambu Lab",
            Material = "PLA",
            Color = "Black"
        });
        return result.Entry!.Id;
    }

    [Fact]
    public async Task CreateSpool_StoresSpoolForFilament()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new SpoolService(db);

        var result = await service.CreateSpoolAsync(filamentId, new SpoolInput
        {
            LotCode = "A1",
            PurchasedAt = new DateOnly(2026, 1, 1),
            WeightGrams = 1000,
            DiameterMm = 1.75m
        });

        Assert.True(result.Ok);
        Assert.Equal(filamentId, result.Spool!.FilamentId);
        Assert.Equal("A1", result.Spool.LotCode);
        Assert.Equal(new DateOnly(2026, 1, 1), result.Spool.PurchasedAt);
        Assert.Equal(1000, result.Spool.WeightGrams);
        Assert.Equal(1.75m, result.Spool.DiameterMm);
    }

    [Fact]
    public async Task ListAllAsync_ReturnsSpoolsAcrossAllFilaments()
    {
        using var db = TestDbFactory.Create();
        var filamentService = new FilamentService(db);
        var filamentId1 = await SeedFilamentAsync(filamentService);
        var filamentId2Result = await filamentService.CreateAsync(new FilamentInput { Brand = "Prusament", Material = "PETG", Color = "Black" });
        var filamentId2 = filamentId2Result.Entry!.Id;

        var service = new SpoolService(db);
        await service.CreateSpoolAsync(filamentId1, new SpoolInput { LotCode = "A" });
        await service.CreateSpoolAsync(filamentId2, new SpoolInput { LotCode = "B" });

        var spools = await service.ListAllAsync();

        Assert.Equal(2, spools.Count);
    }

    [Fact]
    public async Task ListSpoolsForFilament_ReturnsOnlyThatFilamentsSpools()
    {
        using var db = TestDbFactory.Create();
        var filamentService = new FilamentService(db);
        var filamentId1 = await SeedFilamentAsync(filamentService);
        var filamentId2Result = await filamentService.CreateAsync(new FilamentInput { Brand = "Prusament", Material = "PETG", Color = "Black" });
        var filamentId2 = filamentId2Result.Entry!.Id;

        var service = new SpoolService(db);
        await service.CreateSpoolAsync(filamentId1, new SpoolInput { LotCode = "A" });
        await service.CreateSpoolAsync(filamentId2, new SpoolInput { LotCode = "B" });

        var spools = await service.ListSpoolsForFilamentAsync(filamentId1);

        Assert.Single(spools);
        Assert.Equal("A", spools[0].LotCode);
    }

    [Fact]
    public async Task UpdateSpool_ChangesFields()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new SpoolService(db);
        var created = await service.CreateSpoolAsync(filamentId, new SpoolInput { LotCode = "A" });

        var result = await service.UpdateSpoolAsync(created.Spool!.Id, new SpoolInput
        {
            LotCode = "A",
            OpenedAt = new DateOnly(2026, 2, 1),
            WeightGrams = 500,
            DiameterMm = 2.85m
        });

        Assert.True(result.Ok);
        Assert.Equal(new DateOnly(2026, 2, 1), result.Spool!.OpenedAt);
        Assert.Equal(500, result.Spool.WeightGrams);
        Assert.Equal(2.85m, result.Spool.DiameterMm);
    }

    [Fact]
    public async Task DeleteSpool_RemovesIt()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new SpoolService(db);
        var created = await service.CreateSpoolAsync(filamentId, new SpoolInput { LotCode = "A" });

        var result = await service.DeleteSpoolAsync(created.Spool!.Id);

        Assert.True(result.Ok);
        Assert.Null(await service.GetSpoolAsync(created.Spool.Id));
    }

    [Fact]
    public async Task DeleteSpool_BlockedWhileProfileReferencesIt()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new SpoolService(db);
        var spool = await service.CreateSpoolAsync(filamentId, new SpoolInput { LotCode = "A" });
        var profileService = new PrintProfileService(db);
        await profileService.CreateProfileAsync(filamentId, new ProfileInput { Name = "Tweak", NozzleTempC = "230", SpoolId = spool.Spool!.Id });

        var result = await service.DeleteSpoolAsync(spool.Spool.Id);

        Assert.False(result.Ok);
        Assert.Equal("has_profiles", result.Error);
    }

    [Fact]
    public async Task DeleteSpool_BlockedWhilePrintsExist()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new SpoolService(db);
        var spool = await service.CreateSpoolAsync(filamentId, new SpoolInput { LotCode = "A" });
        var profileService = new PrintProfileService(db);
        var profile = await profileService.CreateProfileAsync(filamentId, new ProfileInput { Name = "Standard", NozzleTempC = "230" });
        db.Prints.Add(new Print
        {
            ProfileId = profile.Profile!.Id,
            SpoolId = spool.Spool!.Id,
            Printer = "Bambu Lab P2S",
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0, DateTimeKind.Utc),
            EndedAt = new DateTime(2026, 1, 1, 10, 0, 0, DateTimeKind.Utc),
            Status = PrintStatus.Success
        });
        await db.SaveChangesAsync();

        var result = await service.DeleteSpoolAsync(spool.Spool.Id);

        Assert.False(result.Ok);
        Assert.Equal("has_prints", result.Error);
    }
}
