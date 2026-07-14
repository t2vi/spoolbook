using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;

using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Settings.Colors;
using Spoolbook.Desktop.Features.Spools;
namespace Spoolbook.Desktop.Features.Settings.Filaments;

public class FilamentInput
{
    public required string Brand { get; set; }
    public required string Material { get; set; }
    public string? Variant { get; set; }
    public required string Color { get; set; }
    // Set by the scraper (see ColorHexResolver) when the color name matched a known hex; null
    // falls back to the #CCCCCC placeholder in EnsureColorExistsAsync.
    public string? Hex { get; set; }
}

public class FilamentResult
{
    public bool Ok { get; init; }
    public Filament? Entry { get; init; }
    public string? Error { get; init; }
}

public class FilamentImportSummary
{
    public int Added { get; init; }
    public int Skipped { get; init; }
}

public enum FilamentSortColumn { Brand, Material, Variant, Color }

public class FilamentQuery
{
    public string? Brand { get; set; }
    public string? Material { get; set; }
    public string? Variant { get; set; }
    public string? Color { get; set; }
    public FilamentSortColumn Sort { get; set; } = FilamentSortColumn.Brand;
    public SortOrder Order { get; set; } = SortOrder.Asc;
    public int Page { get; set; } = 1;
    public int PageSize { get; set; } = 20;
}

public class FilamentSearchResult
{
    public required List<Filament> Entries { get; init; }
    public int Total { get; init; }
    public int Page { get; init; }
    public int PageSize { get; init; }
    public int TotalPages { get; init; }
}

public class FilamentService
{
    private readonly SpoolbookDbContext _db;

    public FilamentService(SpoolbookDbContext db)
    {
        _db = db;
    }

    public async Task<List<Filament>> ListAsync() =>
        await _db.Filaments.OrderBy(e => e.Brand).ThenBy(e => e.Material).ToListAsync();

    public async Task<List<string>> ListDistinctBrandsAsync() =>
        await _db.Filaments.Select(e => e.Brand).Distinct().OrderBy(b => b).ToListAsync();

    public async Task<List<string>> ListDistinctMaterialsAsync() =>
        await _db.Filaments.Select(e => e.Material).Distinct().OrderBy(m => m).ToListAsync();

    public async Task<List<string>> ListDistinctVariantsAsync() =>
        await _db.Filaments.Where(e => e.Variant != null).Select(e => e.Variant!).Distinct().OrderBy(v => v).ToListAsync();

    public async Task<FilamentSearchResult> SearchAsync(FilamentQuery query)
    {
        var page = Math.Max(1, query.Page);
        var pageSize = query.PageSize;

        var baseQuery = _db.Filaments.AsQueryable();

        if (!string.IsNullOrEmpty(query.Brand))
            baseQuery = baseQuery.Where(e => e.Brand == query.Brand);
        if (!string.IsNullOrEmpty(query.Material))
            baseQuery = baseQuery.Where(e => e.Material == query.Material);
        if (!string.IsNullOrEmpty(query.Variant))
            baseQuery = baseQuery.Where(e => e.Variant == query.Variant);
        if (!string.IsNullOrEmpty(query.Color))
            baseQuery = baseQuery.Where(e => e.Color == query.Color);

        var total = await baseQuery.CountAsync();

        baseQuery = query.Sort switch
        {
            FilamentSortColumn.Material => query.Order == SortOrder.Desc
                ? baseQuery.OrderByDescending(e => e.Material)
                : baseQuery.OrderBy(e => e.Material),
            FilamentSortColumn.Variant => query.Order == SortOrder.Desc
                ? baseQuery.OrderByDescending(e => e.Variant)
                : baseQuery.OrderBy(e => e.Variant),
            FilamentSortColumn.Color => query.Order == SortOrder.Desc
                ? baseQuery.OrderByDescending(e => e.Color)
                : baseQuery.OrderBy(e => e.Color),
            _ => query.Order == SortOrder.Desc
                ? baseQuery.OrderByDescending(e => e.Brand)
                : baseQuery.OrderBy(e => e.Brand)
        };

        var entries = await baseQuery
            .Skip((page - 1) * pageSize)
            .Take(pageSize)
            .ToListAsync();

        return new FilamentSearchResult
        {
            Entries = entries,
            Total = total,
            Page = page,
            PageSize = pageSize,
            TotalPages = Math.Max(1, (int)Math.Ceiling(total / (double)pageSize))
        };
    }

    private static string? Validate(FilamentInput input)
    {
        if (string.IsNullOrWhiteSpace(input.Brand)) return "Brand is required";
        if (string.IsNullOrWhiteSpace(input.Material)) return "Material is required";
        if (string.IsNullOrWhiteSpace(input.Color)) return "Color is required";
        return null;
    }

    private async Task<bool> IsDuplicateAsync(FilamentInput input, int? excludeId) =>
        await _db.Filaments.AnyAsync(e =>
            e.Brand == input.Brand && e.Material == input.Material &&
            e.Variant == input.Variant && e.Color == input.Color &&
            (excludeId == null || e.Id != excludeId));

    // Colors aren't hand-seeded independently — they're discovered from filaments (known or
    // owned) as they're added. Uses the scraper's resolved hex when available (see
    // FilamentInput.Hex / ColorHexResolver); otherwise a placeholder the user can correct in
    // Settings.
    private async Task EnsureColorExistsAsync(string name, string? hex = null)
    {
        if (!await _db.FilamentColors.AnyAsync(c => c.Name == name))
            _db.FilamentColors.Add(new FilamentColor { Name = name, Hex = hex ?? "#CCCCCC" });
    }

    public async Task<FilamentResult> CreateAsync(FilamentInput input)
    {
        var error = Validate(input);
        if (error is not null) return new FilamentResult { Ok = false, Error = error };
        if (await IsDuplicateAsync(input, null))
            return new FilamentResult { Ok = false, Error = "duplicate" };

        var entry = new Filament { Brand = input.Brand, Material = input.Material, Variant = input.Variant, Color = input.Color };
        _db.Filaments.Add(entry);
        await EnsureColorExistsAsync(input.Color, input.Hex);
        await _db.SaveChangesAsync();

        return new FilamentResult { Ok = true, Entry = entry };
    }

    public async Task<FilamentImportSummary> ImportManyAsync(IEnumerable<FilamentInput> inputs)
    {
        int added = 0, skipped = 0;
        foreach (var input in inputs)
        {
            var result = await CreateAsync(input);
            if (result.Ok) added++; else skipped++;
        }
        return new FilamentImportSummary { Added = added, Skipped = skipped };
    }

    public async Task<FilamentResult> UpdateAsync(int id, FilamentInput input)
    {
        var error = Validate(input);
        if (error is not null) return new FilamentResult { Ok = false, Error = error };
        if (await IsDuplicateAsync(input, id))
            return new FilamentResult { Ok = false, Error = "duplicate" };

        var entry = await _db.Filaments.FindAsync(id);
        if (entry is null) throw new InvalidOperationException("Entry not found");

        entry.Brand = input.Brand;
        entry.Material = input.Material;
        entry.Variant = input.Variant;
        entry.Color = input.Color;
        await EnsureColorExistsAsync(input.Color);
        await _db.SaveChangesAsync();

        return new FilamentResult { Ok = true, Entry = entry };
    }

    public async Task<FilamentResult> DeleteAsync(int id)
    {
        var entry = await _db.Filaments.FindAsync(id);
        if (entry is null) throw new InvalidOperationException("Entry not found");

        if (await _db.Spools.AnyAsync(s => s.FilamentId == id))
            return new FilamentResult { Ok = false, Error = "has_spools" };

        _db.Filaments.Remove(entry);
        await _db.SaveChangesAsync();

        return new FilamentResult { Ok = true };
    }
}
