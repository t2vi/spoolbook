using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using LiveChartsCore;
using LiveChartsCore.Defaults;
using LiveChartsCore.SkiaSharpView;
using LiveChartsCore.SkiaSharpView.Painting;
using SkiaSharp;

using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Features.Settings.Colors;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.BambuImport;
namespace Spoolbook.Desktop.Features.Dashboard;

public partial class DashboardViewModel : ViewModelBase
{
    private readonly FilamentService _filamentService;
    private readonly SpoolService _spoolService;
    private readonly PrintProfileService _profileService;
    private readonly BambuFilamentImportService _importService;
    private readonly SpoolInventoryService _spoolInventoryService;
    private readonly ProfileInventoryService _profileInventoryService;
    private readonly FilamentColorService _colorService;
    private readonly DashboardMetricsService _metricsService;
    private readonly Action<ViewModelBase> _navigate;

    [ObservableProperty]
    private int spoolCount;

    [ObservableProperty]
    private int profileCount;

    [ObservableProperty]
    private int filamentCount;

    [ObservableProperty]
    private string lastFilamentSyncText = "Never synced";

    [ObservableProperty]
    private ISeries[] filamentsByBrandSeries = [];

    [ObservableProperty]
    private ISeries[] filamentsByMaterialSeries = [];

    [ObservableProperty]
    private ISeries[] spoolsByStatusSeries = [];

    [ObservableProperty]
    private ISeries[] printsByStatusSeries = [];

    // Cycled by category index so each bar/slice — and its legend entry — gets a distinct color.
    private static readonly SKColor[] Palette =
    [
        new(0x5B, 0x8F, 0xF9), new(0xFF, 0x6B, 0x6B), new(0x4C, 0xC9, 0x90), new(0xFF, 0xB4, 0x00),
        new(0x9B, 0x5D, 0xE0), new(0x00, 0xBC, 0xD4), new(0xF7, 0x67, 0x07), new(0x8B, 0xC3, 0x4A),
        new(0xE9, 0x1E, 0x63), new(0x60, 0x7D, 0x8B), new(0xFF, 0xEB, 0x3B), new(0x79, 0x55, 0x48),
        new(0x00, 0x96, 0x88), new(0x67, 0x3A, 0xB7), new(0xCD, 0xDC, 0x39), new(0x03, 0xA9, 0xF4)
    ];

    // Each series gets its own X index (rather than defaulting all to 0) so bars sit apart and hover isolates one bar.
    private static ISeries[] BuildColumnSeries(IEnumerable<CategoryCount> data) =>
        data.Select((c, i) => (ISeries)new ColumnSeries<ObservablePoint>
        {
            Values = [new ObservablePoint(i, c.Count)],
            Name = c.Label,
            Fill = new SolidColorPaint(Palette[i % Palette.Length]),
            IgnoresBarPosition = true
        }).ToArray();

    private static ISeries[] BuildPieSeries(IEnumerable<CategoryCount> data) =>
        data.Select((c, i) => (ISeries)new PieSeries<int>
        {
            Values = [c.Count],
            Name = c.Label,
            Fill = new SolidColorPaint(Palette[i % Palette.Length])
        }).ToArray();

    public DashboardViewModel(
        FilamentService filamentService,
        SpoolService spoolService,
        PrintProfileService profileService,
        BambuFilamentImportService importService,
        SpoolInventoryService spoolInventoryService,
        ProfileInventoryService profileInventoryService,
        FilamentColorService colorService,
        DashboardMetricsService metricsService,
        Action<ViewModelBase> navigate)
    {
        _filamentService = filamentService;
        _spoolService = spoolService;
        _profileService = profileService;
        _importService = importService;
        _spoolInventoryService = spoolInventoryService;
        _profileInventoryService = profileInventoryService;
        _colorService = colorService;
        _metricsService = metricsService;
        _navigate = navigate;
        _ = LoadAsync();
    }

    private async Task LoadAsync()
    {
        var spools = await _spoolInventoryService.ListAsync(new SpoolInventoryQuery { Page = 1, PageSize = 1 });
        SpoolCount = spools.Total;

        var profiles = await _profileInventoryService.ListAsync(new ProfileInventoryQuery { Page = 1, PageSize = 1 });
        ProfileCount = profiles.Total;

        var metrics = await _metricsService.GetMetricsAsync();
        FilamentCount = metrics.FilamentCount;
        LastFilamentSyncText = metrics.LastFilamentSyncAt is { } syncedAt
            ? $"Last synced {syncedAt.ToLocalTime():g}"
            : "Never synced";

        FilamentsByBrandSeries = BuildColumnSeries(metrics.FilamentsByBrand);
        FilamentsByMaterialSeries = BuildColumnSeries(metrics.FilamentsByMaterial);
        SpoolsByStatusSeries = BuildPieSeries(metrics.SpoolsByStatus);
        PrintsByStatusSeries = BuildPieSeries(metrics.PrintsByStatus);
    }

    [RelayCommand]
    private void ShowSpools() =>
        _navigate(new SpoolInventoryViewModel(_spoolInventoryService, _spoolService, _filamentService));

    [RelayCommand]
    private void ShowProfiles() =>
        _navigate(new ProfileInventoryViewModel(_profileInventoryService, _profileService, _importService, _filamentService));
}
