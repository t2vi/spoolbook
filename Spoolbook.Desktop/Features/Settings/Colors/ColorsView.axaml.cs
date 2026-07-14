using Avalonia.Controls;
using Avalonia.Interactivity;

namespace Spoolbook.Desktop.Features.Settings.Colors;

public partial class ColorsView : UserControl
{
    public ColorsView()
    {
        InitializeComponent();
    }

    private async void OnAddColorClick(object? sender, RoutedEventArgs e) => await OpenColorEditAsync(null);

    private async void OnEditColorClick(object? sender, RoutedEventArgs e)
    {
        if ((sender as Button)?.Tag is FilamentColor color)
            await OpenColorEditAsync(color);
    }

    private async Task OpenColorEditAsync(FilamentColor? existing)
    {
        if (DataContext is not ColorsViewModel vm) return;
        if (TopLevel.GetTopLevel(this) is not Window owner) return;

        var window = new ColorEditWindow { DataContext = vm.CreateEditViewModel(existing) };
        await window.ShowDialog(owner);
        await vm.ReloadAsync();
    }
}
