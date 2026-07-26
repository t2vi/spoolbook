using System.IO.Compression;
using Spoolbook.Desktop.Data;
using Spoolbook.Desktop.Features.Prints;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Settings.Printers;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Features.Profiles;
namespace Spoolbook.Desktop.Tests;

public class ProjectServiceTests
{
    private static string CreateTempFile(string content = "3mf-bytes")
    {
        var path = Path.Combine(Path.GetTempPath(), $"spoolbook-test-{Guid.NewGuid():N}.3mf");
        File.WriteAllText(path, content);
        return path;
    }

    [Fact]
    public async Task UpsertByPathAsync_CreatesNewProject()
    {
        using var db = TestDbFactory.Create();
        var service = new ProjectService(db);
        var path = CreateTempFile();

        var result = await service.UpsertByPathAsync(path);

        Assert.True(result.Ok);
        Assert.Equal(path, result.Project!.FilePath);
        Assert.Equal(Path.GetFileName(path), result.Project.FileName);
        File.Delete(path);
    }

    [Fact]
    public async Task UpsertByPathAsync_ReusesExistingProjectForSamePath()
    {
        using var db = TestDbFactory.Create();
        var service = new ProjectService(db);
        var path = CreateTempFile();

        var first = await service.UpsertByPathAsync(path);
        var second = await service.UpsertByPathAsync(path);

        Assert.Equal(first.Project!.Id, second.Project!.Id);
        Assert.Single(await service.ListAsync());
        File.Delete(path);
    }

    [Fact]
    public async Task UpsertByPathAsync_MissingFile_ReturnsError()
    {
        using var db = TestDbFactory.Create();
        var service = new ProjectService(db);

        var result = await service.UpsertByPathAsync(Path.Combine(Path.GetTempPath(), "does-not-exist.3mf"));

        Assert.False(result.Ok);
        Assert.Equal("file_not_found", result.Error);
    }

    [Fact]
    public async Task ListAsync_ReturnsSortedByFileName()
    {
        using var db = TestDbFactory.Create();
        var service = new ProjectService(db);
        var zPath = Path.Combine(Path.GetTempPath(), "zeta-project.3mf");
        var aPath = Path.Combine(Path.GetTempPath(), "alpha-project.3mf");
        File.WriteAllText(zPath, "z");
        File.WriteAllText(aPath, "a");

        await service.UpsertByPathAsync(zPath);
        await service.UpsertByPathAsync(aPath);
        var projects = await service.ListAsync();

        Assert.Equal("alpha-project.3mf", projects[0].FileName);
        Assert.Equal("zeta-project.3mf", projects[1].FileName);
        File.Delete(zPath);
        File.Delete(aPath);
    }

    [Fact]
    public async Task DeleteAsync_RemovesProject()
    {
        using var db = TestDbFactory.Create();
        var service = new ProjectService(db);
        var path = CreateTempFile();
        var created = await service.UpsertByPathAsync(path);

        var result = await service.DeleteAsync(created.Project!.Id);

        Assert.True(result.Ok);
        Assert.Empty(await service.ListAsync());
        File.Delete(path);
    }

    [Fact]
    public async Task DeleteAsync_BlockedWhilePrintsExist()
    {
        using var db = TestDbFactory.Create();
        var service = new ProjectService(db);
        var path = CreateTempFile();
        var project = await service.UpsertByPathAsync(path);

        var filamentService = new FilamentService(db);
        var filament = await filamentService.CreateAsync(new FilamentInput { Brand = "Test Brand", Material = "PLA", Color = "Black" });
        var spoolService = new SpoolService(db);
        var spool = await spoolService.CreateSpoolAsync(filament.Entry!.Id, new SpoolInput());
        var profileService = new PrintProfileService(db);
        var profile = await profileService.CreateProfileAsync(filament.Entry.Id, new ProfileInput { Name = "Standard", NozzleTempC = "230" });
        var printerService = new PrinterService(db);
        var printer = await printerService.CreateAsync(new PrinterInput { Name = "Garage P2S" });
        var printService = new PrintService(db, new FakeWeatherService());
        await printService.CreateAsync(profile.Profile!.Id, spool.Spool!.Id, printer.Printer!.Id, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0),
            EndedAt = new DateTime(2026, 1, 1, 10, 0, 0),
            Status = PrintStatus.Success,
            ProjectId = project.Project!.Id
        });

        var result = await service.DeleteAsync(project.Project.Id);

        Assert.False(result.Ok);
        Assert.Equal("has_prints", result.Error);
        File.Delete(path);
    }

    [Fact]
    public void GetFileStatus_ReturnsOk_WhenFileUnchanged()
    {
        var path = CreateTempFile();
        var info = new FileInfo(path);
        var project = new Project
        {
            FilePath = path,
            FileName = Path.GetFileName(path),
            LastKnownWriteTimeUtc = info.LastWriteTimeUtc,
            LastKnownFileSizeBytes = info.Length
        };

        Assert.Equal(ProjectFileStatus.Ok, ProjectService.GetFileStatus(project));
        File.Delete(path);
    }

    [Fact]
    public void GetFileStatus_ReturnsMissing_WhenFileDeleted()
    {
        var path = CreateTempFile();
        var info = new FileInfo(path);
        var project = new Project
        {
            FilePath = path,
            FileName = Path.GetFileName(path),
            LastKnownWriteTimeUtc = info.LastWriteTimeUtc,
            LastKnownFileSizeBytes = info.Length
        };
        File.Delete(path);

        Assert.Equal(ProjectFileStatus.Missing, ProjectService.GetFileStatus(project));
    }

    [Fact]
    public void GetFileStatus_ReturnsChanged_WhenSizeDiffers()
    {
        var path = CreateTempFile();
        var info = new FileInfo(path);
        var project = new Project
        {
            FilePath = path,
            FileName = Path.GetFileName(path),
            LastKnownWriteTimeUtc = info.LastWriteTimeUtc,
            LastKnownFileSizeBytes = info.Length + 1 // simulate stale stat
        };

        Assert.Equal(ProjectFileStatus.Changed, ProjectService.GetFileStatus(project));
        File.Delete(path);
    }

    private static string CreateTemp3mf(params (string PlaterId, string? PlaterName, bool HasThumbnail)[] plates)
    {
        var path = Path.Combine(Path.GetTempPath(), $"spoolbook-test-{Guid.NewGuid():N}.3mf");
        using (var archive = ZipFile.Open(path, ZipArchiveMode.Create))
        {
            var config = new System.Text.StringBuilder("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<config>\n");
            foreach (var plate in plates)
            {
                config.Append("  <plate>\n");
                config.Append($"    <metadata key=\"plater_id\" value=\"{plate.PlaterId}\"/>\n");
                config.Append($"    <metadata key=\"plater_name\" value=\"{plate.PlaterName}\"/>\n");
                if (plate.HasThumbnail)
                    config.Append($"    <metadata key=\"thumbnail_file\" value=\"Metadata/plate_{plate.PlaterId}.png\"/>\n");
                config.Append("  </plate>\n");
            }
            config.Append("</config>\n");

            var configEntry = archive.CreateEntry("Metadata/model_settings.config");
            using (var writer = new StreamWriter(configEntry.Open()))
                writer.Write(config.ToString());

            foreach (var plate in plates.Where(p => p.HasThumbnail))
            {
                var thumbEntry = archive.CreateEntry($"Metadata/plate_{plate.PlaterId}.png");
                using var thumbStream = thumbEntry.Open();
                thumbStream.Write([1, 2, 3, 4]);
            }
        }
        return path;
    }

    [Fact]
    public void ReadPlates_ParsesSinglePlateWithThumbnail()
    {
        var path = CreateTemp3mf(("1", "", true));

        var plates = ProjectService.ReadPlates(path);

        Assert.Single(plates);
        Assert.Equal("1", plates[0].PlaterId);
        Assert.Null(plates[0].PlaterName);
        Assert.Equal(new byte[] { 1, 2, 3, 4 }, plates[0].ThumbnailBytes);
        File.Delete(path);
    }

    [Fact]
    public void ReadPlates_ParsesMultiplePlatesInOrder()
    {
        var path = CreateTemp3mf(("1", "Front", true), ("2", "Back", true));

        var plates = ProjectService.ReadPlates(path);

        Assert.Equal(2, plates.Count);
        Assert.Equal("1", plates[0].PlaterId);
        Assert.Equal("Front", plates[0].PlaterName);
        Assert.Equal("2", plates[1].PlaterId);
        Assert.Equal("Back", plates[1].PlaterName);
        File.Delete(path);
    }

    [Fact]
    public void ReadPlates_MissingThumbnailFile_ReturnsNullBytes()
    {
        var path = CreateTemp3mf(("1", "Only", false));

        var plates = ProjectService.ReadPlates(path);

        Assert.Single(plates);
        Assert.Null(plates[0].ThumbnailBytes);
        File.Delete(path);
    }

    [Fact]
    public void ReadPlates_NoModelSettingsConfig_ReturnsEmpty()
    {
        var path = Path.Combine(Path.GetTempPath(), $"spoolbook-test-{Guid.NewGuid():N}.3mf");
        using (var archive = ZipFile.Open(path, ZipArchiveMode.Create))
            archive.CreateEntry("Metadata/placeholder.txt");

        var plates = ProjectService.ReadPlates(path);

        Assert.Empty(plates);
        File.Delete(path);
    }
}
