using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Settings.Filaments;
namespace Spoolbook.Desktop.Features.Spools;

public partial class SpoolInventoryViewModel : ViewModelBase
{
    private readonly SpoolInventoryService _inventoryService;
    private readonly SpoolService _spoolService;
    private readonly FilamentService _filamentService;

    [ObservableProperty]
    private string? brandFilter;

    [ObservableProperty]
    private string? materialFilter;

    [ObservableProperty]
    private SpoolStatus? statusFilter;

    [ObservableProperty]
    private ObservableCollection<Spool> spools = new();

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

    private SpoolSortColumn _sort = SpoolSortColumn.Filament;
    private SortOrder _order = SortOrder.Asc;

    public SpoolInventoryViewModel(SpoolInventoryService inventoryService, SpoolService spoolService, FilamentService filamentService)
    {
        _inventoryService = inventoryService;
        _spoolService = spoolService;
        _filamentService = filamentService;
        _ = ReloadAsync();
    }

    public async Task ReloadAsync()
    {
        var result = await _inventoryService.ListAsync(new SpoolInventoryQuery
        {
            Brand = string.IsNullOrWhiteSpace(BrandFilter) ? null : BrandFilter,
            Material = string.IsNullOrWhiteSpace(MaterialFilter) ? null : MaterialFilter,
            Status = StatusFilter,
            Sort = _sort,
            Order = _order,
            Page = Page,
            PageSize = PageSize
        });

        Spools = new ObservableCollection<Spool>(result.Spools);
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
        var sort = Enum.Parse<SpoolSortColumn>(column);
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

    public SpoolEditViewModel CreateEditViewModel(Spool? existing) =>
        new(_spoolService, _filamentService, existing);
}
