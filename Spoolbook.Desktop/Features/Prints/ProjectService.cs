using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;
namespace Spoolbook.Desktop.Features.Prints;

public enum ProjectFileStatus { Ok, Missing, Changed }

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

    public async Task<ProjectResult> UpsertByPathAsync(string filePath)
    {
        var info = new FileInfo(filePath);
        if (!info.Exists)
            return new ProjectResult { Ok = false, Error = "file_not_found" };

        var project = await _db.Projects.FirstOrDefaultAsync(p => p.FilePath == filePath);
        if (project is null)
        {
            project = new Project { FilePath = filePath, FileName = info.Name };
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

    // ponytail: stat-based (mtime+size), not a content hash — cheap enough to check on every view, see ADR-0015
    public static ProjectFileStatus GetFileStatus(Project project)
    {
        var info = new FileInfo(project.FilePath);
        if (!info.Exists) return ProjectFileStatus.Missing;

        return info.LastWriteTimeUtc == project.LastKnownWriteTimeUtc && info.Length == project.LastKnownFileSizeBytes
            ? ProjectFileStatus.Ok
            : ProjectFileStatus.Changed;
    }
}
