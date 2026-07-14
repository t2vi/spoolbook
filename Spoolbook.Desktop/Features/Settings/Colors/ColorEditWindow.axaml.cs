using Avalonia.Controls;
namespace Spoolbook.Desktop.Features.Settings.Colors;

public partial class ColorEditWindow : Window
{
    public ColorEditWindow()
    {
        InitializeComponent();
        DataContextChanged += (_, _) =>
        {
            if (DataContext is ColorEditViewModel vm)
                vm.Close = () => Close();
        };
    }
}
