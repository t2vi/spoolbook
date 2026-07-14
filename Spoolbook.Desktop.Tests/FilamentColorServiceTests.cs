using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Settings;
using Spoolbook.Desktop.Features.Settings.General;
using Spoolbook.Desktop.Features.Settings.Colors;
using Spoolbook.Desktop.Features.Settings.Filaments;
namespace Spoolbook.Desktop.Tests;

public class FilamentColorServiceTests
{
    [Fact]
    public async Task List_ReturnsSeedColorsSortedByName()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentColorService(db);

        var colors = await service.ListAsync();

        Assert.Contains(colors, c => c.Name == "Black");
        // SQLite's default collation is binary (ordinal, case-sensitive) — match that here,
        // since the culture-aware default comparer disagrees with it on mixed-case names.
        Assert.Equal(colors.OrderBy(c => c.Name, StringComparer.Ordinal).Select(c => c.Name), colors.Select(c => c.Name));
    }

    [Fact]
    public async Task Create_AddsNewColor()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentColorService(db);

        var result = await service.CreateAsync(new FilamentColorInput { Name = "Not A Real Seeded Color", Hex = "#4B0082" });

        Assert.True(result.Ok);
        Assert.Equal("Not A Real Seeded Color", result.Color!.Name);
        Assert.Equal("#4B0082", result.Color.Hex);
    }

    [Fact]
    public async Task Create_RejectsDuplicateName()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentColorService(db);

        var result = await service.CreateAsync(new FilamentColorInput { Name = "Black", Hex = "#000000" });

        Assert.False(result.Ok);
        Assert.Equal("duplicate", result.Error);
    }

    [Fact]
    public async Task Create_AcceptsCommaSeparatedMultiColorHex()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentColorService(db);

        var result = await service.CreateAsync(new FilamentColorInput { Name = "Not A Seeded Multi Color", Hex = "#1A1A1A, #D4AF37" });

        Assert.True(result.Ok);
        Assert.Equal(["#1A1A1A", "#D4AF37"], result.Color!.Hexes);
    }

    [Fact]
    public async Task Create_RejectsMalformedHexInMultiColorList()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentColorService(db);

        var result = await service.CreateAsync(new FilamentColorInput { Name = "Bad", Hex = "#1A1A1A, not-a-color" });

        Assert.False(result.Ok);
        Assert.Equal("invalid_hex", result.Error);
    }

    [Fact]
    public async Task Update_RenamesExistingColor()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentColorService(db);
        var created = await service.CreateAsync(new FilamentColorInput { Name = "Test Color", Hex = "#111111" });

        var result = await service.UpdateAsync(created.Color!.Id, new FilamentColorInput { Name = "Renamed", Hex = "#222222" });

        Assert.True(result.Ok);
        Assert.Equal("Renamed", result.Color!.Name);
        Assert.Equal("#222222", result.Color.Hex);
    }

    [Fact]
    public async Task SearchAsync_FiltersByNameAndPaginates()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentColorService(db);
        await service.CreateAsync(new FilamentColorInput { Name = "Zeta Zeta", Hex = "#111111" });
        await service.CreateAsync(new FilamentColorInput { Name = "Zeta Zebra", Hex = "#222222" });

        var result = await service.SearchAsync(new ColorQuery { Name = "Zeta Zeta" });

        Assert.Equal(1, result.Total);
        Assert.Equal("#111111", result.Entries[0].Hex);

        var page1 = await service.SearchAsync(new ColorQuery { Sort = ColorSortColumn.Name, Order = SortOrder.Asc, Page = 1, PageSize = 1 });
        Assert.Single(page1.Entries);
        Assert.True(page1.TotalPages > 1);
    }

    [Fact]
    public async Task Delete_RemovesColor()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentColorService(db);
        var created = await service.CreateAsync(new FilamentColorInput { Name = "Test Color", Hex = "#111111" });

        await service.DeleteAsync(created.Color!.Id);

        var colors = await service.ListAsync();
        Assert.DoesNotContain(colors, c => c.Name == "Test Color");
    }
}
