namespace Spoolbook.Desktop.Features.Settings.Filaments;

public class Filament
{
    public int Id { get; set; }
    public required string Brand { get; set; }
    public required string Material { get; set; }
    public string? Variant { get; set; }
    public required string Color { get; set; }

    public override string ToString() =>
        string.IsNullOrEmpty(Variant) ? $"{Brand} {Material} - {Color}" : $"{Brand} {Material} {Variant} - {Color}";
}
