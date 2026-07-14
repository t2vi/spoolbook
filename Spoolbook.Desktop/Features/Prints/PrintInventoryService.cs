using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;

using Spoolbook.Desktop.Common;
namespace Spoolbook.Desktop.Features.Prints;

public enum PrintSortColumn { StartedAt, Status, Printer }

public class PrintInventoryQuery
{
    public string? Brand { get; set; }
    public string? Material { get; set; }
    public PrintStatus? Status { get; set; }
    public PrintSortColumn Sort { get; set; } = PrintSortColumn.StartedAt;
    public SortOrder Order { get; set; } = SortOrder.Desc;
    public int Page { get; set; } = 1;
    public int PageSize { get; set; } = 20;
}

public class PrintInventoryResult
{
    public required List<Print> Prints { get; init; }
    public int Total { get; init; }
    public int Page { get; init; }
    public int PageSize { get; init; }
    public int TotalPages { get; init; }
}

public class PrintInventoryService
{
    private readonly SpoolbookDbContext _db;

    public PrintInventoryService(SpoolbookDbContext db)
    {
        _db = db;
    }

    public async Task<PrintInventoryResult> ListAsync(PrintInventoryQuery query)
    {
        var page = Math.Max(1, query.Page);
        var pageSize = query.PageSize;

        var baseQuery = _db.Prints
            .Include(p => p.Profile)
            .Include(p => p.Spool).ThenInclude(s => s!.Filament)
            .AsQueryable();

        if (!string.IsNullOrEmpty(query.Brand))
            baseQuery = baseQuery.Where(p => p.Spool!.Filament!.Brand == query.Brand);
        if (!string.IsNullOrEmpty(query.Material))
            baseQuery = baseQuery.Where(p => p.Spool!.Filament!.Material == query.Material);
        if (query.Status is not null)
            baseQuery = baseQuery.Where(p => p.Status == query.Status);

        var total = await baseQuery.CountAsync();

        baseQuery = ApplySort(baseQuery, query.Sort, query.Order);

        var prints = await baseQuery
            .Skip((page - 1) * pageSize)
            .Take(pageSize)
            .ToListAsync();

        return new PrintInventoryResult
        {
            Prints = prints,
            Total = total,
            Page = page,
            PageSize = pageSize,
            TotalPages = Math.Max(1, (int)Math.Ceiling(total / (double)pageSize))
        };
    }

    private static IQueryable<Print> ApplySort(IQueryable<Print> query, PrintSortColumn sort, SortOrder order) =>
        sort switch
        {
            PrintSortColumn.Status => order == SortOrder.Desc
                ? query.OrderByDescending(p => p.Status)
                : query.OrderBy(p => p.Status),
            PrintSortColumn.Printer => order == SortOrder.Desc
                ? query.OrderByDescending(p => p.Printer)
                : query.OrderBy(p => p.Printer),
            _ => order == SortOrder.Desc
                ? query.OrderByDescending(p => p.StartedAt)
                : query.OrderBy(p => p.StartedAt)
        };
}
