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
    private const long MaxBytes = 100 * 1024 * 1024;

    private readonly ProjectService _projectService;
    private readonly HttpClient _httpClient;
    private readonly string _storageDir;

    public ProjectUploadService(ProjectService projectService, HttpClient httpClient)
    {
        _projectService = projectService;
        _httpClient = httpClient;
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
        return await SaveBytesAsync(ms.ToArray(), originalFileName);
    }

    // Generic URL fetch (docs/adr/0023) — any direct link to a .3mf file, including a MakerWorld
    // download link copied manually. Auto-resolving a MakerWorld page URL itself is deferred:
    // that needs their unofficial frontend API, not a direct file link.
    public async Task<ProjectResult> SaveFromUrlAsync(string url)
    {
        if (!Uri.TryCreate(url, UriKind.Absolute, out var uri) || (uri.Scheme != "http" && uri.Scheme != "https"))
            return new ProjectResult { Ok = false, Error = "Enter a valid http(s) URL." };

        HttpResponseMessage response;
        try
        {
            response = await _httpClient.GetAsync(uri);
        }
        catch (Exception ex)
        {
            return new ProjectResult { Ok = false, Error = $"Fetch failed: {ex.Message}" };
        }

        if (!response.IsSuccessStatusCode)
            return new ProjectResult { Ok = false, Error = $"Fetch failed: HTTP {(int)response.StatusCode}" };

        var bytes = await response.Content.ReadAsByteArrayAsync();
        if (bytes.Length == 0)
            return new ProjectResult { Ok = false, Error = "Downloaded file was empty." };
        if (bytes.Length > MaxBytes)
            return new ProjectResult { Ok = false, Error = "Downloaded file is too large (over 100 MB)." };

        var fileName = Path.GetFileName(uri.LocalPath);
        if (string.IsNullOrWhiteSpace(fileName) || !fileName.EndsWith(".3mf", StringComparison.OrdinalIgnoreCase))
            fileName = "download.3mf";

        return await SaveBytesAsync(bytes, fileName);
    }

    private async Task<ProjectResult> SaveBytesAsync(byte[] bytes, string displayName)
    {
        var hash = Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
        var storedPath = Path.Combine(_storageDir, $"{hash}.3mf");
        if (!File.Exists(storedPath))
            await File.WriteAllBytesAsync(storedPath, bytes);

        return await _projectService.UpsertByPathAsync(storedPath, displayName);
    }
}
