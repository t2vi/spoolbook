using CommunityToolkit.Mvvm.ComponentModel;
namespace Spoolbook.Desktop.Features.Profiles;

public partial class ProfileFieldEntry : ObservableObject
{
    public required string Name { get; init; }
    public required string Label { get; init; }
    public bool IsBool { get; init; }
    public bool IsTextArea { get; init; }

    [ObservableProperty]
    private string value = "";

    public bool BoolValue
    {
        get => Value == "true";
        set => Value = value ? "true" : "false";
    }

    partial void OnValueChanged(string value) => OnPropertyChanged(nameof(BoolValue));
}

public class ProfileFieldGroup
{
    public required string Title { get; init; }
    public required List<ProfileFieldEntry> Fields { get; init; }
    public bool IsPrintTemperatureSection => Title == "Print temperature";
}

public class ProfileFieldTab
{
    public required string Title { get; init; }
    public required List<ProfileFieldGroup> Sections { get; init; }
}
