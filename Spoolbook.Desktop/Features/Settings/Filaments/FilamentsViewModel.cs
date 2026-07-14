using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Settings.Colors;
using Spoolbook.Desktop.Features.Settings.General;

namespace Spoolbook.Desktop.Features.Settings.Filaments;

public partial class FilamentsViewModel : ViewModelBase
{
    private readonly FilamentService _filamentService;
    private readonly FilamentColorService _colorService;
    private readonly AppSettingsService _appSettingsService;
    private readonly FilamentCatalogSyncService _catalogSyncService = new();

    [ObservableProperty]
    private ObservableCollection<Filament> filaments = new();

    [ObservableProperty]
    private string? errorMessage;

    [ObservableProperty]
    private string? syncStatusMessage;

    [ObservableProperty]
    private bool isSyncing;

    [ObservableProperty]
    private string? brandFilter;

    [ObservableProperty]
    private string? materialFilter;

    [ObservableProperty]
    private string? variantFilter;

    [ObservableProperty]
    private string? colorFilter;

    [ObservableProperty]
    private int page = 1;

    [ObservableProperty]
    private int totalPages = 1;

    [ObservableProperty]
    private int pageSize = 10;

    [ObservableProperty]
    private ObservableCollection<string> brandOptions = new();

    [ObservableProperty]
    private ObservableCollection<string> materialOptions = new();

    [ObservableProperty]
    private ObservableCollection<string> variantOptions = new();

    [ObservableProperty]
    private ObservableCollection<string> colorFilterOptions = new();

    private FilamentSortColumn _sort = FilamentSortColumn.Brand;
    private SortOrder _order = SortOrder.Asc;

    public FilamentsViewModel(FilamentService filamentService, FilamentColorService colorService, AppSettingsService appSettingsService)
    {
        _filamentService = filamentService;
        _colorService = colorService;
        _appSettingsService = appSettingsService;
        _ = ReloadAsync();
    }

    private async Task LoadFilterOptionsAsync()
    {
        BrandOptions = new ObservableCollection<string>(await _filamentService.ListDistinctBrandsAsync());
        MaterialOptions = new ObservableCollection<string>(await _filamentService.ListDistinctMaterialsAsync());
        VariantOptions = new ObservableCollection<string>(await _filamentService.ListDistinctVariantsAsync());

        var colors = await _colorService.ListAsync();
        ColorFilterOptions = new ObservableCollection<string>(colors.Select(c => c.Name));
        // Saving a known filament can auto-create a new color (see FilamentColorService) — refresh the
        // shared swatch cache so it renders correctly without waiting on the Colors tab's own reload.
        Converters.ColorSwatchConverter.SetPalette(colors);
    }

    public async Task ReloadAsync()
    {
        var result = await _filamentService.SearchAsync(new FilamentQuery
        {
            Brand = string.IsNullOrWhiteSpace(BrandFilter) ? null : BrandFilter,
            Material = string.IsNullOrWhiteSpace(MaterialFilter) ? null : MaterialFilter,
            Variant = string.IsNullOrWhiteSpace(VariantFilter) ? null : VariantFilter,
            Color = string.IsNullOrWhiteSpace(ColorFilter) ? null : ColorFilter,
            Sort = _sort,
            Order = _order,
            Page = Page,
            PageSize = PageSize
        });

        Filaments = new ObservableCollection<Filament>(result.Entries);
        TotalPages = result.TotalPages;
        await LoadFilterOptionsAsync();
    }

    public FilamentEditViewModel CreateEditViewModel(Filament? existing) =>
        new(_filamentService, _colorService, existing);

    [RelayCommand]
    private async Task DeleteAsync(Filament entry)
    {
        var result = await _filamentService.DeleteAsync(entry.Id);
        if (!result.Ok)
        {
            ErrorMessage = result.Error == "has_spools" ? "Can't delete — spools exist for this filament." : result.Error;
            return;
        }

        ErrorMessage = null;
        await ReloadAsync();
    }

    [RelayCommand]
    private async Task SyncCatalogAsync()
    {
        IsSyncing = true;
        SyncStatusMessage = "Syncing filament catalog...";

        var additionalSources = await _appSettingsService.GetAdditionalFilamentSourceUrlsAsync();
        var result = await _catalogSyncService.FetchAsync(additionalSources);
        if (!result.Ok)
        {
            SyncStatusMessage = $"Sync failed: {result.Error}";
            IsSyncing = false;
            return;
        }

        var summary = await _filamentService.ImportManyAsync(result.Entries);
        await _appSettingsService.RecordFilamentSyncAsync();
        SyncStatusMessage = $"Added {summary.Added} new, skipped {summary.Skipped} duplicates.";
        IsSyncing = false;
        await ReloadAsync();
    }

    [RelayCommand]
    private async Task ApplyFiltersAsync()
    {
        Page = 1;
        await ReloadAsync();
    }

    partial void OnPageSizeChanged(int value)
    {
        Page = 1;
        _ = ReloadAsync();
    }

    [RelayCommand]
    private async Task SortByAsync(string column)
    {
        var sort = Enum.Parse<FilamentSortColumn>(column);
        _order = _sort == sort && _order == SortOrder.Asc ? SortOrder.Desc : SortOrder.Asc;
        _sort = sort;
        Page = 1;
        await ReloadAsync();
    }

    [RelayCommand]
    private async Task PreviousPageAsync()
    {
        if (Page <= 1) return;
        Page--;
        await ReloadAsync();
    }

    [RelayCommand]
    private async Task NextPageAsync()
    {
        if (Page >= TotalPages) return;
        Page++;
        await ReloadAsync();
    }
}
