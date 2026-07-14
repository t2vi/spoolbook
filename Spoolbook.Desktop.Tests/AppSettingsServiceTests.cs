using Spoolbook.Desktop.Features.Settings;
using Spoolbook.Desktop.Features.Settings.General;
using Spoolbook.Desktop.Features.Settings.Colors;
using Spoolbook.Desktop.Features.Settings.Filaments;
namespace Spoolbook.Desktop.Tests;

public class AppSettingsServiceTests
{
    [Fact]
    public async Task GetAsync_CreatesDefaultRowWithNullOverridesWhenMissing()
    {
        using var db = TestDbFactory.Create();
        var service = new AppSettingsService(db);

        var settings = await service.GetAsync();

        Assert.Null(settings.BambuUserPresetsDir);
        Assert.Null(settings.BambuSystemProfilesDir);
    }

    [Fact]
    public async Task SaveAsync_PersistsOverrides()
    {
        using var db = TestDbFactory.Create();
        var service = new AppSettingsService(db);

        await service.SaveAsync(new AppSettingsInput
        {
            BambuUserPresetsDir = "/custom/user",
            BambuSystemProfilesDir = "/custom/system"
        });

        var settings = await service.GetAsync();
        Assert.Equal("/custom/user", settings.BambuUserPresetsDir);
        Assert.Equal("/custom/system", settings.BambuSystemProfilesDir);
    }

    [Fact]
    public async Task SaveAsync_BlankValueClearsOverrideToNull()
    {
        using var db = TestDbFactory.Create();
        var service = new AppSettingsService(db);
        await service.SaveAsync(new AppSettingsInput { BambuUserPresetsDir = "/custom/user" });

        await service.SaveAsync(new AppSettingsInput { BambuUserPresetsDir = "  " });

        var settings = await service.GetAsync();
        Assert.Null(settings.BambuUserPresetsDir);
    }

    [Fact]
    public async Task RecordFilamentSyncAsync_SetsLastFilamentSyncAt()
    {
        using var db = TestDbFactory.Create();
        var service = new AppSettingsService(db);

        await service.RecordFilamentSyncAsync();

        var settings = await service.GetAsync();
        Assert.NotNull(settings.LastFilamentSyncAt);
    }

    [Fact]
    public async Task GetAdditionalFilamentSourceUrlsAsync_ReturnsEmptyWhenUnset()
    {
        using var db = TestDbFactory.Create();
        var service = new AppSettingsService(db);

        var urls = await service.GetAdditionalFilamentSourceUrlsAsync();

        Assert.Empty(urls);
    }

    [Fact]
    public async Task GetAdditionalFilamentSourceUrlsAsync_SplitsNewlineSeparatedList()
    {
        using var db = TestDbFactory.Create();
        var service = new AppSettingsService(db);
        await service.SaveAsync(new AppSettingsInput
        {
            AdditionalFilamentSourceUrls = "https://example.com/a.json\n  \nhttps://example.com/b.json"
        });

        var urls = await service.GetAdditionalFilamentSourceUrlsAsync();

        Assert.Equal(["https://example.com/a.json", "https://example.com/b.json"], urls);
    }
}
