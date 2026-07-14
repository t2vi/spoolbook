using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
namespace Spoolbook.Desktop.Features.Profiles;

public partial class ProfileEditWindow : Window
{
    public ProfileEditWindow()
    {
        InitializeComponent();
        DataContextChanged += (_, _) =>
        {
            if (DataContext is ProfileEditViewModel vm)
            {
                vm.Close = () => Close();
                vm.SwitchTo = newVm => DataContext = newVm;
            }
        };
    }

    private async void OnChooseFileClick(object? sender, RoutedEventArgs e)
    {
        var topLevel = TopLevel.GetTopLevel(this);
        if (topLevel is null) return;

        var files = await topLevel.StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Choose a Bambu Studio filament preset to link",
            AllowMultiple = false,
            FileTypeFilter = new[] { new FilePickerFileType("Preset JSON") { Patterns = new[] { "*.json" } } }
        });

        var file = files.FirstOrDefault();
        if (file is null) return;

        if (DataContext is ProfileEditViewModel vm)
        {
            await vm.LinkToFileCommand.ExecuteAsync(file.Path.LocalPath);
        }
    }

    private async void OnVersionsClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not ProfileEditViewModel vm) return;

        var pickerVm = new VersionPickerViewModel(vm.Versions)
        {
            RenameHandler = version => vm.RenameVersionAsync(version, version.VersionName)
        };
        var picker = new VersionPickerWindow { DataContext = pickerVm };
        var selected = await picker.ShowDialog<PrintProfile?>(this);
        if (selected is not null) await vm.LoadVersionAsync(selected);
    }

    private async void OnSaveClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not ProfileEditViewModel vm) return;

        if (vm.PendingNewVersion)
        {
            var name = await PromptForVersionNameAsync();
            if (string.IsNullOrWhiteSpace(name)) return;
            await vm.SaveAsNewVersionAsync(name);
        }
        else
        {
            await vm.SaveInPlaceAsync();
        }
    }

    private async void OnSaveAsNewVersionClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not ProfileEditViewModel vm) return;

        var name = await PromptForVersionNameAsync();
        if (string.IsNullOrWhiteSpace(name)) return;
        await vm.SaveAsNewVersionAsync(name);
    }

    private async Task<string?> PromptForVersionNameAsync()
    {
        var prompt = new TextPromptWindow
        {
            DataContext = new TextPromptViewModel { Title = "Name this version (e.g. \"Winter 2026\")" }
        };
        return await prompt.ShowDialog<string?>(this);
    }
}
