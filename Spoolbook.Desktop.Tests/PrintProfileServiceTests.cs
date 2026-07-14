using Spoolbook.Desktop.Data;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Features.Prints;
namespace Spoolbook.Desktop.Tests;

public class PrintProfileServiceTests
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
    public async Task CreateProfile_StoresBaseFieldsAndDynamicFields()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new PrintProfileService(db);

        var result = await service.CreateProfileAsync(filamentId, new ProfileInput
        {
            Name = "Standard",
            NozzleTempC = "230",
            Fields =
            {
                ["HotPlateTempC"] = "65",
                ["RetractionMm"] = "0.8",
                ["Soluble"] = "false"
            }
        });

        Assert.True(result.Ok);
        Assert.Equal("Standard", result.Profile!.Name);
        Assert.Equal(230, result.Profile.NozzleTempC);
        Assert.Equal(65, result.Profile.HotPlateTempC);
        Assert.Equal(0.8m, result.Profile.RetractionMm);
        Assert.False(result.Profile.Soluble);
    }

    [Fact]
    public async Task CreateProfile_LeavesUnsetDynamicFieldsNull()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new PrintProfileService(db);

        var result = await service.CreateProfileAsync(filamentId, new ProfileInput
        {
            Name = "Bare",
            NozzleTempC = "230"
        });

        Assert.True(result.Ok);
        Assert.Null(result.Profile!.HotPlateTempC);
        Assert.Null(result.Profile.RetractionMm);
        Assert.Null(result.Profile.Soluble);
    }

    [Fact]
    public async Task CreateProfile_RejectsMissingNameOrNozzleTemp()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new PrintProfileService(db);

        var result = await service.CreateProfileAsync(filamentId, new ProfileInput { Name = "", NozzleTempC = "" });

        Assert.False(result.Ok);
        Assert.NotNull(result.Errors);
    }

    [Fact]
    public async Task UpdateProfile_PreservesSourceAndRawSettingsWhenNotTouched()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new PrintProfileService(db);

        var created = await service.CreateProfileAsync(filamentId, new ProfileInput
        {
            Name = "Imported",
            NozzleTempC = "230",
            Source = ProfileSource.SlicerImport,
            SourceSlicer = SlicerType.BambuStudio,
            RawSettingsJson = "{\"nozzle_temperature\":[\"230\"]}"
        });

        var result = await service.UpdateProfileAsync(created.Profile!.Id, new ProfileInput
        {
            Name = "Imported (tweaked)",
            NozzleTempC = "232"
        });

        Assert.True(result.Ok);
        Assert.Equal(ProfileSource.SlicerImport, result.Profile!.Source);
        Assert.Equal(SlicerType.BambuStudio, result.Profile.SourceSlicer);
        Assert.Equal("{\"nozzle_temperature\":[\"230\"]}", result.Profile.RawSettingsJson);
    }

    [Fact]
    public async Task CreateProfile_StoresSourcePresetPathAndVersionNumber()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new PrintProfileService(db);

        var result = await service.CreateProfileAsync(filamentId, new ProfileInput
        {
            Name = "Yellow v2",
            NozzleTempC = "215",
            SourcePresetPath = "/fake/Bambu PLA Basic - FRC - Yellow.json",
            VersionNumber = 2
        });

        Assert.True(result.Ok);
        Assert.Equal("/fake/Bambu PLA Basic - FRC - Yellow.json", result.Profile!.SourcePresetPath);
        Assert.Equal(2, result.Profile.VersionNumber);
    }

    [Fact]
    public async Task CreateProfile_StoresVersionName()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new PrintProfileService(db);

        var result = await service.CreateProfileAsync(filamentId, new ProfileInput
        {
            Name = "Yellow",
            NozzleTempC = "215",
            SourcePresetPath = "/fake/Yellow.json",
            VersionNumber = 2,
            VersionName = "Winter 2026"
        });

        Assert.True(result.Ok);
        Assert.Equal("Winter 2026", result.Profile!.VersionName);
    }

    [Fact]
    public async Task RenameVersion_UpdatesVersionNameOnExistingRow()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new PrintProfileService(db);
        var created = await service.CreateProfileAsync(filamentId, new ProfileInput { Name = "Yellow", NozzleTempC = "215" });

        await service.RenameVersionAsync(created.Profile!.Id, "Summer 2026");

        var reloaded = await service.GetProfileAsync(created.Profile.Id);
        Assert.Equal("Summer 2026", reloaded!.VersionName);
    }

    [Fact]
    public async Task UpdateProfile_LeavesSourcePresetPathAndVersionNumberUntouched()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new PrintProfileService(db);

        var created = await service.CreateProfileAsync(filamentId, new ProfileInput
        {
            Name = "Yellow",
            NozzleTempC = "215",
            SourcePresetPath = "/fake/Bambu PLA Basic - FRC - Yellow.json",
            VersionNumber = 1
        });

        var result = await service.UpdateProfileAsync(created.Profile!.Id, new ProfileInput
        {
            Name = "Yellow (renamed)",
            NozzleTempC = "220"
        });

        Assert.True(result.Ok);
        Assert.Equal("/fake/Bambu PLA Basic - FRC - Yellow.json", result.Profile!.SourcePresetPath);
        Assert.Equal(1, result.Profile.VersionNumber);
    }

    [Fact]
    public async Task UpdateProfile_PersistsSourcePresetPathWhenLinkedDuringEdit()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new PrintProfileService(db);

        var created = await service.CreateProfileAsync(filamentId, new ProfileInput
        {
            Name = "Manual",
            NozzleTempC = "230"
        });

        var result = await service.UpdateProfileAsync(created.Profile!.Id, new ProfileInput
        {
            Name = "Manual",
            NozzleTempC = "230",
            Source = ProfileSource.SlicerImport,
            SourceSlicer = SlicerType.BambuStudio,
            RawSettingsJson = "{\"nozzle_temperature\":[\"230\"]}",
            SourcePresetPath = "/fake/Bambu PLA Basic - FRC - Yellow.json"
        });

        Assert.True(result.Ok);
        Assert.Equal(ProfileSource.SlicerImport, result.Profile!.Source);
        Assert.Equal(SlicerType.BambuStudio, result.Profile.SourceSlicer);
        Assert.Equal("{\"nozzle_temperature\":[\"230\"]}", result.Profile.RawSettingsJson);
        Assert.Equal("/fake/Bambu PLA Basic - FRC - Yellow.json", result.Profile.SourcePresetPath);
    }

    [Fact]
    public async Task ListVersions_ReturnsAllProfilesSharingSourcePresetPathOrderedByVersion()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new PrintProfileService(db);
        const string path = "/fake/Bambu PLA Basic - FRC - Yellow.json";

        await service.CreateProfileAsync(filamentId, new ProfileInput
        {
            Name = "Yellow", NozzleTempC = "215", SourcePresetPath = path, VersionNumber = 2
        });
        await service.CreateProfileAsync(filamentId, new ProfileInput
        {
            Name = "Yellow", NozzleTempC = "210", SourcePresetPath = path, VersionNumber = 1
        });
        await service.CreateProfileAsync(filamentId, new ProfileInput
        {
            Name = "Unrelated", NozzleTempC = "200", SourcePresetPath = "/fake/other.json", VersionNumber = 1
        });

        var versions = await service.ListVersionsAsync(filamentId, path);

        Assert.Equal(new[] { 1, 2 }, versions.Select(v => v.VersionNumber));
    }

    [Fact]
    public async Task CreateProfile_DemotesPreviousVersionsWithSameSourcePresetPath()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new PrintProfileService(db);
        const string path = "/fake/Bambu PLA Basic - FRC - Yellow.json";

        var v1 = await service.CreateProfileAsync(filamentId, new ProfileInput
        {
            Name = "Yellow", NozzleTempC = "210", SourcePresetPath = path, VersionNumber = 1
        });

        var v2 = await service.CreateProfileAsync(filamentId, new ProfileInput
        {
            Name = "Yellow", NozzleTempC = "215", SourcePresetPath = path, VersionNumber = 2
        });

        var reloadedV1 = await service.GetProfileAsync(v1.Profile!.Id);
        Assert.False(reloadedV1!.IsCurrentVersion);
        Assert.True(v2.Profile!.IsCurrentVersion);
    }

    [Fact]
    public async Task ListProfilesForFilament_OnlyReturnsCurrentVersions()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new PrintProfileService(db);
        const string path = "/fake/Bambu PLA Basic - FRC - Yellow.json";

        await service.CreateProfileAsync(filamentId, new ProfileInput
        {
            Name = "Yellow", NozzleTempC = "210", SourcePresetPath = path, VersionNumber = 1
        });
        await service.CreateProfileAsync(filamentId, new ProfileInput
        {
            Name = "Yellow", NozzleTempC = "215", SourcePresetPath = path, VersionNumber = 2
        });

        var profiles = await service.ListProfilesForFilamentAsync(filamentId);

        Assert.Single(profiles);
        Assert.Equal(2, profiles[0].VersionNumber);
    }

    [Fact]
    public async Task DuplicateProfile_CopiesFieldsWithModifiedName()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new PrintProfileService(db);
        var created = await service.CreateProfileAsync(filamentId, new ProfileInput
        {
            Name = "Standard",
            NozzleTempC = "230",
            Fields = { ["HotPlateTempC"] = "65" }
        });

        var duplicate = await service.DuplicateProfileAsync(created.Profile!.Id);

        Assert.NotEqual(created.Profile.Id, duplicate.Id);
        Assert.Equal("Standard (copy)", duplicate.Name);
        Assert.Equal(230, duplicate.NozzleTempC);
        Assert.Equal(65, duplicate.HotPlateTempC);
    }

    [Fact]
    public async Task ListProfilesForFilament_ReturnsGenericBeforeSpoolSpecific()
    {
        using var db = TestDbFactory.Create();
        var filamentService = new FilamentService(db);
        var filamentId = await SeedFilamentAsync(filamentService);
        var spoolService = new SpoolService(db);
        var spool = await spoolService.CreateSpoolAsync(filamentId, new SpoolInput { LotCode = "A" });
        var service = new PrintProfileService(db);

        await service.CreateProfileAsync(filamentId, new ProfileInput { Name = "Spool tweak", NozzleTempC = "235", SpoolId = spool.Spool!.Id });
        await service.CreateProfileAsync(filamentId, new ProfileInput { Name = "Generic", NozzleTempC = "230" });

        var profiles = await service.ListProfilesForFilamentAsync(filamentId);

        Assert.Equal(new[] { "Generic", "Spool tweak" }, profiles.Select(p => p.Name));
    }

    [Fact]
    public async Task DeleteProfile_RemovesIt()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new PrintProfileService(db);
        var created = await service.CreateProfileAsync(filamentId, new ProfileInput { Name = "Standard", NozzleTempC = "230" });

        var result = await service.DeleteProfileAsync(created.Profile!.Id);

        Assert.True(result.Ok);
        Assert.Null(await service.GetProfileAsync(created.Profile.Id));
    }

    private static async Task SeedPrintReferencingProfileAsync(SpoolbookDbContext db, int filamentId, int profileId)
    {
        var spoolService = new SpoolService(db);
        var spool = await spoolService.CreateSpoolAsync(filamentId, new SpoolInput());
        db.Prints.Add(new Print
        {
            ProfileId = profileId,
            SpoolId = spool.Spool!.Id,
            Printer = "Bambu Lab P2S",
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0, DateTimeKind.Utc),
            EndedAt = new DateTime(2026, 1, 1, 10, 0, 0, DateTimeKind.Utc),
            Status = PrintStatus.Success
        });
        await db.SaveChangesAsync();
    }

    [Fact]
    public async Task UpdateProfile_BlockedOncePrintReferencesIt()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new PrintProfileService(db);
        var created = await service.CreateProfileAsync(filamentId, new ProfileInput { Name = "Standard", NozzleTempC = "230" });
        await SeedPrintReferencingProfileAsync(db, filamentId, created.Profile!.Id);

        var result = await service.UpdateProfileAsync(created.Profile.Id, new ProfileInput { Name = "Standard (edited)", NozzleTempC = "235" });

        Assert.False(result.Ok);
        Assert.True(result.Errors!.ContainsKey("Locked"));
    }

    [Fact]
    public async Task DeleteProfile_BlockedWhilePrintsExist()
    {
        using var db = TestDbFactory.Create();
        var filamentId = await SeedFilamentAsync(new FilamentService(db));
        var service = new PrintProfileService(db);
        var created = await service.CreateProfileAsync(filamentId, new ProfileInput { Name = "Standard", NozzleTempC = "230" });
        await SeedPrintReferencingProfileAsync(db, filamentId, created.Profile!.Id);

        var result = await service.DeleteProfileAsync(created.Profile.Id);

        Assert.False(result.Ok);
        Assert.Equal("has_prints", result.Error);
    }
}
