using Avalonia.Controls;
using Avalonia.Input;
namespace Spoolbook.Desktop.Features.Profiles;

public partial class TextPromptWindow : Window
{
    public TextPromptWindow()
    {
        InitializeComponent();
        DataContextChanged += (_, _) =>
        {
            if (DataContext is TextPromptViewModel vm)
            {
                vm.Close = result => Close(result);
                KeyDown += (_, e) =>
                {
                    if (e.Key != Key.Escape) return;
                    e.Handled = true;
                    vm.CancelCommand.Execute(null);
                };
            }
        };
    }
}
