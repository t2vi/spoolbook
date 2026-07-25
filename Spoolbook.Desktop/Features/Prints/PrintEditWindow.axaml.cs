using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using Spoolbook.Desktop.Common;
namespace Spoolbook.Desktop.Features.Prints;

public partial class PrintEditWindow : Window
{
    public PrintEditWindow()
    {
        InitializeComponent();
        DataContextChanged += (_, _) =>
        {
            if (DataContext is PrintEditViewModel vm)
            {
                vm.Close = () => Close();
                EditWindowEscape.Attach(this, () => vm.IsDirty, () => vm.SaveCommand.ExecuteAsync(null), () => vm.CancelCommand.Execute(null));
            }
        };
    }

    private async void OnBrowseProjectClick(object? sender, RoutedEventArgs e)
    {
        var topLevel = TopLevel.GetTopLevel(this);
        if (topLevel is null) return;

        var files = await topLevel.StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = "Choose a .3mf project file to attach",
            AllowMultiple = false,
            FileTypeFilter = new[] { new FilePickerFileType("3MF project") { Patterns = new[] { "*.3mf" } } }
        });

        var file = files.FirstOrDefault();
        if (file is null) return;

        if (DataContext is PrintEditViewModel vm)
            await vm.AttachProjectFileCommand.ExecuteAsync(file.Path.LocalPath);
    }
}
