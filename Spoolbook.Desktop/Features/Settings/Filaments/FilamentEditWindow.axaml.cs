using Avalonia.Controls;
using Spoolbook.Desktop.Common;
namespace Spoolbook.Desktop.Features.Settings.Filaments;

public partial class FilamentEditWindow : Window
{
    public FilamentEditWindow()
    {
        InitializeComponent();
        DataContextChanged += (_, _) =>
        {
            if (DataContext is FilamentEditViewModel vm)
            {
                vm.Close = () => Close();
                EditWindowEscape.Attach(this, () => vm.IsDirty, () => vm.SaveCommand.ExecuteAsync(null), () => vm.CancelCommand.Execute(null));
            }
        };
    }
}
