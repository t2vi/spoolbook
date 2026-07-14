using Microsoft.Data.Sqlite;
using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;
namespace Spoolbook.Desktop.Tests;

public static class TestDbFactory
{
    public static SpoolbookDbContext Create()
    {
        var connection = new SqliteConnection("DataSource=:memory:");
        connection.Open();

        var options = new DbContextOptionsBuilder<SpoolbookDbContext>()
            .UseSqlite(connection)
            .Options;

        var context = new SpoolbookDbContext(options);
        context.Database.EnsureCreated();
        return context;
    }
}
