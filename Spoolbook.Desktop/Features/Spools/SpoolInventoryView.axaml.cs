using Avalonia.Controls;
using Avalonia.Interactivity;

namespace Spoolbook.Desktop.Features.Spools;

public partial class SpoolInventoryView : UserControl
{
    public SpoolInventoryView()
    {
        InitializeComponent();
    }

    private async void OnAddSpoolClick(object? sender, RoutedEventArgs e) => await OpenEditAsync(null);

    private async void OnEditSpoolClick(object? sender, RoutedEventArgs e)
    {
        if ((sender as Button)?.Tag is Spool entry)
            await OpenEditAsync(entry);
    }

    private async Task OpenEditAsync(Spool? existing)
    {
        if (DataContext is not SpoolInventoryViewModel vm) return;
        if (TopLevel.GetTopLevel(this) is not Window owner) return;

        var window = new SpoolEditWindow { DataContext = vm.CreateEditViewModel(existing) };
        await window.ShowDialog(owner);
        await vm.ReloadAsync();
    }
}
