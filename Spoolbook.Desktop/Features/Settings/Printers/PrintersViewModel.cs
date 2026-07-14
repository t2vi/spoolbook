using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
namespace Spoolbook.Desktop.Features.Settings.Printers;

public partial class PrintersViewModel : ViewModelBase
{
    private readonly PrinterService _printerService;

    [ObservableProperty]
    private ObservableCollection<Printer> printers = new();

    [ObservableProperty]
    private string? errorMessage;

    public PrintersViewModel(PrinterService printerService)
    {
        _printerService = printerService;
        _ = ReloadAsync();
    }

    public async Task ReloadAsync()
    {
        Printers = new ObservableCollection<Printer>(await _printerService.ListAsync());
    }

    public PrinterEditViewModel CreateEditViewModel(Printer? existing) => new(_printerService, existing);

    [RelayCommand]
    private async Task DeleteAsync(Printer printer)
    {
        var result = await _printerService.DeleteAsync(printer.Id);
        if (!result.Ok)
        {
            ErrorMessage = result.Error == "has_prints" ? "Can't delete — prints exist for this printer." : result.Error;
            return;
        }

        ErrorMessage = null;
        await ReloadAsync();
    }
}
