namespace Spoolbook.Desktop.Features.Settings.Colors;

public class FilamentColor
{
    public int Id { get; set; }
    public required string Name { get; set; }
    public required string Hex { get; set; }

    // Multi-color filaments (e.g. "Black+Gold") store their hex list comma-separated in the
    // same column — everything else is just a 1-element list.
    public IReadOnlyList<string> Hexes =>
        Hex.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
}
