using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;

namespace Spoolbook.Desktop.Features.Settings.General;

public partial class GeneralSettingsView : UserControl
{
    public GeneralSettingsView()
    {
        InitializeComponent();
    }

    private async void OnBrowseUserPresetsDirClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not GeneralSettingsViewModel vm) return;
        var folder = await PickFolderAsync();
        if (folder is not null) vm.BambuUserPresetsDir = folder;
    }

    private async void OnBrowseSystemProfilesDirClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not GeneralSettingsViewModel vm) return;
        var folder = await PickFolderAsync();
        if (folder is not null) vm.BambuSystemProfilesDir = folder;
    }

    private async Task<string?> PickFolderAsync()
    {
        var topLevel = TopLevel.GetTopLevel(this);
        if (topLevel is null) return null;

        var folders = await topLevel.StorageProvider.OpenFolderPickerAsync(new FolderPickerOpenOptions { AllowMultiple = false });
        return folders.Count > 0 ? folders[0].Path.LocalPath : null;
    }
}
