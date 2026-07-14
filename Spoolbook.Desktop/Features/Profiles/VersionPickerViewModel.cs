using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
namespace Spoolbook.Desktop.Features.Profiles;

public partial class VersionPickerViewModel : ViewModelBase
{
    public IReadOnlyList<PrintProfile> Versions { get; }

    public Action<PrintProfile?>? Close { get; set; }
    public Func<PrintProfile, Task>? RenameHandler { get; set; }

    public VersionPickerViewModel(IReadOnlyList<PrintProfile> versions)
    {
        Versions = versions;
    }

    [RelayCommand]
    private void Select(PrintProfile version) => Close?.Invoke(version);

    [RelayCommand]
    private void Cancel() => Close?.Invoke(null);

    [RelayCommand]
    private async Task Rename(PrintProfile version)
    {
        if (RenameHandler is not null) await RenameHandler(version);
    }
}
