namespace Spoolbook.Desktop.Features.Settings.Printers;

public class Printer
{
    public int Id { get; set; }
    public required string Name { get; set; }
    public string? Model { get; set; }
}
