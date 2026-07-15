using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Settings.Printers;
namespace Spoolbook.Desktop.Features.Prints;

public partial class PrintInventoryViewModel : ViewModelBase
{
    private readonly PrintInventoryService _inventoryService;
    private readonly PrintService _printService;
    private readonly SpoolService _spoolService;
    private readonly PrintProfileService _profileService;
    private readonly FilamentService _filamentService;
    private readonly PrinterService _printerService;
    private readonly ProjectService _projectService;

    [ObservableProperty]
    private string? brandFilter;

    [ObservableProperty]
    private string? materialFilter;

    [ObservableProperty]
    private PrintStatus? statusFilter;

    [ObservableProperty]
    private ObservableCollection<Print> prints = new();

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

    public static PrintStatus[] StatusOptions { get; } = Enum.GetValues<PrintStatus>();

    private PrintSortColumn _sort = PrintSortColumn.StartedAt;
    private SortOrder _order = SortOrder.Desc;

    public PrintInventoryViewModel(
        PrintInventoryService inventoryService,
        PrintService printService,
        SpoolService spoolService,
        PrintProfileService profileService,
        FilamentService filamentService,
        PrinterService printerService,
        ProjectService projectService)
    {
        _inventoryService = inventoryService;
        _printService = printService;
        _spoolService = spoolService;
        _profileService = profileService;
        _filamentService = filamentService;
        _printerService = printerService;
        _projectService = projectService;
        _ = ReloadAsync();
    }

    public async Task ReloadAsync()
    {
        var result = await _inventoryService.ListAsync(new PrintInventoryQuery
        {
            Brand = string.IsNullOrWhiteSpace(BrandFilter) ? null : BrandFilter,
            Material = string.IsNullOrWhiteSpace(MaterialFilter) ? null : MaterialFilter,
            Status = StatusFilter,
            Sort = _sort,
            Order = _order,
            Page = Page,
            PageSize = PageSize
        });

        Prints = new ObservableCollection<Print>(result.Prints);
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
        var sort = Enum.Parse<PrintSortColumn>(column);
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

    public PrintEditViewModel CreateEditViewModel(Print? existing) =>
        new(_printService, _spoolService, _profileService, _printerService, _projectService, existing);
}
