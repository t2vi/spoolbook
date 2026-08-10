using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;
namespace Spoolbook.Desktop.Features.Prints;

public class ReadingInput
{
    public decimal? NozzleTempC { get; set; }
    public decimal? BedTempC { get; set; }
    public decimal? ChamberTempC { get; set; }
    public string? AmsSlot { get; set; }
    public int? ProgressPct { get; set; }
}

// Buffers live MQTT telemetry into Jobs/Readings and matches them to the retrospective Print
// form afterward. See docs/adr/0017-printer-telemetry-mqtt-job-print-separation.md — the actual
// MQTT wire client (subscribing to a real printer) is a separate slice; this is the pure logic
// behind it, exercised directly by tests rather than over a network.
public class PrinterTelemetryService
{
    private readonly SpoolbookDbContext _db;

    public PrinterTelemetryService(SpoolbookDbContext db)
    {
        _db = db;
    }

    public async Task RecordReadingAsync(int printerId, string externalJobId, ReadingInput input, DateTime? at = null)
    {
        var recordedAt = at ?? DateTime.UtcNow;

        var job = await _db.PrinterJobs
            .FirstOrDefaultAsync(j => j.PrinterId == printerId && j.ExternalJobId == externalJobId && j.EndedAt == null);

        if (job is null)
        {
            job = new PrinterJob { PrinterId = printerId, ExternalJobId = externalJobId, StartedAt = recordedAt };
            _db.PrinterJobs.Add(job);
        }

        _db.PrinterReadings.Add(new PrinterReading
        {
            PrinterJob = job,
            RecordedAt = recordedAt,
            NozzleTempC = input.NozzleTempC,
            BedTempC = input.BedTempC,
            ChamberTempC = input.ChamberTempC,
            AmsSlot = input.AmsSlot,
            ProgressPct = input.ProgressPct
        });

        await _db.SaveChangesAsync();
    }

    public async Task EndJobAsync(int printerId, string externalJobId, DateTime? at = null)
    {
        var job = await _db.PrinterJobs
            .FirstOrDefaultAsync(j => j.PrinterId == printerId && j.ExternalJobId == externalJobId && j.EndedAt == null);
        if (job is null) return;

        job.EndedAt = at ?? DateTime.UtcNow;
        await _db.SaveChangesAsync();
    }

    // Auto-match candidate for the retrospective Print form: closest unattached Job for this
    // Printer by start time, shown as a dismissible chip rather than attached silently.
    public async Task<PrinterJob?> FindMatchForPrintAsync(int printerId, DateTime printStartedAt)
    {
        var candidates = await _db.PrinterJobs
            .Where(j => j.PrinterId == printerId && j.PrintId == null)
            .ToListAsync();

        return candidates
            .OrderBy(j => Math.Abs((j.StartedAt - printStartedAt).Ticks))
            .FirstOrDefault();
    }

    public async Task AttachJobToPrintAsync(int jobId, int printId)
    {
        var job = await _db.PrinterJobs.FindAsync(jobId);
        if (job is null) throw new InvalidOperationException("Job not found");

        job.PrintId = printId;
        await _db.SaveChangesAsync();
    }

    // Unattached Jobs (and their Readings, via cascade) older than the cutoff are discarded —
    // ADR-0017's 7-day retention window. Attached Jobs are kept regardless of age.
    public async Task PurgeUnattachedJobsOlderThanAsync(DateTime cutoff)
    {
        var stale = await _db.PrinterJobs
            .Where(j => j.PrintId == null && j.StartedAt < cutoff)
            .ToListAsync();

        _db.PrinterJobs.RemoveRange(stale);
        await _db.SaveChangesAsync();
    }
}
