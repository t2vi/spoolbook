namespace Spoolbook.Desktop.Features.Prints;

public class Project
{
    public int Id { get; set; }
    public required string FilePath { get; set; }
    public required string FileName { get; set; }
    public DateTime LastKnownWriteTimeUtc { get; set; }
    public long LastKnownFileSizeBytes { get; set; }
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
}
