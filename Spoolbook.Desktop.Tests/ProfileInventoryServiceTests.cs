using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Features.Profiles;
namespace Spoolbook.Desktop.Tests;

public class ProfileInventoryServiceTests
{
    private async Task Seed(FilamentService filamentService, SpoolService spoolService, PrintProfileService profileService)
    {
        var bambu = await filamentService.CreateAsync(new FilamentInput { Brand = "Bambu Lab", Material = "PLA", Color = "Black" });
        var prusa = await filamentService.CreateAsync(new FilamentInput { Brand = "Test Prusament", Material = "PETG", Color = "Test Galaxy Black" });
        var spool = await spoolService.CreateSpoolAsync(bambu.Entry!.Id, new SpoolInput { LotCode = "A" });

        await profileService.CreateProfileAsync(bambu.Entry!.Id, new ProfileInput { Name = "generic-profile", NozzleTempC = "220" });
        await profileService.CreateProfileAsync(bambu.Entry.Id, new ProfileInput { Name = "spool-profile", NozzleTempC = "225", SpoolId = spool.Spool!.Id });
        await profileService.CreateProfileAsync(prusa.Entry!.Id, new ProfileInput { Name = "petg-profile", NozzleTempC = "240" });
    }

    [Fact]
    public async Task List_ReturnsAllProfilesAcrossFilamentsByDefault()
    {
        using var db = TestDbFactory.Create();
        await Seed(new FilamentService(db), new SpoolService(db), new PrintProfileService(db));
        var service = new ProfileInventoryService(db);

        var result = await service.ListAsync(new ProfileInventoryQuery());

        Assert.Equal(3, result.Total);
    }

    [Fact]
    public async Task List_FiltersByMaterial()
    {
        using var db = TestDbFactory.Create();
        await Seed(new FilamentService(db), new SpoolService(db), new PrintProfileService(db));
        var service = new ProfileInventoryService(db);

        var result = await service.ListAsync(new ProfileInventoryQuery { Material = "PLA" });

        Assert.Equal(new[] { "generic-profile", "spool-profile" }, result.Profiles.Select(p => p.Name).OrderBy(n => n));
    }

    [Fact]
    public async Task List_FiltersByScope()
    {
        using var db = TestDbFactory.Create();
        await Seed(new FilamentService(db), new SpoolService(db), new PrintProfileService(db));
        var service = new ProfileInventoryService(db);

        var generic = await service.ListAsync(new ProfileInventoryQuery { Scope = ProfileScope.Generic });
        var spoolScoped = await service.ListAsync(new ProfileInventoryQuery { Scope = ProfileScope.Spool });

        Assert.Equal(new[] { "generic-profile", "petg-profile" }, generic.Profiles.Select(p => p.Name).OrderBy(n => n));
        Assert.Equal(new[] { "spool-profile" }, spoolScoped.Profiles.Select(p => p.Name));
    }

    [Fact]
    public async Task List_SortsByNozzleTempDescending()
    {
        using var db = TestDbFactory.Create();
        await Seed(new FilamentService(db), new SpoolService(db), new PrintProfileService(db));
        var service = new ProfileInventoryService(db);

        var result = await service.ListAsync(new ProfileInventoryQuery { Sort = ProfileSortColumn.NozzleTempC, Order = SortOrder.Desc });

        Assert.Equal("petg-profile", result.Profiles[0].Name);
    }

    [Fact]
    public async Task List_OnlyReturnsCurrentVersions()
    {
        using var db = TestDbFactory.Create();
        var filamentService = new FilamentService(db);
        var profileService = new PrintProfileService(db);
        await Seed(filamentService, new SpoolService(db), profileService);
        var bambu = await filamentService.CreateAsync(new FilamentInput { Brand = "Bambu Lab", Material = "PLA", Color = "Yellow" });
        const string path = "/fake/Bambu PLA Basic - FRC - Yellow.json";
        await profileService.CreateProfileAsync(bambu.Entry!.Id, new ProfileInput
        {
            Name = "Yellow", NozzleTempC = "210", SourcePresetPath = path, VersionNumber = 1
        });
        await profileService.CreateProfileAsync(bambu.Entry.Id, new ProfileInput
        {
            Name = "Yellow", NozzleTempC = "215", SourcePresetPath = path, VersionNumber = 2
        });
        var service = new ProfileInventoryService(db);

        var result = await service.ListAsync(new ProfileInventoryQuery());

        Assert.Equal(4, result.Total);
        Assert.Single(result.Profiles, p => p.Name == "Yellow");
    }

    [Fact]
    public async Task List_Paginates()
    {
        using var db = TestDbFactory.Create();
        await Seed(new FilamentService(db), new SpoolService(db), new PrintProfileService(db));
        var service = new ProfileInventoryService(db);

        var page1 = await service.ListAsync(new ProfileInventoryQuery { Page = 1, PageSize = 2 });
        var page2 = await service.ListAsync(new ProfileInventoryQuery { Page = 2, PageSize = 2 });

        Assert.Equal(2, page1.Profiles.Count);
        Assert.Single(page2.Profiles);
        Assert.Equal(2, page1.TotalPages);
    }
}
