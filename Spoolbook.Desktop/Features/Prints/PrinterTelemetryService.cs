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

        var isNewJob = job is null;
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

        // Auto-create-on-send (docs/adr/0017's 2026-08-14 addendum): a brand-new Job attaches
        // straight to the printer's open (InProgress, not yet attached) Print instead of waiting
        // for the retrospective dismissible-chip match — there's no ambiguity to confirm, since
        // the Print was created moments before by the same "send" action that produced this Job.
        // Only checked for a *new* job — an already-attached job must never be reassigned just
        // because a second InProgress Print for the same printer shows up later.
        if (isNewJob)
        {
            var attachedPrintIds = _db.PrinterJobs.Where(j => j.PrintId != null).Select(j => j.PrintId);
            var openPrint = await _db.Prints
                .Where(p => p.PrinterId == printerId && p.Status == PrintStatus.InProgress && !attachedPrintIds.Contains(p.Id))
                .OrderByDescending(p => p.StartedAt)
                .FirstOrDefaultAsync();
            if (openPrint is not null)
                job.PrintId = openPrint.Id;
        }

        await _db.SaveChangesAsync();
    }

    public async Task EndJobAsync(int printerId, string externalJobId, string? terminalGcodeState = null, DateTime? at = null)
    {
        var job = await _db.PrinterJobs
            .FirstOrDefaultAsync(j => j.PrinterId == printerId && j.ExternalJobId == externalJobId && j.EndedAt == null);
        if (job is null) return;

        var endedAt = at ?? DateTime.UtcNow;
        job.EndedAt = endedAt;

        if (job.PrintId is { } printId)
        {
            var print = await _db.Prints.FindAsync(printId);
            if (print is not null && print.Status == PrintStatus.InProgress)
            {
                print.EndedAt = endedAt;
                print.Status = MapTerminalGcodeState(terminalGcodeState);
            }
        }

        await _db.SaveChangesAsync();
    }

    // FINISH/FAILED are unambiguous. Everything else (IDLE, or a delta message that omitted
    // gcode_state before this one) falls back to Partial rather than guessing — IDLE could mean
    // a dropped FINISH right before the idle snapshot, or the printer going idle after a
    // user-initiated Stop, and guessing wrong in either direction is worse than a review flag.
    private static PrintStatus MapTerminalGcodeState(string? gcodeState) => gcodeState switch
    {
        "FINISH" => PrintStatus.Success,
        "FAILED" => PrintStatus.Failed,
        _ => PrintStatus.Partial
    };

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
