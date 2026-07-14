using Avalonia.Controls;
namespace Spoolbook.Desktop.Features.Settings.Printers;

public partial class PrinterEditWindow : Window
{
    public PrinterEditWindow()
    {
        InitializeComponent();
        DataContextChanged += (_, _) =>
        {
            if (DataContext is PrinterEditViewModel vm)
                vm.Close = () => Close();
        };
    }
}
