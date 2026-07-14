using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Settings.Colors;
namespace Spoolbook.Desktop.Features.Settings.Filaments;

public partial class FilamentEditViewModel : ViewModelBase
{
    private readonly FilamentService _filamentService;
    private readonly FilamentColorService _colorService;
    private readonly int? _id;
    private List<FilamentColor> _colors = new();

    [ObservableProperty]
    private string brand = "";

    [ObservableProperty]
    private string material = "";

    [ObservableProperty]
    private string? variant;

    [ObservableProperty]
    private string color = "";

    [ObservableProperty]
    private string? errorMessage;

    [ObservableProperty]
    private ObservableCollection<string> colorNameOptions = new();

    public bool IsEdit { get; }
    public string PageTitle => IsEdit ? "Edit filament" : "Add filament";
    public string? SwatchHex => _colors.FirstOrDefault(c => c.Name == Color)?.Hex;
    public Action? Close { get; set; }

    partial void OnColorChanged(string value) => OnPropertyChanged(nameof(SwatchHex));

    public FilamentEditViewModel(FilamentService filamentService, FilamentColorService colorService, Filament? existing)
    {
        _filamentService = filamentService;
        _colorService = colorService;

        if (existing is not null)
        {
            _id = existing.Id;
            IsEdit = true;
            Brand = existing.Brand;
            Material = existing.Material;
            Variant = existing.Variant;
            Color = existing.Color;
        }

        _ = LoadColorsAsync();
    }

    private async Task LoadColorsAsync()
    {
        _colors = await _colorService.ListAsync();
        ColorNameOptions = new ObservableCollection<string>(_colors.Select(c => c.Name));
        OnPropertyChanged(nameof(SwatchHex));
    }

    [RelayCommand]
    private async Task SaveAsync()
    {
        var input = new FilamentInput
        {
            Brand = Brand,
            Material = Material,
            Variant = string.IsNullOrWhiteSpace(Variant) ? null : Variant,
            Color = Color
        };
        var result = _id.HasValue
            ? await _filamentService.UpdateAsync(_id.Value, input)
            : await _filamentService.CreateAsync(input);

        if (!result.Ok)
        {
            ErrorMessage = result.Error == "duplicate" ? "This exact filament is already in the catalog." : result.Error;
            return;
        }

        Close?.Invoke();
    }

    [RelayCommand]
    private void Cancel() => Close?.Invoke();
}
