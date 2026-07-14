using Avalonia.Controls;
namespace Spoolbook.Desktop.Features.Spools;

public partial class SpoolEditWindow : Window
{
    public SpoolEditWindow()
    {
        InitializeComponent();
        DataContextChanged += (_, _) =>
        {
            if (DataContext is SpoolEditViewModel vm)
                vm.Close = () => Close();
        };
    }
}
