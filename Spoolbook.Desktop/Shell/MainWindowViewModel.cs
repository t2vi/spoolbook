using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Features.Prints;
using Spoolbook.Desktop.Features.Settings;
using Spoolbook.Desktop.Features.Settings.General;
using Spoolbook.Desktop.Features.Settings.Colors;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Dashboard;
using Spoolbook.Desktop.Features.BambuImport;
namespace Spoolbook.Desktop.Shell;

public partial class MainWindowViewModel : ViewModelBase
{
    private readonly FilamentService _filamentService;
    private readonly SpoolService _spoolService;
    private readonly PrintProfileService _profileService;
    private readonly BambuFilamentImportService _importService;
    private readonly SpoolInventoryService _spoolInventoryService;
    private readonly ProfileInventoryService _profileInventoryService;
    private readonly PrintService _printService;
    private readonly PrintInventoryService _printInventoryService;
    private readonly FilamentColorService _colorService;
    private readonly AppSettingsService _appSettingsService;
    private readonly DashboardMetricsService _metricsService;

    [ObservableProperty]
    private ViewModelBase currentPage;

    public MainWindowViewModel(
        FilamentService filamentService,
        SpoolService spoolService,
        PrintProfileService profileService,
        BambuFilamentImportService importService,
        SpoolInventoryService spoolInventoryService,
        ProfileInventoryService profileInventoryService,
        PrintService printService,
        PrintInventoryService printInventoryService,
        FilamentColorService colorService,
        AppSettingsService appSettingsService,
        DashboardMetricsService metricsService)
    {
        _filamentService = filamentService;
        _spoolService = spoolService;
        _profileService = profileService;
        _importService = importService;
        _spoolInventoryService = spoolInventoryService;
        _profileInventoryService = profileInventoryService;
        _printService = printService;
        _printInventoryService = printInventoryService;
        _colorService = colorService;
        _appSettingsService = appSettingsService;
        _metricsService = metricsService;

        currentPage = new DashboardViewModel(
            filamentService, spoolService, profileService, importService,
            spoolInventoryService, profileInventoryService, colorService, metricsService, Navigate);
    }

    private void Navigate(ViewModelBase page) => CurrentPage = page;

    [RelayCommand]
    private void ShowDashboard() =>
        Navigate(new DashboardViewModel(
            _filamentService, _spoolService, _profileService, _importService,
            _spoolInventoryService, _profileInventoryService, _colorService, _metricsService, Navigate));

    [RelayCommand]
    private void ShowSpools() =>
        Navigate(new SpoolInventoryViewModel(_spoolInventoryService, _spoolService, _filamentService));

    [RelayCommand]
    private void ShowProfiles() =>
        Navigate(new ProfileInventoryViewModel(_profileInventoryService, _profileService, _importService, _filamentService));

    [RelayCommand]
    private void ShowPrints() =>
        Navigate(new PrintInventoryViewModel(_printInventoryService, _printService, _spoolService, _profileService, _filamentService));

    [RelayCommand]
    private void ShowSettings() =>
        Navigate(new SettingsViewModel(_colorService, _filamentService, _appSettingsService, _importService));
}
