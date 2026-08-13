namespace Spoolbook.Desktop.Features.Prints;

public class Project
{
    public int Id { get; set; }
    public required string FilePath { get; set; }
    public required string FileName { get; set; }
    public DateTime LastKnownWriteTimeUtc { get; set; }
    public long LastKnownFileSizeBytes { get; set; }
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;

    // sha256 of the 3D/3dmodel.model zip entry, computed once at creation — used to suggest a
    // version link on re-upload of a re-slice (docs/adr/0023). Null when the file wasn't a
    // readable .3mf zip at upload time.
    public string? MeshHash { get; set; }
    public int? PreviousVersionProjectId { get; set; }
    public int VersionNumber { get; set; } = 1;
    public bool IsCurrentVersion { get; set; } = true;
}
