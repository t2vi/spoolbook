using Avalonia.Controls;
using Spoolbook.Desktop.Common;
namespace Spoolbook.Desktop.Features.Spools;

public partial class SpoolEditWindow : Window
{
    public SpoolEditWindow()
    {
        InitializeComponent();
        DataContextChanged += (_, _) =>
        {
            if (DataContext is SpoolEditViewModel vm)
            {
                vm.Close = () => Close();
                EditWindowEscape.Attach(this, () => vm.IsDirty, () => vm.SaveCommand.ExecuteAsync(null), () => vm.CancelCommand.Execute(null));
            }
        };
    }
}
