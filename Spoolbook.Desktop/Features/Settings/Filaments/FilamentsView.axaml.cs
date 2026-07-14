using Avalonia.Controls;
using Avalonia.Interactivity;

namespace Spoolbook.Desktop.Features.Settings.Filaments;

public partial class FilamentsView : UserControl
{
    public FilamentsView()
    {
        InitializeComponent();
    }

    private async void OnAddFilamentClick(object? sender, RoutedEventArgs e) => await OpenEditAsync(null);

    private async void OnEditFilamentClick(object? sender, RoutedEventArgs e)
    {
        if ((sender as Button)?.Tag is Filament entry)
            await OpenEditAsync(entry);
    }

    private async Task OpenEditAsync(Filament? existing)
    {
        if (DataContext is not FilamentsViewModel vm) return;
        if (TopLevel.GetTopLevel(this) is not Window owner) return;

        var window = new FilamentEditWindow { DataContext = vm.CreateEditViewModel(existing) };
        await window.ShowDialog(owner);
        await vm.ReloadAsync();
    }
}
