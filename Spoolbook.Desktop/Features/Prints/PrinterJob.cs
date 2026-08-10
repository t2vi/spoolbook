using Spoolbook.Desktop.Features.Settings.Printers;
namespace Spoolbook.Desktop.Features.Prints;

// The printer's own live, in-progress instance — CONTEXT.md's "Job", distinct from Print (the
// retrospective record). See docs/adr/0017-printer-telemetry-mqtt-job-print-separation.md.
public class PrinterJob
{
    public int Id { get; set; }
    public int PrinterId { get; set; }
    public Printer? Printer { get; set; }
    public required string ExternalJobId { get; set; }
    public DateTime StartedAt { get; set; }
    public DateTime? EndedAt { get; set; }
    public int? PrintId { get; set; }
    public Print? Print { get; set; }
    public List<PrinterReading> Readings { get; set; } = [];
}

public class PrinterReading
{
    public int Id { get; set; }
    public int PrinterJobId { get; set; }
    public PrinterJob? PrinterJob { get; set; }
    public DateTime RecordedAt { get; set; }
    public decimal? NozzleTempC { get; set; }
    public decimal? BedTempC { get; set; }
    public decimal? ChamberTempC { get; set; }
    public string? AmsSlot { get; set; }
    public int? ProgressPct { get; set; }
}
