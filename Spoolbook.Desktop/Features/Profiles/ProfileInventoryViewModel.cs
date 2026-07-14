using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.BambuImport;
namespace Spoolbook.Desktop.Features.Profiles;

public partial class ProfileInventoryViewModel : ViewModelBase
{
    private readonly ProfileInventoryService _inventoryService;
    private readonly PrintProfileService _profileService;
    private readonly BambuFilamentImportService _importService;
    private readonly FilamentService _filamentService;

    [ObservableProperty]
    private string? brandFilter;

    [ObservableProperty]
    private string? materialFilter;

    [ObservableProperty]
    private ProfileScope? scopeFilter;

    [ObservableProperty]
    private ObservableCollection<PrintProfile> profiles = new();

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

    private ProfileSortColumn _sort = ProfileSortColumn.Filament;
    private SortOrder _order = SortOrder.Asc;

    public ProfileInventoryViewModel(
        ProfileInventoryService inventoryService,
        PrintProfileService profileService,
        BambuFilamentImportService importService,
        FilamentService filamentService)
    {
        _inventoryService = inventoryService;
        _profileService = profileService;
        _importService = importService;
        _filamentService = filamentService;
        _ = ReloadAsync();
    }

    public async Task ReloadAsync()
    {
        var result = await _inventoryService.ListAsync(new ProfileInventoryQuery
        {
            Brand = string.IsNullOrWhiteSpace(BrandFilter) ? null : BrandFilter,
            Material = string.IsNullOrWhiteSpace(MaterialFilter) ? null : MaterialFilter,
            Scope = ScopeFilter,
            Sort = _sort,
            Order = _order,
            Page = Page,
            PageSize = PageSize
        });

        Profiles = new ObservableCollection<PrintProfile>(result.Profiles);
        TotalPages = result.TotalPages;
        BrandOptions = new ObservableCollection<string>(await _filamentService.ListDistinctBrandsAsync());
        MaterialOptions = new ObservableCollection<string>(await _filamentService.ListDistinctMaterialsAsync());
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
        var sort = Enum.Parse<ProfileSortColumn>(column);
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

    public ProfileEditViewModel CreateEditViewModel(PrintProfile? existing) =>
        new(_profileService, _importService, _filamentService, existing);
}
