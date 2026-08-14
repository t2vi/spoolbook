using System.IO.Compression;
using System.Net.Http.Headers;
using Spoolbook.Desktop.Features.Prints;
using Spoolbook.Desktop.Features.Profiles;

namespace Spoolbook.Web.Services;

public class ReslicingResult
{
    public bool Ok { get; init; }
    public Project? Project { get; init; }
    public string? Error { get; init; }
}

// Re-slices a Project's .3mf with a chosen PrintProfile's settings patched in, via the
// standalone OrcaSlicer wrapper (slicer-service/, re-slicing session 2026-08-14). The file
// manipulation (patching project_settings.config, splicing it into a copy of the .3mf) lives
// here rather than in the wrapper — ProfileConfigPatcher owns the field mapping, and the wrapper
// stays a dumb single-file-in/single-file-out executor with zero domain logic.
public class ReslicingService
{
    private readonly HttpClient _httpClient;
    private readonly ProjectUploadService _uploadService;

    public ReslicingService(HttpClient httpClient, ProjectUploadService uploadService)
    {
        _httpClient = httpClient;
        _uploadService = uploadService;
    }

    public async Task<ReslicingResult> ReslicingAsync(Project project, PrintProfile profile, CancellationToken ct = default)
    {
        string patchedFilePath;
        try
        {
            patchedFilePath = BuildPatchedProjectFile(project, profile);
        }
        catch (Exception ex)
        {
            return new ReslicingResult { Ok = false, Error = $"Couldn't prepare project for re-slicing: {ex.Message}" };
        }

        try
        {
            byte[] slicedBytes;
            try
            {
                slicedBytes = await SliceAsync(patchedFilePath, ct);
            }
            catch (Exception ex)
            {
                return new ReslicingResult { Ok = false, Error = $"Re-slice failed: {ex.Message}" };
            }

            using var ms = new MemoryStream(slicedBytes);
            var result = await _uploadService.SaveAndUpsertAsync(ms, project.FileName);
            return new ReslicingResult { Ok = result.Ok, Project = result.Project, Error = result.Error };
        }
        finally
        {
            TryDelete(patchedFilePath);
        }
    }

    // Copies the original .3mf (never mutated in place — ProjectUploadService's storage is
    // content-hash-addressed, so the source file is effectively immutable) and swaps its
    // Metadata/project_settings.config entry for one patched with the chosen profile's settings.
    private static string BuildPatchedProjectFile(Project project, PrintProfile profile)
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"spoolbook-reslice-{Guid.NewGuid():N}.3mf");
        File.Copy(project.FilePath, tempPath);

        using var archive = ZipFile.Open(tempPath, ZipArchiveMode.Update);
        var entry = archive.GetEntry("Metadata/project_settings.config")
            ?? throw new InvalidOperationException("Project has no Metadata/project_settings.config — not a sliced export?");

        string originalJson;
        using (var reader = new StreamReader(entry.Open()))
            originalJson = reader.ReadToEnd();

        var patchedJson = ProfileConfigPatcher.Patch(originalJson, profile);

        entry.Delete();
        var newEntry = archive.CreateEntry("Metadata/project_settings.config");
        using var writer = new StreamWriter(newEntry.Open());
        writer.Write(patchedJson);

        return tempPath;
    }

    private async Task<byte[]> SliceAsync(string filePath, CancellationToken ct)
    {
        using var content = new MultipartFormDataContent();
        await using var fileStream = File.OpenRead(filePath);
        using var fileContent = new StreamContent(fileStream);
        fileContent.Headers.ContentType = new MediaTypeHeaderValue("application/octet-stream");
        content.Add(fileContent, "project", Path.GetFileName(filePath));

        var response = await _httpClient.PostAsync("/slice", content, ct);
        if (!response.IsSuccessStatusCode)
        {
            var detail = await response.Content.ReadAsStringAsync(ct);
            throw new InvalidOperationException($"slicer-service returned {(int)response.StatusCode}: {detail}");
        }

        return await response.Content.ReadAsByteArrayAsync(ct);
    }

    private static void TryDelete(string path)
    {
        try { File.Delete(path); } catch { /* best effort cleanup */ }
    }
}
