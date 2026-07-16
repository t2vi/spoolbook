using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;

using Spoolbook.Desktop.Common;
namespace Spoolbook.Desktop.Features.Profiles;

public enum ProfileScope { Generic, Spool }
public enum ProfileSortColumn { Filament, Name, NozzleTempC, CreatedAt }

public class ProfileInventoryQuery
{
    public string? Brand { get; set; }
    public string? Material { get; set; }
    public ProfileScope? Scope { get; set; }
    public ProfileSortColumn Sort { get; set; } = ProfileSortColumn.Filament;
    public SortOrder Order { get; set; } = SortOrder.Asc;
    public int Page { get; set; } = 1;
    public int PageSize { get; set; } = 20;
}

public class ProfileInventoryResult
{
    public required List<PrintProfile> Profiles { get; init; }
    public int Total { get; init; }
    public int Page { get; init; }
    public int PageSize { get; init; }
    public int TotalPages { get; init; }
}

public class ProfileInventoryService
{
    private readonly SpoolbookDbContext _db;

    public ProfileInventoryService(SpoolbookDbContext db)
    {
        _db = db;
    }

    public async Task<ProfileInventoryResult> ListAsync(ProfileInventoryQuery query)
    {
        var baseQuery = _db.PrintProfiles.Include(p => p.Filament).Where(p => p.IsCurrentVersion);

        if (!string.IsNullOrEmpty(query.Brand))
            baseQuery = baseQuery.Where(p => p.Filament!.Brand == query.Brand);
        if (!string.IsNullOrEmpty(query.Material))
            baseQuery = baseQuery.Where(p => p.Filament!.Material == query.Material);

        baseQuery = query.Scope switch
        {
            ProfileScope.Generic => baseQuery.Where(p => p.SpoolId == null),
            ProfileScope.Spool => baseQuery.Where(p => p.SpoolId != null),
            _ => baseQuery
        };

        baseQuery = query.Sort switch
        {
            ProfileSortColumn.Name => query.Order == SortOrder.Desc
                ? baseQuery.OrderByDescending(p => p.Name)
                : baseQuery.OrderBy(p => p.Name),
            ProfileSortColumn.NozzleTempC => query.Order == SortOrder.Desc
                ? baseQuery.OrderByDescending(p => p.NozzleTempC)
                : baseQuery.OrderBy(p => p.NozzleTempC),
            ProfileSortColumn.CreatedAt => query.Order == SortOrder.Desc
                ? baseQuery.OrderByDescending(p => p.CreatedAt)
                : baseQuery.OrderBy(p => p.CreatedAt),
            _ => query.Order == SortOrder.Desc
                ? baseQuery.OrderByDescending(p => p.Filament!.Brand).ThenByDescending(p => p.Filament!.Material).ThenByDescending(p => p.CreatedAt)
                : baseQuery.OrderBy(p => p.Filament!.Brand).ThenBy(p => p.Filament!.Material).ThenBy(p => p.CreatedAt)
        };

        var paged = await baseQuery.ToPagedListAsync(query.Page, query.PageSize);

        return new ProfileInventoryResult
        {
            Profiles = paged.Items,
            Total = paged.Total,
            Page = paged.Page,
            PageSize = paged.PageSize,
            TotalPages = paged.TotalPages
        };
    }
}
