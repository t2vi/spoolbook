using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
namespace Spoolbook.Desktop.Features.Settings.Printers;

public partial class PrinterEditViewModel : EditViewModelBase
{
    private readonly PrinterService _printerService;
    private readonly int? _id;

    [ObservableProperty]
    private string name = "";

    [ObservableProperty]
    private string? model;

    [ObservableProperty]
    private string? ipAddress;

    [ObservableProperty]
    private string? accessCode;

    [ObservableProperty]
    private string? serialNumber;

    [ObservableProperty]
    private string? errorMessage;

    public bool IsEdit { get; }
    public string PageTitle => IsEdit ? "Edit printer" : "Add printer";
    public Action? Close { get; set; }

    partial void OnNameChanged(string value) => MarkDirty();
    partial void OnModelChanged(string? value) => MarkDirty();
    partial void OnIpAddressChanged(string? value) => MarkDirty();
    partial void OnAccessCodeChanged(string? value) => MarkDirty();
    partial void OnSerialNumberChanged(string? value) => MarkDirty();

    public PrinterEditViewModel(PrinterService printerService, Printer? existing)
    {
        _printerService = printerService;

        if (existing is not null)
        {
            _id = existing.Id;
            IsEdit = true;
            Name = existing.Name;
            Model = existing.Model;
            IpAddress = existing.IpAddress;
            AccessCode = existing.AccessCode;
            SerialNumber = existing.SerialNumber;
        }

        Loaded = true;
    }

    [RelayCommand]
    private async Task SaveAsync()
    {
        var input = new PrinterInput
        {
            Name = Name,
            Model = string.IsNullOrWhiteSpace(Model) ? null : Model,
            IpAddress = string.IsNullOrWhiteSpace(IpAddress) ? null : IpAddress,
            AccessCode = string.IsNullOrWhiteSpace(AccessCode) ? null : AccessCode,
            SerialNumber = string.IsNullOrWhiteSpace(SerialNumber) ? null : SerialNumber
        };
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
