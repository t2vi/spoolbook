namespace Spoolbook.Desktop.Features.Settings.Printers;

public class Printer
{
    public int Id { get; set; }
    public required string Name { get; set; }
    public string? Model { get; set; }

    // LAN-mode MQTT connection (Bambu Studio's Developer Mode credentials) — optional, only
    // needed for printers you want live telemetry from. Plaintext: see docs/adr/0017.
    public string? IpAddress { get; set; }
    public string? AccessCode { get; set; }
    public string? SerialNumber { get; set; }
}
