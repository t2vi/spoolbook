using Avalonia.Controls;
using Avalonia.Interactivity;

namespace Spoolbook.Desktop.Features.Settings.Printers;

public partial class PrintersView : UserControl
{
    public PrintersView()
    {
        InitializeComponent();
    }

    private async void OnAddPrinterClick(object? sender, RoutedEventArgs e) => await OpenPrinterEditAsync(null);

    private async void OnEditPrinterClick(object? sender, RoutedEventArgs e)
    {
        if ((sender as Button)?.Tag is Printer printer)
            await OpenPrinterEditAsync(printer);
    }

    private async Task OpenPrinterEditAsync(Printer? existing)
    {
        if (DataContext is not PrintersViewModel vm) return;
        if (TopLevel.GetTopLevel(this) is not Window owner) return;

        var window = new PrinterEditWindow { DataContext = vm.CreateEditViewModel(existing) };
        await window.ShowDialog(owner);
        await vm.ReloadAsync();
    }
}
