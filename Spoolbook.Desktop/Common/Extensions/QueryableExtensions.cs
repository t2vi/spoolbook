using Microsoft.EntityFrameworkCore;

namespace Spoolbook.Desktop.Common;

public record PagedResult<T>(List<T> Items, int Total, int Page, int PageSize, int TotalPages);

public static class QueryableExtensions
{
    public static async Task<PagedResult<T>> ToPagedListAsync<T>(this IQueryable<T> query, int page, int pageSize)
    {
        var pageNumber = Math.Max(1, page);
        var total = await query.CountAsync();
        var items = await query
            .Skip((pageNumber - 1) * pageSize)
            .Take(pageSize)
            .ToListAsync();

        return new PagedResult<T>(items, total, pageNumber, pageSize, Math.Max(1, (int)Math.Ceiling(total / (double)pageSize)));
    }
}
