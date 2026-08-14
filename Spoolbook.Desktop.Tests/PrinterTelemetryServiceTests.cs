using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;
using Spoolbook.Desktop.Features.Prints;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Settings.Printers;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Services.Weather;
namespace Spoolbook.Desktop.Tests;

// Covers the buffer/match/attach/purge logic from docs/adr/0017-printer-telemetry-mqtt-job-print-separation.md.
// The actual MQTT wire client (subscribing to a real Bambu P2S) is a separate, later slice —
// this is the pure, testable Job/Reading/Print logic that sits behind it.
public class PrinterTelemetryServiceTests
{
    private static async Task<int> SeedPrinterAsync(SpoolbookDbContext db)
    {
        var printerService = new PrinterService(db);
        var printer = await printerService.CreateAsync(new PrinterInput { Name = "Garage P2S" });
        return printer.Printer!.Id;
    }

    private static async Task<(int ProfileId, int SpoolId, int PrinterId)> SeedPrintDepsAsync(SpoolbookDbContext db, int printerId)
    {
        var filamentService = new FilamentService(db);
        var filament = await filamentService.CreateAsync(new FilamentInput { Brand = "Bambu Lab", Material = "PLA", Color = "Black" });
        var spoolService = new SpoolService(db);
        var spool = await spoolService.CreateSpoolAsync(filament.Entry!.Id, new SpoolInput());
        var profileService = new PrintProfileService(db);
        var profile = await profileService.CreateProfileAsync(filament.Entry.Id, new ProfileInput { Name = "Standard", NozzleTempC = "230" });
        return (profile.Profile!.Id, spool.Spool!.Id, printerId);
    }

    [Fact]
    public async Task RecordReadingAsync_CreatesNewJobOnFirstReadingForExternalId()
    {
        using var db = TestDbFactory.Create();
        var printerId = await SeedPrinterAsync(db);
        var service = new PrinterTelemetryService(db);

        await service.RecordReadingAsync(printerId, "job-1", new ReadingInput { NozzleTempC = 245, BedTempC = 70 });

        var jobs = await db.PrinterJobs.Include(j => j.Readings).ToListAsync();
        Assert.Single(jobs);
        Assert.Equal("job-1", jobs[0].ExternalJobId);
        Assert.Single(jobs[0].Readings);
        Assert.Equal(245, jobs[0].Readings[0].NozzleTempC);
    }

    [Fact]
    public async Task RecordReadingAsync_AppendsReadingToExistingActiveJob()
    {
        using var db = TestDbFactory.Create();
        var printerId = await SeedPrinterAsync(db);
        var service = new PrinterTelemetryService(db);

        await service.RecordReadingAsync(printerId, "job-1", new ReadingInput { NozzleTempC = 245 });
        await service.RecordReadingAsync(printerId, "job-1", new ReadingInput { NozzleTempC = 248 });

        var jobs = await db.PrinterJobs.Include(j => j.Readings).ToListAsync();
        Assert.Single(jobs);
        Assert.Equal(2, jobs[0].Readings.Count);
    }

    [Fact]
    public async Task RecordReadingAsync_DifferentExternalIdsCreateSeparateJobs()
    {
        using var db = TestDbFactory.Create();
        var printerId = await SeedPrinterAsync(db);
        var service = new PrinterTelemetryService(db);

        await service.RecordReadingAsync(printerId, "job-1", new ReadingInput { NozzleTempC = 245 });
        await service.RecordReadingAsync(printerId, "job-2", new ReadingInput { NozzleTempC = 245 });

        var jobs = await db.PrinterJobs.ToListAsync();
        Assert.Equal(2, jobs.Count);
    }

    [Fact]
    public async Task EndJobAsync_SetsEndedAt()
    {
        using var db = TestDbFactory.Create();
        var printerId = await SeedPrinterAsync(db);
        var service = new PrinterTelemetryService(db);
        await service.RecordReadingAsync(printerId, "job-1", new ReadingInput { NozzleTempC = 245 });

        await service.EndJobAsync(printerId, "job-1");

        var job = await db.PrinterJobs.FirstAsync();
        Assert.NotNull(job.EndedAt);
    }

    private static async Task<int> SeedInProgressPrintAsync(SpoolbookDbContext db, int profileId, int spoolId, int printerId, DateTime startedAt)
    {
        var print = new Print
        {
            ProfileId = profileId,
            SpoolId = spoolId,
            PrinterId = printerId,
            StartedAt = startedAt,
            EndedAt = null,
            Status = PrintStatus.InProgress
        };
        db.Prints.Add(print);
        await db.SaveChangesAsync();
        return print.Id;
    }

    [Fact]
    public async Task RecordReadingAsync_AutoAttachesNewJobToOpenInProgressPrintForSamePrinter()
    {
        using var db = TestDbFactory.Create();
        var printerId = await SeedPrinterAsync(db);
        var (profileId, spoolId, _) = await SeedPrintDepsAsync(db, printerId);
        var printId = await SeedInProgressPrintAsync(db, profileId, spoolId, printerId, new DateTime(2026, 1, 1, 8, 0, 0));
        var service = new PrinterTelemetryService(db);

        await service.RecordReadingAsync(printerId, "job-1", new ReadingInput { NozzleTempC = 245 });

        var job = await db.PrinterJobs.FirstAsync();
        Assert.Equal(printId, job.PrintId);
    }

    [Fact]
    public async Task RecordReadingAsync_DoesNotReattachOnSubsequentReadingsForSameJob()
    {
        using var db = TestDbFactory.Create();
        var printerId = await SeedPrinterAsync(db);
        var (profileId, spoolId, _) = await SeedPrintDepsAsync(db, printerId);
        var printId = await SeedInProgressPrintAsync(db, profileId, spoolId, printerId, new DateTime(2026, 1, 1, 8, 0, 0));
        var service = new PrinterTelemetryService(db);
        await service.RecordReadingAsync(printerId, "job-1", new ReadingInput { NozzleTempC = 245 });

        // A second, unrelated InProgress print shows up before the next reading — the already-
        // attached job must not be reassigned to it.
        var secondPrintId = await SeedInProgressPrintAsync(db, profileId, spoolId, printerId, new DateTime(2026, 1, 1, 9, 0, 0));
        await service.RecordReadingAsync(printerId, "job-1", new ReadingInput { NozzleTempC = 248 });

        var job = await db.PrinterJobs.FirstAsync();
        Assert.Equal(printId, job.PrintId);
        Assert.NotEqual(secondPrintId, job.PrintId);
    }

    [Fact]
    public async Task RecordReadingAsync_DoesNotAutoAttachWhenNoInProgressPrintExists()
    {
        using var db = TestDbFactory.Create();
        var printerId = await SeedPrinterAsync(db);
        var service = new PrinterTelemetryService(db);

        await service.RecordReadingAsync(printerId, "job-1", new ReadingInput { NozzleTempC = 245 });

        var job = await db.PrinterJobs.FirstAsync();
        Assert.Null(job.PrintId);
    }

    [Theory]
    [InlineData("FINISH", PrintStatus.Success)]
    [InlineData("FAILED", PrintStatus.Failed)]
    [InlineData("IDLE", PrintStatus.Partial)]
    [InlineData(null, PrintStatus.Partial)]
    public async Task EndJobAsync_SetsAttachedPrintStatusFromTerminalGcodeState(string? gcodeState, PrintStatus expected)
    {
        using var db = TestDbFactory.Create();
        var printerId = await SeedPrinterAsync(db);
        var (profileId, spoolId, _) = await SeedPrintDepsAsync(db, printerId);
        var printId = await SeedInProgressPrintAsync(db, profileId, spoolId, printerId, new DateTime(2026, 1, 1, 8, 0, 0));
        var service = new PrinterTelemetryService(db);
        await service.RecordReadingAsync(printerId, "job-1", new ReadingInput { NozzleTempC = 245 });

        await service.EndJobAsync(printerId, "job-1", gcodeState);

        var print = await db.Prints.FindAsync(printId);
        Assert.Equal(expected, print!.Status);
        Assert.NotNull(print.EndedAt);
    }

    [Fact]
    public async Task EndJobAsync_DoesNotTouchPrintStatus_WhenJobHasNoAttachedPrint()
    {
        using var db = TestDbFactory.Create();
        var printerId = await SeedPrinterAsync(db);
        var service = new PrinterTelemetryService(db);
        await service.RecordReadingAsync(printerId, "job-1", new ReadingInput { NozzleTempC = 245 });

        await service.EndJobAsync(printerId, "job-1", "FINISH");

        var job = await db.PrinterJobs.FirstAsync();
        Assert.NotNull(job.EndedAt);
        Assert.Null(job.PrintId);
    }

    [Fact]
    public async Task FindMatchForPrintAsync_ReturnsClosestUnattachedJobByStartTime()
    {
        using var db = TestDbFactory.Create();
        var printerId = await SeedPrinterAsync(db);
        var service = new PrinterTelemetryService(db);
        await service.RecordReadingAsync(printerId, "job-far", new ReadingInput { NozzleTempC = 245 }, at: new DateTime(2026, 1, 1, 6, 0, 0));
        await service.RecordReadingAsync(printerId, "job-close", new ReadingInput { NozzleTempC = 245 }, at: new DateTime(2026, 1, 1, 8, 0, 0));

        var match = await service.FindMatchForPrintAsync(printerId, new DateTime(2026, 1, 1, 8, 5, 0));

        Assert.NotNull(match);
        Assert.Equal("job-close", match!.ExternalJobId);
    }

    [Fact]
    public async Task FindMatchForPrintAsync_ExcludesAlreadyAttachedJobs()
    {
        using var db = TestDbFactory.Create();
        var printerId = await SeedPrinterAsync(db);
        var (profileId, spoolId, _) = await SeedPrintDepsAsync(db, printerId);
        var printService = new PrintService(db, new FakeWeatherService());
        var service = new PrinterTelemetryService(db);
        await service.RecordReadingAsync(printerId, "job-1", new ReadingInput { NozzleTempC = 245 }, at: new DateTime(2026, 1, 1, 8, 0, 0));
        var job = await db.PrinterJobs.FirstAsync();
        var print = await printService.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0), Status = PrintStatus.Success
        });
        await service.AttachJobToPrintAsync(job.Id, print.Print!.Id);

        var match = await service.FindMatchForPrintAsync(printerId, new DateTime(2026, 1, 1, 8, 0, 0));

        Assert.Null(match);
    }

    [Fact]
    public async Task FindMatchForPrintAsync_ReturnsNullWhenNoUnattachedJobsExist()
    {
        using var db = TestDbFactory.Create();
        var printerId = await SeedPrinterAsync(db);
        var service = new PrinterTelemetryService(db);

        var match = await service.FindMatchForPrintAsync(printerId, DateTime.UtcNow);

        Assert.Null(match);
    }

    [Fact]
    public async Task AttachJobToPrintAsync_SetsPrintId()
    {
        using var db = TestDbFactory.Create();
        var printerId = await SeedPrinterAsync(db);
        var (profileId, spoolId, _) = await SeedPrintDepsAsync(db, printerId);
        var printService = new PrintService(db, new FakeWeatherService());
        var service = new PrinterTelemetryService(db);
        await service.RecordReadingAsync(printerId, "job-1", new ReadingInput { NozzleTempC = 245 });
        var job = await db.PrinterJobs.FirstAsync();
        var print = await printService.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0), Status = PrintStatus.Success
        });

        await service.AttachJobToPrintAsync(job.Id, print.Print!.Id);

        var updated = await db.PrinterJobs.FindAsync(job.Id);
        Assert.Equal(print.Print.Id, updated!.PrintId);
    }

    [Fact]
    public async Task PurgeUnattachedJobsOlderThanAsync_RemovesOldUnattachedJobsAndTheirReadings()
    {
        using var db = TestDbFactory.Create();
        var printerId = await SeedPrinterAsync(db);
        var service = new PrinterTelemetryService(db);
        await service.RecordReadingAsync(printerId, "old-job", new ReadingInput { NozzleTempC = 245 }, at: new DateTime(2026, 1, 1, 8, 0, 0));

        await service.PurgeUnattachedJobsOlderThanAsync(new DateTime(2026, 1, 5, 0, 0, 0));

        Assert.Empty(await db.PrinterJobs.ToListAsync());
        Assert.Empty(await db.PrinterReadings.ToListAsync());
    }

    [Fact]
    public async Task PurgeUnattachedJobsOlderThanAsync_KeepsAttachedJobsRegardlessOfAge()
    {
        using var db = TestDbFactory.Create();
        var printerId = await SeedPrinterAsync(db);
        var (profileId, spoolId, _) = await SeedPrintDepsAsync(db, printerId);
        var printService = new PrintService(db, new FakeWeatherService());
        var service = new PrinterTelemetryService(db);
        await service.RecordReadingAsync(printerId, "old-job", new ReadingInput { NozzleTempC = 245 }, at: new DateTime(2026, 1, 1, 8, 0, 0));
        var job = await db.PrinterJobs.FirstAsync();
        var print = await printService.CreateAsync(profileId, spoolId, printerId, new PrintInput
        {
            StartedAt = new DateTime(2026, 1, 1, 8, 0, 0), EndedAt = new DateTime(2026, 1, 1, 10, 0, 0), Status = PrintStatus.Success
        });
        await service.AttachJobToPrintAsync(job.Id, print.Print!.Id);

        await service.PurgeUnattachedJobsOlderThanAsync(new DateTime(2026, 1, 5, 0, 0, 0));

        Assert.Single(await db.PrinterJobs.ToListAsync());
    }

    [Fact]
    public async Task PurgeUnattachedJobsOlderThanAsync_KeepsRecentUnattachedJobs()
    {
        using var db = TestDbFactory.Create();
        var printerId = await SeedPrinterAsync(db);
        var service = new PrinterTelemetryService(db);
        await service.RecordReadingAsync(printerId, "recent-job", new ReadingInput { NozzleTempC = 245 }, at: new DateTime(2026, 1, 6, 8, 0, 0));

        await service.PurgeUnattachedJobsOlderThanAsync(new DateTime(2026, 1, 5, 0, 0, 0));

        Assert.Single(await db.PrinterJobs.ToListAsync());
    }
}
