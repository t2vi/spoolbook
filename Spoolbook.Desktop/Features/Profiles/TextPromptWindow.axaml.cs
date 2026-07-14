using Avalonia.Controls;
namespace Spoolbook.Desktop.Features.Profiles;

public partial class TextPromptWindow : Window
{
    public TextPromptWindow()
    {
        InitializeComponent();
        DataContextChanged += (_, _) =>
        {
            if (DataContext is TextPromptViewModel vm)
                vm.Close = result => Close(result);
        };
    }
}
