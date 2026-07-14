using Microsoft.EntityFrameworkCore;
using Microsoft.EntityFrameworkCore.Design;

namespace Spoolbook.Desktop.Data;

// Used by `dotnet ef migrations add` at design time only.
public class SpoolbookDbContextFactory : IDesignTimeDbContextFactory<SpoolbookDbContext>
{
    public SpoolbookDbContext CreateDbContext(string[] args)
    {
        var options = new DbContextOptionsBuilder<SpoolbookDbContext>()
            .UseSqlite("Data Source=design_time.db")
            .Options;

        return new SpoolbookDbContext(options);
    }
}
