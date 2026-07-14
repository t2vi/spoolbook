using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Features.Settings.Printers;
namespace Spoolbook.Desktop.Features.Prints;

public enum PrintStatus { Success, Failed, Partial }
public enum AmbientSource { WeatherApi, Sensor, Manual }

public class Print
{
    public int Id { get; set; }
    public int ProfileId { get; set; }
    public PrintProfile? Profile { get; set; }
    public int SpoolId { get; set; }
    public Spool? Spool { get; set; }
    public int PrinterId { get; set; }
    public Printer? Printer { get; set; }
    public DateTime StartedAt { get; set; }
    public DateTime EndedAt { get; set; }
    public PrintStatus Status { get; set; }
    public string? Notes { get; set; }
    public decimal? AmbientTempC { get; set; }
    public decimal? AmbientHumidityPct { get; set; }
    public AmbientSource? AmbientSource { get; set; }
    public int? AmsHumidityPct { get; set; }
    public decimal? ActualRoomTempC { get; set; }
    public bool? CleanBuildPlate { get; set; }
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
}
