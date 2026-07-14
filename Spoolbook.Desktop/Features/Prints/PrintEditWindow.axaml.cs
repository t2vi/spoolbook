using Avalonia.Controls;
namespace Spoolbook.Desktop.Features.Prints;

public partial class PrintEditWindow : Window
{
    public PrintEditWindow()
    {
        InitializeComponent();
        DataContextChanged += (_, _) =>
        {
            if (DataContext is PrintEditViewModel vm)
                vm.Close = () => Close();
        };
    }
}
