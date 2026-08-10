using Spoolbook.Desktop.Data;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Settings.Printers;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Features.Prints;
using Spoolbook.Desktop.Services.Weather;
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
    public async Task CreateAsync_StoresOptionalProjectReference()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var service = new PrintService(db, new FakeWeatherService());
        var projectService = new ProjectService(db);
        var path = Path.Combine(Path.GetTempPath(), $"spoolbook-test-{Guid.NewGuid():N}.3mf");
        File.WriteAllText(path, "3mf-bytes");
        var project = await projectService.UpsertByPathAsync(path);

        var result = await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0),
            EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Success,
            ProjectId = project.Project!.Id
        });

        Assert.Equal(project.Project.Id, result.Print!.ProjectId);
        File.Delete(path);
    }

    [Fact]
    public async Task CreateAsync_ProjectIsOptional()
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

        Assert.Null(result.Print!.ProjectId);
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

    [Fact]
    public async Task CreateAsync_StoresProjectPlaterId()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var service = new PrintService(db, new FakeWeatherService());

        var result = await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0),
            EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Success,
            ProjectPlaterId = "2"
        });

        Assert.Equal("2", result.Print!.ProjectPlaterId);
    }

    [Fact]
    public async Task UpdateAsync_UpdatesProjectPlaterId()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var service = new PrintService(db, new FakeWeatherService());
        var created = await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0), Status = PrintStatus.Success,
            ProjectPlaterId = "1"
        });

        var result = await service.UpdateAsync(created.Print!.Id, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0), Status = PrintStatus.Success,
            ProjectPlaterId = "3"
        });

        Assert.Equal("3", result.Print!.ProjectPlaterId);
    }

    [Fact]
    public async Task CreateAsync_StoresFailureModesForFailedPrint()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var service = new PrintService(db, new FakeWeatherService());

        var result = await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Failed,
            FailureModes = [FailureMode.Stringing, FailureMode.LayerAdhesion]
        });

        Assert.True(result.Ok);
        Assert.Equal(2, result.Print!.FailureModes.Count);
        Assert.Contains(result.Print.FailureModes, f => f.Mode == FailureMode.Stringing);
        Assert.Contains(result.Print.FailureModes, f => f.Mode == FailureMode.LayerAdhesion);
    }

    [Fact]
    public async Task CreateAsync_StoresFailureModesForPartialPrint()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var service = new PrintService(db, new FakeWeatherService());

        var result = await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Partial,
            FailureModes = [FailureMode.Warping]
        });

        Assert.True(result.Ok);
        Assert.Single(result.Print!.FailureModes);
    }

    [Fact]
    public async Task CreateAsync_RejectsFailureModesWhenStatusIsSuccess()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var service = new PrintService(db, new FakeWeatherService());

        var result = await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Success,
            FailureModes = [FailureMode.Stringing]
        });

        Assert.False(result.Ok);
        Assert.Equal("failure_modes_require_failed_or_partial", result.Error);
    }

    [Fact]
    public async Task UpdateAsync_ReplacesFailureModes()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var service = new PrintService(db, new FakeWeatherService());
        var created = await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Failed,
            FailureModes = [FailureMode.Stringing]
        });

        var result = await service.UpdateAsync(created.Print!.Id, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Partial,
            FailureModes = [FailureMode.Warping, FailureMode.Clog]
        });

        Assert.True(result.Ok);
        Assert.Equal(2, result.Print!.FailureModes.Count);
        Assert.DoesNotContain(result.Print.FailureModes, f => f.Mode == FailureMode.Stringing);
    }

    [Fact]
    public async Task UpdateAsync_RejectsFailureModesWhenStatusIsSuccess()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var service = new PrintService(db, new FakeWeatherService());
        var created = await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Failed,
            FailureModes = [FailureMode.Stringing]
        });

        var result = await service.UpdateAsync(created.Print!.Id, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Success,
            FailureModes = [FailureMode.Stringing]
        });

        Assert.False(result.Ok);
        Assert.Equal("failure_modes_require_failed_or_partial", result.Error);
    }

    private static async Task<Project> SeedProjectAsync(SpoolbookDbContext db)
    {
        var projectService = new ProjectService(db);
        var path = Path.Combine(Path.GetTempPath(), $"spoolbook-test-{Guid.NewGuid():N}.3mf");
        File.WriteAllText(path, "3mf-bytes");
        var result = await projectService.UpsertByPathAsync(path);
        return result.Project!;
    }

    [Fact]
    public async Task RecommendProfileForProjectAsync_PrefersSuccessOverPartialOverFailed()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var filamentId = (await db.PrintProfiles.FindAsync(profileId))!.FilamentId;
        var profileService = new PrintProfileService(db);
        var successProfile = await profileService.CreateProfileAsync(filamentId, new ProfileInput { Name = "Success profile", NozzleTempC = "235" });
        var service = new PrintService(db, new FakeWeatherService());
        var project = await SeedProjectAsync(db);

        await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Failed, ProjectId = project.Id
        });
        await service.CreateAsync(successProfile.Profile!.Id, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 2, 8, 0, 0), EndedAt = new DateTime(2026, 1, 2, 10, 0, 0),
            Status = PrintStatus.Success, ProjectId = project.Id
        });

        var recommended = await service.RecommendProfileForProjectAsync(project.Id, currentTempC: null);

        Assert.Equal(successProfile.Profile.Id, recommended!.Id);
    }

    [Fact]
    public async Task RecommendProfileForProjectAsync_TiesBrokenByClosestActualRoomTemp()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var filamentId = (await db.PrintProfiles.FindAsync(profileId))!.FilamentId;
        var profileService = new PrintProfileService(db);
        var coldProfile = await profileService.CreateProfileAsync(filamentId, new ProfileInput { Name = "Cold", NozzleTempC = "230" });
        var hotProfile = await profileService.CreateProfileAsync(filamentId, new ProfileInput { Name = "Hot", NozzleTempC = "240" });
        var service = new PrintService(db, new FakeWeatherService());
        var project = await SeedProjectAsync(db);

        await service.CreateAsync(coldProfile.Profile!.Id, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Success, ProjectId = project.Id, ActualRoomTempC = 15m
        });
        await service.CreateAsync(hotProfile.Profile!.Id, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 2, 8, 0, 0), EndedAt = new DateTime(2026, 1, 2, 10, 0, 0),
            Status = PrintStatus.Success, ProjectId = project.Id, ActualRoomTempC = 25m
        });

        var recommended = await service.RecommendProfileForProjectAsync(project.Id, currentTempC: 23m);

        Assert.Equal(hotProfile.Profile.Id, recommended!.Id);
    }

    [Fact]
    public async Task RecommendProfileForProjectAsync_FallsBackToAmbientTempWhenRoomTempMissing()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var filamentId = (await db.PrintProfiles.FindAsync(profileId))!.FilamentId;
        var profileService = new PrintProfileService(db);
        var profileA = await profileService.CreateProfileAsync(filamentId, new ProfileInput { Name = "A", NozzleTempC = "230" });
        var profileB = await profileService.CreateProfileAsync(filamentId, new ProfileInput { Name = "B", NozzleTempC = "240" });
        var service = new PrintService(db, new FakeWeatherService { Result = (12m, 50m) });
        var project = await SeedProjectAsync(db);

        // No ActualRoomTempC -> falls back to fetched AmbientTempC (12), which matches currentTempC exactly.
        await service.CreateAsync(profileA.Profile!.Id, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Success, ProjectId = project.Id
        });
        await service.CreateAsync(profileB.Profile!.Id, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 2, 8, 0, 0), EndedAt = new DateTime(2026, 1, 2, 10, 0, 0),
            Status = PrintStatus.Success, ProjectId = project.Id, ActualRoomTempC = 30m
        });

        var recommended = await service.RecommendProfileForProjectAsync(project.Id, currentTempC: 12m);

        Assert.Equal(profileA.Profile.Id, recommended!.Id);
    }

    [Fact]
    public async Task RecommendProfileForProjectAsync_ReturnsNullWhenNoPrintsForProject()
    {
        using var db = TestDbFactory.Create();
        var service = new PrintService(db, new FakeWeatherService());
        var project = await SeedProjectAsync(db);

        var recommended = await service.RecommendProfileForProjectAsync(project.Id, currentTempC: 20m);

        Assert.Null(recommended);
    }

    [Fact]
    public async Task RecommendProfileForProjectAsync_IgnoresPrintsForOtherProjects()
    {
        using var db = TestDbFactory.Create();
        var (profileId, spoolId, printerId) = await SeedAsync(db);
        var service = new PrintService(db, new FakeWeatherService());
        var targetProject = await SeedProjectAsync(db);
        var otherProject = await SeedProjectAsync(db);

        await service.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Success, ProjectId = otherProject.Id
        });

        var recommended = await service.RecommendProfileForProjectAsync(targetProject.Id, currentTempC: 20m);

        Assert.Null(recommended);
    }
}
