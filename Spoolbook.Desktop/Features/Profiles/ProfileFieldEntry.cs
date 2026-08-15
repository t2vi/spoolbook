namespace Spoolbook.Desktop.Features.Profiles;

// Pure data shape — serialized directly as JSON by Spoolbook.Web's /api/profiles/field-spec
// endpoint (ProfileEndpoints.cs). No observable/two-way-binding machinery needed since nothing
// mutates an instance after construction; the Svelte client owns its own edit-time state.
public class ProfileFieldEntry
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

    public required string Value { get; init; }
    public bool BoolValue => Value == "true";
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
