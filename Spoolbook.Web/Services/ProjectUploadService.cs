using System.Security.Cryptography;
using Spoolbook.Desktop.Features.Prints;

namespace Spoolbook.Web.Services;

// Desktop's Project model links a .3mf by local filesystem path (ADR-0015) and detects drift
// via mtime+size, since the file lives outside the app's control. Web has no such path — the
// browser only hands over bytes — so uploads get a permanent server-side copy instead, named
// by content hash. That naturally dedupes re-uploads of the same file (ProjectService.
// UpsertByPathAsync finds the existing Project row for that path) and means drift can never
// happen, since nothing but this service ever writes into the storage directory.
public class ProjectUploadService
{
    private readonly ProjectService _projectService;
    private readonly string _storageDir;

    public ProjectUploadService(ProjectService projectService)
    {
        _projectService = projectService;
        var dataDir = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
            "spoolbook");
        _storageDir = Path.Combine(dataDir, "projects");
        Directory.CreateDirectory(_storageDir);
    }

    public async Task<ProjectResult> SaveAndUpsertAsync(Stream content, string originalFileName)
    {
        using var ms = new MemoryStream();
        await content.CopyToAsync(ms);
        var bytes = ms.ToArray();

        var hash = Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
        var storedPath = Path.Combine(_storageDir, $"{hash}.3mf");
        if (!File.Exists(storedPath))
            await File.WriteAllBytesAsync(storedPath, bytes);

        return await _projectService.UpsertByPathAsync(storedPath, originalFileName);
    }
}
