using Avalonia.Controls;
namespace Spoolbook.Desktop.Features.Profiles;

public partial class VersionPickerWindow : Window
{
    public VersionPickerWindow()
    {
        InitializeComponent();
        DataContextChanged += (_, _) =>
        {
            if (DataContext is VersionPickerViewModel vm)
                vm.Close = result => Close(result);
        };
    }
}
