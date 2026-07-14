using Spoolbook.Desktop.Features.Settings.Filaments;
namespace Spoolbook.Desktop.Features.Spools;

public class Spool
{
    public int Id { get; set; }
    public int FilamentId { get; set; }
    public Filament? Filament { get; set; }
    public string? LotCode { get; set; }
    public DateOnly? PurchasedAt { get; set; }
    public DateOnly? OpenedAt { get; set; }
    public DateOnly? EmptiedAt { get; set; }
    public int? WeightGrams { get; set; }
    public decimal? DiameterMm { get; set; }
    public string? Notes { get; set; }
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;

    public override string ToString() =>
        LotCode is null ? $"{Filament}" : $"{Filament} (Lot: {LotCode})";
}
