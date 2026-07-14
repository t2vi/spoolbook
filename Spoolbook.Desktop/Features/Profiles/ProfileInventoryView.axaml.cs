using Avalonia.Controls;
using Avalonia.Interactivity;

namespace Spoolbook.Desktop.Features.Profiles;

public partial class ProfileInventoryView : UserControl
{
    public ProfileInventoryView()
    {
        InitializeComponent();
    }

    private async void OnAddProfileClick(object? sender, RoutedEventArgs e) => await OpenEditAsync(null);

    private async void OnEditProfileClick(object? sender, RoutedEventArgs e)
    {
        if ((sender as Button)?.Tag is PrintProfile entry)
            await OpenEditAsync(entry);
    }

    private async Task OpenEditAsync(PrintProfile? existing)
    {
        if (DataContext is not ProfileInventoryViewModel vm) return;
        if (TopLevel.GetTopLevel(this) is not Window owner) return;

        var window = new ProfileEditWindow { DataContext = vm.CreateEditViewModel(existing) };
        await window.ShowDialog(owner);
        await vm.ReloadAsync();
    }
}
