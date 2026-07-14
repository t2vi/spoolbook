using Avalonia.Controls;
namespace Spoolbook.Desktop.Features.Settings.Filaments;

public partial class FilamentEditWindow : Window
{
    public FilamentEditWindow()
    {
        InitializeComponent();
        DataContextChanged += (_, _) =>
        {
            if (DataContext is FilamentEditViewModel vm)
                vm.Close = () => Close();
        };
    }
}
