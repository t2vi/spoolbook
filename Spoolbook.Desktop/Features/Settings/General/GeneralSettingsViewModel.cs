using System.Reflection;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.BambuImport;
using Spoolbook.Desktop.Features.Settings.Filaments;

namespace Spoolbook.Desktop.Features.Settings.General;

public partial class GeneralSettingsViewModel : ViewModelBase
{
    private readonly AppSettingsService _appSettingsService;
    private readonly BambuFilamentImportService _importService;

    [ObservableProperty]
    private string? bambuUserPresetsDir;

    [ObservableProperty]
    private string? bambuSystemProfilesDir;

    [ObservableProperty]
    private string? detectedBambuUserPresetsDir;

    [ObservableProperty]
    private string? detectedBambuSystemProfilesDir;

    public string AppVersion { get; } = Assembly.GetExecutingAssembly().GetName().Version?.ToString(3) ?? "dev";

    public string FilamentDbSource { get; } = FilamentCatalogSyncService.CatalogUrl;

    [ObservableProperty]
    private string filamentDbVersion = "Bundled (not yet synced)";

    [ObservableProperty]
    private string? additionalFilamentSourceUrls;

    public GeneralSettingsViewModel(AppSettingsService appSettingsService, BambuFilamentImportService importService)
    {
        _appSettingsService = appSettingsService;
        _importService = importService;
        _ = LoadAsync();
    }

    private async Task LoadAsync()
    {
        DetectedBambuUserPresetsDir = BambuPaths.FindUserFilamentPresetsDir();
        DetectedBambuSystemProfilesDir = BambuPaths.FindSystemProfilesDir();

        var settings = await _appSettingsService.GetAsync();
        BambuUserPresetsDir = settings.BambuUserPresetsDir;
        BambuSystemProfilesDir = settings.BambuSystemProfilesDir;
        AdditionalFilamentSourceUrls = settings.AdditionalFilamentSourceUrls;

        // Filament DB has no separate version number — the last successful sync date doubles
        // as one, since that's what actually determines which catalog snapshot is loaded.
        if (settings.LastFilamentSyncAt is { } syncedAt)
            FilamentDbVersion = syncedAt.ToString("yyyy.MM.dd");
    }

    [RelayCommand]
    private async Task SaveAsync()
    {
        await _appSettingsService.SaveAsync(new AppSettingsInput
        {
            BambuUserPresetsDir = BambuUserPresetsDir,
            BambuSystemProfilesDir = BambuSystemProfilesDir,
            AdditionalFilamentSourceUrls = AdditionalFilamentSourceUrls
        });

        var effectiveUserDir = string.IsNullOrWhiteSpace(BambuUserPresetsDir) ? DetectedBambuUserPresetsDir ?? "" : BambuUserPresetsDir;
        var effectiveSystemDir = string.IsNullOrWhiteSpace(BambuSystemProfilesDir) ? DetectedBambuSystemProfilesDir ?? "" : BambuSystemProfilesDir;
        _importService.UpdatePaths(effectiveUserDir, effectiveSystemDir);
    }
}
