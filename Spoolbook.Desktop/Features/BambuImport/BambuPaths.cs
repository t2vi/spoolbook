using System.Runtime.InteropServices;
namespace Spoolbook.Desktop.Features.BambuImport;

// Best-effort default install locations for Bambu Studio's user + system presets.
// Only verified on macOS so far — Windows/Linux paths are reasonable guesses,
// not yet confirmed against a real install (multi-OS build is a later concern).
public static class BambuPaths
{
    public static string? FindUserFilamentPresetsDir()
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        string userRoot;

        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            userRoot = Path.Combine(home, "Library", "Application Support", "BambuStudio", "user");
        else if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            userRoot = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "BambuStudio", "user");
        else
            userRoot = Path.Combine(home, ".config", "BambuStudio", "user");

        if (!Directory.Exists(userRoot)) return null;

        // A single Bambu Studio account's presets live under user/<account-id>/filament.
        // There's often also an empty "default" account folder alongside the real one —
        // pick the one that actually has preset files, not just any existing directory.
        var candidate = Directory.GetDirectories(userRoot)
            .Select(d => Path.Combine(d, "filament"))
            .FirstOrDefault(d => Directory.Exists(d) && Directory.GetFiles(d, "*.json").Length > 0);

        return candidate;
    }

    public static string? FindSystemProfilesDir()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
        {
            var candidates = new[]
            {
                "/Applications/BambuStudio.app/Contents/Resources/profiles",
                "/Applications/Bambu Studio.app/Contents/Resources/profiles"
            };
            return candidates.FirstOrDefault(Directory.Exists);
        }

        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            var candidates = new[]
            {
                @"C:\Program Files\Bambu Studio\resources\profiles",
                @"C:\Program Files (x86)\Bambu Studio\resources\profiles"
            };
            return candidates.FirstOrDefault(Directory.Exists);
        }

        return null;
    }
}
