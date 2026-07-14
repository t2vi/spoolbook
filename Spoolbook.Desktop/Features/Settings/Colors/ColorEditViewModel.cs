using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
namespace Spoolbook.Desktop.Features.Settings.Colors;

public partial class ColorEditViewModel : ViewModelBase
{
    private readonly FilamentColorService _colorService;
    private readonly int? _id;

    [ObservableProperty]
    private string name = "";

    [ObservableProperty]
    private string hex = "#FFFFFF";

    [ObservableProperty]
    private string? errorMessage;

    public bool IsEdit { get; }
    public string PageTitle => IsEdit ? "Edit color" : "Add color";
    public Action? Close { get; set; }

    public ColorEditViewModel(FilamentColorService colorService, FilamentColor? existing)
    {
        _colorService = colorService;

        if (existing is not null)
        {
            _id = existing.Id;
            IsEdit = true;
            Name = existing.Name;
            Hex = existing.Hex;
        }
    }

    [RelayCommand]
    private async Task SaveAsync()
    {
        var input = new FilamentColorInput { Name = Name, Hex = Hex };
        var result = _id.HasValue
            ? await _colorService.UpdateAsync(_id.Value, input)
            : await _colorService.CreateAsync(input);

        if (!result.Ok)
        {
            ErrorMessage = result.Error switch
            {
                "duplicate" => "A color with this name already exists.",
                "invalid_hex" => "Each hex code must look like #RRGGBB (comma-separate for multi-color).",
                _ => result.Error
            };
            return;
        }

        Close?.Invoke();
    }

    [RelayCommand]
    private void Cancel() => Close?.Invoke();
}
