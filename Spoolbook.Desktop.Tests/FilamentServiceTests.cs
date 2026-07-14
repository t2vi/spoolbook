using Microsoft.EntityFrameworkCore;

using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Settings.General;
using Spoolbook.Desktop.Features.Settings.Colors;
using Spoolbook.Desktop.Features.Settings.Filaments;
namespace Spoolbook.Desktop.Tests;

public class FilamentServiceTests
{
    [Fact]
    public async Task ListDistinctBrandsAsync_ReturnsSortedDistinctBrands()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);
        await service.CreateAsync(new FilamentInput { Brand = "Zeta Brand", Material = "PLA", Color = "Black" });
        await service.CreateAsync(new FilamentInput { Brand = "Zeta Brand", Material = "PETG", Color = "White" });

        var brands = await service.ListDistinctBrandsAsync();

        Assert.Equal(1, brands.Count(b => b == "Zeta Brand"));
    }

    [Fact]
    public async Task ListDistinctVariantsAsync_ExcludesNulls()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);
        await service.CreateAsync(new FilamentInput { Brand = "Zeta Brand", Material = "PLA", Variant = "Zeta Variant", Color = "Black" });
        await service.CreateAsync(new FilamentInput { Brand = "Zeta Brand", Material = "PETG", Color = "White" });

        var variants = await service.ListDistinctVariantsAsync();

        Assert.Contains("Zeta Variant", variants);
        Assert.DoesNotContain(variants, v => v is null);
    }

    [Fact]
    public async Task Create_AddsNewColorNameToFilamentColorsIfMissing()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);

        await service.CreateAsync(new FilamentInput { Brand = "Test Brand", Material = "PLA", Color = "Test Novel Color" });

        Assert.True(await db.FilamentColors.AnyAsync(c => c.Name == "Test Novel Color"));
    }

    [Fact]
    public async Task Create_NoHexProvided_UsesPlaceholder()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);

        await service.CreateAsync(new FilamentInput { Brand = "Test Brand", Material = "PLA", Color = "Test Novel Color" });

        var color = await db.FilamentColors.SingleAsync(c => c.Name == "Test Novel Color");
        Assert.Equal("#CCCCCC", color.Hex);
    }

    [Fact]
    public async Task Create_HexProvided_UsesResolvedHexInsteadOfPlaceholder()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);

        await service.CreateAsync(new FilamentInput { Brand = "Test Brand", Material = "PLA", Color = "Test Resolved Color", Hex = "#FF69B4" });

        var color = await db.FilamentColors.SingleAsync(c => c.Name == "Test Resolved Color");
        Assert.Equal("#FF69B4", color.Hex);
    }

    [Fact]
    public async Task Create_DoesNotDuplicateFilamentColorIfNameAlreadyExists()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);

        await service.CreateAsync(new FilamentInput { Brand = "Test Brand", Material = "PLA", Color = "Black" });

        Assert.Equal(1, await db.FilamentColors.CountAsync(c => c.Name == "Black"));
    }

    [Fact]
    public async Task List_ReturnsSeedEntriesSortedByBrandThenMaterial()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);

        var entries = await service.ListAsync();

        Assert.Contains(entries, e => e.Brand == "Bambu Lab" && e.Material == "PLA" && e.Variant == "Basic");
    }

    [Fact]
    public async Task Create_AddsNewEntry()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);

        var result = await service.CreateAsync(new FilamentInput
        {
            Brand = "Test Brand", Material = "PLA", Variant = "Matte", Color = "Black"
        });

        Assert.True(result.Ok);
        Assert.Equal("Test Brand", result.Entry!.Brand);
    }

    [Fact]
    public async Task Create_RejectsDuplicateBrandMaterialVariantColor()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);
        await service.CreateAsync(new FilamentInput { Brand = "Test Brand", Material = "PLA", Color = "Black" });

        var result = await service.CreateAsync(new FilamentInput { Brand = "Test Brand", Material = "PLA", Color = "Black" });

        Assert.False(result.Ok);
        Assert.Equal("duplicate", result.Error);
    }

    [Fact]
    public async Task Update_ChangesExistingEntry()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);
        var created = await service.CreateAsync(new FilamentInput { Brand = "Test Brand", Material = "PLA", Color = "Black" });

        var result = await service.UpdateAsync(created.Entry!.Id, new FilamentInput
        {
            Brand = "Test Brand", Material = "PLA", Color = "White"
        });

        Assert.True(result.Ok);
        Assert.Equal("White", result.Entry!.Color);
    }

    private async Task SeedSearchDataAsync(FilamentService service)
    {
        await service.CreateAsync(new FilamentInput { Brand = "Zeta Brand", Material = "PETG", Variant = "Basic", Color = "Black" });
        await service.CreateAsync(new FilamentInput { Brand = "Alpha Brand", Material = "PLA", Variant = "Matte", Color = "White" });
        await service.CreateAsync(new FilamentInput { Brand = "Alpha Brand", Material = "ABS", Color = "Red" });
    }

    [Fact]
    public async Task SearchAsync_FiltersByVariant()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);
        await SeedSearchDataAsync(service);

        var result = await service.SearchAsync(new FilamentQuery { Brand = "Alpha Brand", Variant = "Matte" });

        Assert.Equal(1, result.Total);
        Assert.Equal("PLA", result.Entries[0].Material);
    }

    [Fact]
    public async Task SearchAsync_FiltersByColor()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);
        await SeedSearchDataAsync(service);

        var result = await service.SearchAsync(new FilamentQuery { Brand = "Alpha Brand", Color = "Red" });

        Assert.Equal(1, result.Total);
        Assert.Equal("ABS", result.Entries[0].Material);
    }

    [Fact]
    public async Task SearchAsync_FiltersByBrand()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);
        await SeedSearchDataAsync(service);

        var result = await service.SearchAsync(new FilamentQuery { Brand = "Alpha Brand" });

        Assert.Equal(2, result.Total);
        Assert.All(result.Entries, e => Assert.Equal("Alpha Brand", e.Brand));
    }

    [Fact]
    public async Task SearchAsync_FiltersByMaterial()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);
        await SeedSearchDataAsync(service);

        var result = await service.SearchAsync(new FilamentQuery { Brand = "Zeta Brand", Material = "PETG" });

        Assert.Equal(1, result.Total);
        Assert.Equal("Zeta Brand", result.Entries[0].Brand);
    }

    [Fact]
    public async Task SearchAsync_SortsByMaterialDescending()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);
        await SeedSearchDataAsync(service);

        var result = await service.SearchAsync(new FilamentQuery
        {
            Brand = "Alpha Brand", Sort = FilamentSortColumn.Material, Order = SortOrder.Desc
        });

        Assert.Equal("PLA", result.Entries[0].Material);
    }

    [Fact]
    public async Task SearchAsync_Paginates()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);
        await SeedSearchDataAsync(service);

        var page1 = await service.SearchAsync(new FilamentQuery { Brand = "Alpha Brand", Page = 1, PageSize = 1 });
        var page2 = await service.SearchAsync(new FilamentQuery { Brand = "Alpha Brand", Page = 2, PageSize = 1 });

        Assert.Single(page1.Entries);
        Assert.Single(page2.Entries);
        Assert.Equal(2, page1.TotalPages);
    }

    [Fact]
    public async Task Delete_RemovesEntry()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);
        var created = await service.CreateAsync(new FilamentInput { Brand = "Test Brand", Material = "PLA", Color = "Black" });

        await service.DeleteAsync(created.Entry!.Id);

        var entries = await service.ListAsync();
        Assert.DoesNotContain(entries, e => e.Brand == "Test Brand");
    }

    [Fact]
    public async Task ImportManyAsync_AddsNewEntriesAndSkipsDuplicates()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);
        await service.CreateAsync(new FilamentInput { Brand = "Bambu Lab", Material = "PLA", Variant = "Basic", Color = "Existing Color" });

        var summary = await service.ImportManyAsync(
        [
            new FilamentInput { Brand = "Bambu Lab", Material = "PLA", Variant = "Basic", Color = "Existing Color" },
            new FilamentInput { Brand = "Bambu Lab", Material = "PLA", Variant = "Basic", Color = "New Color One" },
            new FilamentInput { Brand = "Bambu Lab", Material = "PETG", Variant = "HF", Color = "New Color Two" }
        ]);

        Assert.Equal(2, summary.Added);
        Assert.Equal(1, summary.Skipped);
    }

    [Fact]
    public async Task Delete_BlockedWhileSpoolsExist()
    {
        using var db = TestDbFactory.Create();
        var service = new FilamentService(db);
        var spoolService = new Spoolbook.Desktop.Features.Spools.SpoolService(db);
        var created = await service.CreateAsync(new FilamentInput { Brand = "Test Brand", Material = "PLA", Color = "Black" });
        await spoolService.CreateSpoolAsync(created.Entry!.Id, new Spoolbook.Desktop.Features.Spools.SpoolInput());

        var result = await service.DeleteAsync(created.Entry.Id);

        Assert.False(result.Ok);
        Assert.Equal("has_spools", result.Error);
    }
}
