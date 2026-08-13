using System.IO.Compression;
using System.Security.Cryptography;
using System.Xml.Linq;
using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;
namespace Spoolbook.Desktop.Features.Prints;

public enum ProjectFileStatus { Ok, Missing, Changed }

public class ProjectPlate
{
    public required string PlaterId { get; init; }
    public string? PlaterName { get; init; }
    public byte[]? ThumbnailBytes { get; init; }
}

public class ProjectResult
{
    public bool Ok { get; init; }
    public Project? Project { get; init; }
    public string? Error { get; init; }
}

public class ProjectService
{
    private readonly SpoolbookDbContext _db;

    public ProjectService(SpoolbookDbContext db)
    {
        _db = db;
    }

    public async Task<List<Project>> ListAsync() =>
        await _db.Projects.OrderBy(p => p.FileName).ToListAsync();

    // displayName overrides the on-disk filename — needed for Web uploads, which store the
    // file under a content-hash-derived path (see Spoolbook.Web's ProjectUploadService) and
    // would otherwise show that hash instead of the name the user actually uploaded.
    public async Task<ProjectResult> UpsertByPathAsync(string filePath, string? displayName = null)
    {
        var info = new FileInfo(filePath);
        if (!info.Exists)
            return new ProjectResult { Ok = false, Error = "file_not_found" };

        var project = await _db.Projects.FirstOrDefaultAsync(p => p.FilePath == filePath);
        if (project is null)
        {
            project = new Project { FilePath = filePath, FileName = displayName ?? info.Name, MeshHash = ComputeMeshHash(filePath) };
            _db.Projects.Add(project);
        }

        project.LastKnownWriteTimeUtc = info.LastWriteTimeUtc;
        project.LastKnownFileSizeBytes = info.Length;
        await _db.SaveChangesAsync();

        return new ProjectResult { Ok = true, Project = project };
    }

    public async Task<ProjectResult> DeleteAsync(int id)
    {
        var project = await _db.Projects.FindAsync(id);
        if (project is null) throw new InvalidOperationException("Project not found");

        if (await _db.Prints.AnyAsync(p => p.ProjectId == id))
            return new ProjectResult { Ok = false, Error = "has_prints" };

        _db.Projects.Remove(project);
        await _db.SaveChangesAsync();

        return new ProjectResult { Ok = true };
    }

    // sha256 of just the mesh geometry entry, not the whole file — a re-slice of the same design
    // changes settings/thumbnails but usually not this. Byte-exact, not canonicalized (docs/adr/0023).
    // Returns null for anything that isn't a readable .3mf zip with that entry, rather than throwing.
    public static string? ComputeMeshHash(string filePath)
    {
        try
        {
            using var archive = ZipFile.OpenRead(filePath);
            var meshEntry = archive.GetEntry("3D/3dmodel.model");
            if (meshEntry is null) return null;

            using var meshStream = meshEntry.Open();
            return Convert.ToHexString(SHA256.HashData(meshStream)).ToLowerInvariant();
        }
        catch (InvalidDataException)
        {
            return null;
        }
    }

    // Suggests an existing Project this upload might be a new version of — mesh hash first
    // (strongest signal), filename as a weaker fallback for when re-export reformatting misses
    // the mesh hash. Caller still confirms explicitly (docs/adr/0023) — this only pre-selects.
    public async Task<Project?> FindVersionCandidateAsync(string? meshHash, string fileName)
    {
        if (!string.IsNullOrEmpty(meshHash))
        {
            var byMesh = await _db.Projects.FirstOrDefaultAsync(p => p.MeshHash == meshHash);
            if (byMesh is not null) return byMesh;
        }

        return await _db.Projects.FirstOrDefaultAsync(p => p.FileName == fileName);
    }

    public async Task LinkAsNewVersionAsync(int newProjectId, int previousVersionProjectId)
    {
        var newProject = await _db.Projects.FindAsync(newProjectId);
        var previousProject = await _db.Projects.FindAsync(previousVersionProjectId);
        if (newProject is null || previousProject is null)
            throw new InvalidOperationException("Project not found");

        previousProject.IsCurrentVersion = false;
        newProject.PreviousVersionProjectId = previousProject.Id;
        newProject.VersionNumber = previousProject.VersionNumber + 1;
        newProject.IsCurrentVersion = true;

        await _db.SaveChangesAsync();
    }

    // ponytail: stat-based (mtime+size), not a content hash — cheap enough to check on every view, see ADR-0015
    public static ProjectFileStatus GetFileStatus(Project project)
    {
        var info = new FileInfo(project.FilePath);
        if (!info.Exists) return ProjectFileStatus.Missing;

        return info.LastWriteTimeUtc == project.LastKnownWriteTimeUtc && info.Length == project.LastKnownFileSizeBytes
            ? ProjectFileStatus.Ok
            : ProjectFileStatus.Changed;
    }

    // Reads plates fresh from the .3mf zip on every call — no cached copy, matching the
    // stat-based (not content-hashed) drift detection above (ADR-0015/0016).
    public static List<ProjectPlate> ReadPlates(string filePath)
    {
        using var archive = ZipFile.OpenRead(filePath);
        var configEntry = archive.GetEntry("Metadata/model_settings.config");
        if (configEntry is null) return [];

        using var configStream = configEntry.Open();
        var doc = XDocument.Load(configStream);

        var plates = new List<ProjectPlate>();
        foreach (var plateEl in doc.Root?.Elements("plate") ?? [])
        {
            string? Meta(string key) => plateEl.Elements("metadata")
                .FirstOrDefault(m => (string?)m.Attribute("key") == key)?.Attribute("value")?.Value;

            var platerId = Meta("plater_id");
            if (platerId is null) continue;

            byte[]? thumbnailBytes = null;
            var thumbnailFile = Meta("thumbnail_file");
            var thumbnailEntry = thumbnailFile is not null ? archive.GetEntry(thumbnailFile) : null;
            if (thumbnailEntry is not null)
            {
                using var thumbnailStream = thumbnailEntry.Open();
                using var ms = new MemoryStream();
                thumbnailStream.CopyTo(ms);
                thumbnailBytes = ms.ToArray();
            }

            var platerName = Meta("plater_name");
            plates.Add(new ProjectPlate
            {
                PlaterId = platerId,
                PlaterName = string.IsNullOrEmpty(platerName) ? null : platerName,
                ThumbnailBytes = thumbnailBytes
            });
        }

        return plates;
    }
}
