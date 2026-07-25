using Avalonia.Controls;
using Avalonia.Interactivity;
namespace Spoolbook.Desktop.Common;

// KeepEditing is first (default(T) == 0) so dismissing the window any other way
// (e.g. the titlebar close button) can't accidentally discard or save.
public enum UnsavedChangesChoice { KeepEditing, Discard, Save }

public partial class UnsavedChangesWindow : Window
{
    public UnsavedChangesWindow()
    {
        InitializeComponent();
    }

    private void OnKeepEditingClick(object? sender, RoutedEventArgs e) => Close(UnsavedChangesChoice.KeepEditing);
    private void OnDiscardClick(object? sender, RoutedEventArgs e) => Close(UnsavedChangesChoice.Discard);
    private void OnSaveClick(object? sender, RoutedEventArgs e) => Close(UnsavedChangesChoice.Save);
}
