using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;
namespace Spoolbook.Desktop.Features.Profiles;

public class ProfileInput
{
    public int? SpoolId { get; set; }
    public required string Name { get; set; }
    public required string NozzleTempC { get; set; }
    public string? NozzleTempInitialC { get; set; }
    public ProfileSource? Source { get; set; }
    public SlicerType? SourceSlicer { get; set; }
    public string? RawSettingsJson { get; set; }
    public string? SourcePresetPath { get; set; }
    public int VersionNumber { get; set; } = 1;
    public string? VersionName { get; set; }
    public string? Notes { get; set; }
    public Dictionary<string, string> Fields { get; set; } = new();
}

public class ProfileResult
{
    public bool Ok { get; init; }
    public PrintProfile? Profile { get; init; }
    public Dictionary<string, string>? Errors { get; init; }
}

public class ProfileDeleteResult
{
    public bool Ok { get; init; }
    public string? Error { get; init; }
}

public class PrintProfileService
{
    private readonly SpoolbookDbContext _db;

    public PrintProfileService(SpoolbookDbContext db)
    {
        _db = db;
    }

    private static Dictionary<string, string>? Validate(ProfileInput input)
    {
        var errors = new Dictionary<string, string>();
        if (string.IsNullOrWhiteSpace(input.Name)) errors["Name"] = "Name is required";
        if (string.IsNullOrWhiteSpace(input.NozzleTempC) || !int.TryParse(input.NozzleTempC, out _))
            errors["NozzleTempC"] = "Nozzle temp is required";

        return errors.Count > 0 ? errors : null;
    }

    private static int? ParseNullableInt(string? raw) =>
        string.IsNullOrWhiteSpace(raw) ? null : (int.TryParse(raw, out var v) ? v : null);

    public async Task<ProfileResult> CreateProfileAsync(int filamentId, ProfileInput input)
    {
        var errors = Validate(input);
        if (errors is not null) return new ProfileResult { Ok = false, Errors = errors };

        var profile = new PrintProfile
        {
            FilamentId = filamentId,
            SpoolId = input.SpoolId,
            Name = input.Name,
            NozzleTempC = int.Parse(input.NozzleTempC),
            NozzleTempInitialC = ParseNullableInt(input.NozzleTempInitialC),
            Source = input.Source ?? ProfileSource.Manual,
            SourceSlicer = input.SourceSlicer,
            RawSettingsJson = input.RawSettingsJson,
            SourcePresetPath = input.SourcePresetPath,
            VersionNumber = input.VersionNumber,
            VersionName = input.VersionName,
            Notes = input.Notes
        };
        ProfileFieldMapper.Apply(profile, input.Fields);

        _db.PrintProfiles.Add(profile);

        if (input.SourcePresetPath is not null)
        {
            var siblings = await _db.PrintProfiles
                .Where(p => p.FilamentId == filamentId && p.SourcePresetPath == input.SourcePresetPath)
                .ToListAsync();
            foreach (var sibling in siblings) sibling.IsCurrentVersion = false;
        }
        profile.IsCurrentVersion = true;

        await _db.SaveChangesAsync();

        return new ProfileResult { Ok = true, Profile = profile };
    }

    public async Task<ProfileResult> UpdateProfileAsync(int id, ProfileInput input)
    {
        var errors = Validate(input);
        if (errors is not null) return new ProfileResult { Ok = false, Errors = errors };

        if (await _db.Prints.AnyAsync(p => p.ProfileId == id))
            return new ProfileResult { Ok = false, Errors = new Dictionary<string, string> { ["Locked"] = "This version has been used in a Print — save as a new version instead." } };

        var profile = await _db.PrintProfiles.FindAsync(id);
        if (profile is null) throw new InvalidOperationException("Profile not found");

        profile.SpoolId = input.SpoolId;
        profile.Name = input.Name;
        profile.NozzleTempC = int.Parse(input.NozzleTempC);
        profile.NozzleTempInitialC = ParseNullableInt(input.NozzleTempInitialC);
        // Falls back to the existing value when input omits it, so a plain field edit
        // (which doesn't set these on ProfileInput) can't wipe provenance — but linking
        // a preset during this same edit session does carry a value and gets persisted.
        profile.Source = input.Source ?? profile.Source;
        profile.SourceSlicer = input.SourceSlicer ?? profile.SourceSlicer;
        profile.RawSettingsJson = input.RawSettingsJson ?? profile.RawSettingsJson;
        profile.SourcePresetPath = input.SourcePresetPath ?? profile.SourcePresetPath;
        // VersionNumber intentionally left untouched — "Check for updates" creates a new
        // profile via CreateProfileAsync for version bumps, Update never bumps it.
        profile.Notes = input.Notes;
        ProfileFieldMapper.Apply(profile, input.Fields);

        await _db.SaveChangesAsync();

        return new ProfileResult { Ok = true, Profile = profile };
    }

    public async Task<PrintProfile?> GetProfileAsync(int id)
    {
        return await _db.PrintProfiles.FindAsync(id);
    }

    public async Task<List<PrintProfile>> ListProfilesForFilamentAsync(int filamentId)
    {
        return await _db.PrintProfiles
            .Where(p => p.FilamentId == filamentId && p.IsCurrentVersion)
            .OrderBy(p => p.SpoolId == null ? 0 : 1)
            .ThenBy(p => p.Name)
            .ToListAsync();
    }

    public async Task<List<PrintProfile>> ListVersionsAsync(int filamentId, string sourcePresetPath)
    {
        return await _db.PrintProfiles
            .Where(p => p.FilamentId == filamentId && p.SourcePresetPath == sourcePresetPath)
            .OrderBy(p => p.VersionNumber)
            .ToListAsync();
    }

    public async Task RenameVersionAsync(int id, string? versionName)
    {
        var profile = await _db.PrintProfiles.FindAsync(id);
        if (profile is null) throw new InvalidOperationException("Profile not found");

        profile.VersionName = versionName;
        await _db.SaveChangesAsync();
    }

    public async Task<PrintProfile> DuplicateProfileAsync(int id)
    {
        var existing = await _db.PrintProfiles.AsNoTracking().FirstOrDefaultAsync(p => p.Id == id);
        if (existing is null) throw new InvalidOperationException("Profile not found");

        var duplicate = new PrintProfile
        {
            FilamentId = existing.FilamentId,
            SpoolId = existing.SpoolId,
            Name = $"{existing.Name} (copy)",
            NozzleTempC = existing.NozzleTempC,
            Source = existing.Source,
            SourceSlicer = existing.SourceSlicer,
            RawSettingsJson = existing.RawSettingsJson,
            Notes = existing.Notes
        };
        ProfileFieldMapper.Apply(duplicate, ProfileFieldMapper.ToFieldStrings(existing));

        _db.PrintProfiles.Add(duplicate);
        await _db.SaveChangesAsync();

        return duplicate;
    }

    public async Task<ProfileDeleteResult> DeleteProfileAsync(int id)
    {
        var profile = await _db.PrintProfiles.FindAsync(id);
        if (profile is null) throw new InvalidOperationException("Profile not found");

        if (await _db.Prints.AnyAsync(p => p.ProfileId == id))
            return new ProfileDeleteResult { Ok = false, Error = "has_prints" };

        _db.PrintProfiles.Remove(profile);
        await _db.SaveChangesAsync();

        return new ProfileDeleteResult { Ok = true };
    }
}
