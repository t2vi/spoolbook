using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;

using Spoolbook.Desktop.Common;
namespace Spoolbook.Desktop.Features.Spools;

public enum SpoolStatus { Unopened, Opened, Empty }
public enum SpoolSortColumn { Filament, PurchasedAt, OpenedAt, EmptiedAt }

public class SpoolInventoryQuery
{
    public string? Brand { get; set; }
    public string? Material { get; set; }
    public SpoolStatus? Status { get; set; }
    public DateOnly? PurchasedFrom { get; set; }
    public DateOnly? PurchasedTo { get; set; }
    public SpoolSortColumn Sort { get; set; } = SpoolSortColumn.Filament;
    public SortOrder Order { get; set; } = SortOrder.Asc;
    public int Page { get; set; } = 1;
    public int PageSize { get; set; } = 20;
}

public class SpoolInventoryResult
{
    public required List<Spool> Spools { get; init; }
    public int Total { get; init; }
    public int Page { get; init; }
    public int PageSize { get; init; }
    public int TotalPages { get; init; }
}

public class SpoolInventoryService
{
    private readonly SpoolbookDbContext _db;

    public SpoolInventoryService(SpoolbookDbContext db)
    {
        _db = db;
    }

    public async Task<SpoolInventoryResult> ListAsync(SpoolInventoryQuery query)
    {
        var page = Math.Max(1, query.Page);
        var pageSize = query.PageSize;

        var baseQuery = _db.Spools.Include(s => s.Filament).AsQueryable();

        if (!string.IsNullOrEmpty(query.Brand))
            baseQuery = baseQuery.Where(s => s.Filament!.Brand == query.Brand);
        if (!string.IsNullOrEmpty(query.Material))
            baseQuery = baseQuery.Where(s => s.Filament!.Material == query.Material);
        if (query.PurchasedFrom is not null)
            baseQuery = baseQuery.Where(s => s.PurchasedAt >= query.PurchasedFrom);
        if (query.PurchasedTo is not null)
            baseQuery = baseQuery.Where(s => s.PurchasedAt <= query.PurchasedTo);

        baseQuery = query.Status switch
        {
            SpoolStatus.Unopened => baseQuery.Where(s => s.OpenedAt == null),
            SpoolStatus.Opened => baseQuery.Where(s => s.OpenedAt != null && s.EmptiedAt == null),
            SpoolStatus.Empty => baseQuery.Where(s => s.EmptiedAt != null),
            _ => baseQuery
        };

        var total = await baseQuery.CountAsync();

        baseQuery = ApplySort(baseQuery, query.Sort, query.Order);

        var spools = await baseQuery
            .Skip((page - 1) * pageSize)
            .Take(pageSize)
            .ToListAsync();

        return new SpoolInventoryResult
        {
            Spools = spools,
            Total = total,
            Page = page,
            PageSize = pageSize,
            TotalPages = Math.Max(1, (int)Math.Ceiling(total / (double)pageSize))
        };
    }

    private static IQueryable<Spool> ApplySort(IQueryable<Spool> query, SpoolSortColumn sort, SortOrder order)
    {
        // Date columns: nulls always sort last, regardless of asc/desc.
        return sort switch
        {
            SpoolSortColumn.PurchasedAt => order == SortOrder.Desc
                ? query.OrderBy(s => s.PurchasedAt == null).ThenByDescending(s => s.PurchasedAt)
                : query.OrderBy(s => s.PurchasedAt == null).ThenBy(s => s.PurchasedAt),
            SpoolSortColumn.OpenedAt => order == SortOrder.Desc
                ? query.OrderBy(s => s.OpenedAt == null).ThenByDescending(s => s.OpenedAt)
                : query.OrderBy(s => s.OpenedAt == null).ThenBy(s => s.OpenedAt),
            SpoolSortColumn.EmptiedAt => order == SortOrder.Desc
                ? query.OrderBy(s => s.EmptiedAt == null).ThenByDescending(s => s.EmptiedAt)
                : query.OrderBy(s => s.EmptiedAt == null).ThenBy(s => s.EmptiedAt),
            _ => order == SortOrder.Desc
                ? query.OrderByDescending(s => s.Filament!.Brand).ThenByDescending(s => s.Filament!.Material).ThenByDescending(s => s.CreatedAt)
                : query.OrderBy(s => s.Filament!.Brand).ThenBy(s => s.Filament!.Material).ThenBy(s => s.CreatedAt)
        };
    }
}
