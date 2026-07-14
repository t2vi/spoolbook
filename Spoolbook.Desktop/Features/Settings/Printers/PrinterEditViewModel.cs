using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
namespace Spoolbook.Desktop.Features.Settings.Printers;

public partial class PrinterEditViewModel : ViewModelBase
{
    private readonly PrinterService _printerService;
    private readonly int? _id;

    [ObservableProperty]
    private string name = "";

    [ObservableProperty]
    private string? model;

    [ObservableProperty]
    private string? errorMessage;

    public bool IsEdit { get; }
    public string PageTitle => IsEdit ? "Edit printer" : "Add printer";
    public Action? Close { get; set; }

    public PrinterEditViewModel(PrinterService printerService, Printer? existing)
    {
        _printerService = printerService;

        if (existing is not null)
        {
            _id = existing.Id;
            IsEdit = true;
            Name = existing.Name;
            Model = existing.Model;
        }
    }

    [RelayCommand]
    private async Task SaveAsync()
    {
        var input = new PrinterInput { Name = Name, Model = string.IsNullOrWhiteSpace(Model) ? null : Model };
        var result = _id.HasValue
            ? await _printerService.UpdateAsync(_id.Value, input)
            : await _printerService.CreateAsync(input);

        if (!result.Ok)
        {
            ErrorMessage = result.Error switch
            {
                "duplicate" => "A printer with this name already exists.",
                _ => result.Error
            };
            return;
        }

        Close?.Invoke();
    }

    [RelayCommand]
    private void Cancel() => Close?.Invoke();
}
