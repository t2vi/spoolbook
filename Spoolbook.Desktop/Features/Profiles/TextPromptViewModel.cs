using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
namespace Spoolbook.Desktop.Features.Profiles;

public partial class TextPromptViewModel : ViewModelBase
{
    [ObservableProperty]
    private string title = "";

    [ObservableProperty]
    private string text = "";

    public Action<string?>? Close { get; set; }

    [RelayCommand]
    private void Ok() => Close?.Invoke(Text);

    [RelayCommand]
    private void Cancel() => Close?.Invoke(null);
}
