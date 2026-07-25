using CommunityToolkit.Mvvm.ComponentModel;
namespace Spoolbook.Desktop.Features.Profiles;

public partial class ProfileFieldEntry : ObservableObject
{
    public required string Name { get; init; }
    public required string Label { get; init; }
    public string Unit { get; init; } = "";
    public bool IsBool { get; init; }
    public bool IsTextArea { get; init; }
    public bool IsNumeric { get; init; }
    public IReadOnlyList<string>? Options { get; init; }
    public bool IsEnum => Options is not null;
    public bool IsPlainText => !IsBool && !IsEnum && !IsTextArea;

    // Some fields (e.g. Default color) are only ever set by Bambu's own first-party presets —
    // third-party/community presets leave them blank, so show the row only once there's a value.
    public bool HideWhenBlank { get; init; }
    public bool ShowRow => !HideWhenBlank || !string.IsNullOrWhiteSpace(Value);

    [ObservableProperty]
    private string value = "";

    public bool BoolValue
    {
        get => Value == "true";
        set => Value = value ? "true" : "false";
    }

    partial void OnValueChanged(string value)
    {
        OnPropertyChanged(nameof(BoolValue));
        OnPropertyChanged(nameof(ShowRow));
    }
}

public class ProfileFieldGroup
{
    public required string Title { get; init; }
    public required List<ProfileFieldEntry> Fields { get; init; }
}

public class ProfileFieldTab
{
    public required string Title { get; init; }
    public required List<ProfileFieldGroup> Sections { get; init; }
}
