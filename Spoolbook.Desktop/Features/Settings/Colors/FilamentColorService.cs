using System.Text.RegularExpressions;
using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;

using Spoolbook.Desktop.Common;
namespace Spoolbook.Desktop.Features.Settings.Colors;

public class FilamentColorInput
{
    public required string Name { get; set; }
    public required string Hex { get; set; }
}

public class FilamentColorResult
{
    public bool Ok { get; init; }
    public FilamentColor? Color { get; init; }
    public string? Error { get; init; }
}

public enum ColorSortColumn { Name, Hex }

public class ColorQuery
{
    public string? Name { get; set; }
    public ColorSortColumn Sort { get; set; } = ColorSortColumn.Name;
    public SortOrder Order { get; set; } = SortOrder.Asc;
    public int Page { get; set; } = 1;
    public int PageSize { get; set; } = 20;
}

public class ColorSearchResult
{
    public required List<FilamentColor> Entries { get; init; }
    public int Total { get; init; }
    public int Page { get; init; }
    public int PageSize { get; init; }
    public int TotalPages { get; init; }
}

public class FilamentColorService
{
    private readonly SpoolbookDbContext _db;

    public FilamentColorService(SpoolbookDbContext db)
    {
        _db = db;
    }

    public async Task<List<FilamentColor>> ListAsync() =>
        await _db.FilamentColors.OrderBy(c => c.Name).ToListAsync();

    public async Task<ColorSearchResult> SearchAsync(ColorQuery query)
    {
        var page = Math.Max(1, query.Page);
        var pageSize = query.PageSize;

        var baseQuery = _db.FilamentColors.AsQueryable();

        if (!string.IsNullOrEmpty(query.Name))
            baseQuery = baseQuery.Where(c => c.Name == query.Name);

        var total = await baseQuery.CountAsync();

        baseQuery = query.Sort switch
        {
            ColorSortColumn.Hex => query.Order == SortOrder.Desc
                ? baseQuery.OrderByDescending(c => c.Hex)
                : baseQuery.OrderBy(c => c.Hex),
            _ => query.Order == SortOrder.Desc
                ? baseQuery.OrderByDescending(c => c.Name)
                : baseQuery.OrderBy(c => c.Name)
        };

        var entries = await baseQuery
            .Skip((page - 1) * pageSize)
            .Take(pageSize)
            .ToListAsync();

        return new ColorSearchResult
        {
            Entries = entries,
            Total = total,
            Page = page,
            PageSize = pageSize,
            TotalPages = Math.Max(1, (int)Math.Ceiling(total / (double)pageSize))
        };
    }

    private static readonly Regex HexPattern = new(@"^#[0-9A-Fa-f]{6}$");

    // Hex may be a comma-separated list for multi-color filaments (e.g. "Black+Gold") — every
    // part must still be a valid 6-digit hex color.
    private static string? Validate(FilamentColorInput input)
    {
        if (string.IsNullOrWhiteSpace(input.Name)) return "Name is required";
        if (string.IsNullOrWhiteSpace(input.Hex)) return "Hex is required";

        var parts = input.Hex.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        if (parts.Length == 0 || parts.Any(p => !HexPattern.IsMatch(p))) return "invalid_hex";

        return null;
    }

    public async Task<FilamentColorResult> CreateAsync(FilamentColorInput input)
    {
        var error = Validate(input);
        if (error is not null) return new FilamentColorResult { Ok = false, Error = error };

        if (await _db.FilamentColors.AnyAsync(c => c.Name == input.Name))
            return new FilamentColorResult { Ok = false, Error = "duplicate" };

        var color = new FilamentColor { Name = input.Name, Hex = input.Hex };
        _db.FilamentColors.Add(color);
        await _db.SaveChangesAsync();

        return new FilamentColorResult { Ok = true, Color = color };
    }

    public async Task<FilamentColorResult> UpdateAsync(int id, FilamentColorInput input)
    {
        var error = Validate(input);
        if (error is not null) return new FilamentColorResult { Ok = false, Error = error };

        if (await _db.FilamentColors.AnyAsync(c => c.Name == input.Name && c.Id != id))
            return new FilamentColorResult { Ok = false, Error = "duplicate" };

        var color = await _db.FilamentColors.FindAsync(id);
        if (color is null) throw new InvalidOperationException("Color not found");

        color.Name = input.Name;
        color.Hex = input.Hex;
        await _db.SaveChangesAsync();

        return new FilamentColorResult { Ok = true, Color = color };
    }

    public async Task DeleteAsync(int id)
    {
        var color = await _db.FilamentColors.FindAsync(id);
        if (color is null) throw new InvalidOperationException("Color not found");

        _db.FilamentColors.Remove(color);
        await _db.SaveChangesAsync();
    }
}
