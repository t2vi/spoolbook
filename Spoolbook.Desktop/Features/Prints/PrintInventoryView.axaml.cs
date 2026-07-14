using Avalonia.Controls;
using Avalonia.Interactivity;

namespace Spoolbook.Desktop.Features.Prints;

public partial class PrintInventoryView : UserControl
{
    public PrintInventoryView()
    {
        InitializeComponent();
    }

    private async void OnAddPrintClick(object? sender, RoutedEventArgs e) => await OpenEditAsync(null);

    private async void OnEditPrintClick(object? sender, RoutedEventArgs e)
    {
        if ((sender as Button)?.Tag is Print entry)
            await OpenEditAsync(entry);
    }

    private async Task OpenEditAsync(Print? existing)
    {
        if (DataContext is not PrintInventoryViewModel vm) return;
        if (TopLevel.GetTopLevel(this) is not Window owner) return;

        var window = new PrintEditWindow { DataContext = vm.CreateEditViewModel(existing) };
        await window.ShowDialog(owner);
        await vm.ReloadAsync();
    }
}
