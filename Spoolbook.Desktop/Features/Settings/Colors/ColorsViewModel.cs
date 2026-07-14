using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;

namespace Spoolbook.Desktop.Features.Settings.Colors;

public partial class ColorsViewModel : ViewModelBase
{
    private readonly FilamentColorService _colorService;

    [ObservableProperty]
    private ObservableCollection<FilamentColor> colors = new();

    [ObservableProperty]
    private string? nameFilter;

    [ObservableProperty]
    private int page = 1;

    [ObservableProperty]
    private int totalPages = 1;

    [ObservableProperty]
    private int pageSize = 10;

    [ObservableProperty]
    private ObservableCollection<string> nameOptions = new();

    private ColorSortColumn _sort = ColorSortColumn.Name;
    private SortOrder _order = SortOrder.Asc;

    public ColorsViewModel(FilamentColorService colorService)
    {
        _colorService = colorService;
        _ = ReloadAsync();
    }

    public async Task ReloadAsync()
    {
        var full = await _colorService.ListAsync();
        Converters.ColorSwatchConverter.SetPalette(full);
        NameOptions = new ObservableCollection<string>(full.Select(c => c.Name));

        var result = await _colorService.SearchAsync(new ColorQuery
        {
            Name = string.IsNullOrWhiteSpace(NameFilter) ? null : NameFilter,
            Sort = _sort,
            Order = _order,
            Page = Page,
            PageSize = PageSize
        });

        Colors = new ObservableCollection<FilamentColor>(result.Entries);
        TotalPages = result.TotalPages;
    }

    public ColorEditViewModel CreateEditViewModel(FilamentColor? existing) => new(_colorService, existing);

    [RelayCommand]
    private async Task DeleteAsync(FilamentColor color)
    {
        await _colorService.DeleteAsync(color.Id);
        await ReloadAsync();
    }

    [RelayCommand]
    private async Task ApplyFiltersAsync()
    {
        Page = 1;
        await ReloadAsync();
    }

    [RelayCommand]
    private async Task SortByAsync(string column)
    {
        var sort = Enum.Parse<ColorSortColumn>(column);
        _order = _sort == sort && _order == SortOrder.Asc ? SortOrder.Desc : SortOrder.Asc;
        _sort = sort;
        Page = 1;
        await ReloadAsync();
    }

    partial void OnPageSizeChanged(int value)
    {
        Page = 1;
        _ = ReloadAsync();
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
