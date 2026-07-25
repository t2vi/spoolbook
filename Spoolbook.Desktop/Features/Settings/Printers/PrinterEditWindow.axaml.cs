using Avalonia.Controls;
using Spoolbook.Desktop.Common;
namespace Spoolbook.Desktop.Features.Settings.Printers;

public partial class PrinterEditWindow : Window
{
    public PrinterEditWindow()
    {
        InitializeComponent();
        DataContextChanged += (_, _) =>
        {
            if (DataContext is PrinterEditViewModel vm)
            {
                vm.Close = () => Close();
                EditWindowEscape.Attach(this, () => vm.IsDirty, () => vm.SaveCommand.ExecuteAsync(null), () => vm.CancelCommand.Execute(null));
            }
        };
    }
}
