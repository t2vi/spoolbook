using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;
namespace Spoolbook.Desktop.Features.Spools;

public class SpoolInput
{
    public string? LotCode { get; set; }
    public DateOnly? PurchasedAt { get; set; }
    public DateOnly? OpenedAt { get; set; }
    public DateOnly? EmptiedAt { get; set; }
    public int? WeightGrams { get; set; }
    public decimal? DiameterMm { get; set; }
    public string? Notes { get; set; }
}

public class SpoolResult
{
    public bool Ok { get; init; }
    public Spool? Spool { get; init; }
    public string? Error { get; init; }
}

public class SpoolService
{
    private readonly SpoolbookDbContext _db;

    public SpoolService(SpoolbookDbContext db)
    {
        _db = db;
    }

    public async Task<List<Spool>> ListSpoolsForFilamentAsync(int filamentId)
    {
        return await _db.Spools
            .Where(s => s.FilamentId == filamentId)
            .OrderBy(s => s.CreatedAt)
            .ToListAsync();
    }

    public async Task<List<Spool>> ListAllAsync()
    {
        return await _db.Spools
            .Include(s => s.Filament)
            .OrderBy(s => s.CreatedAt)
            .ToListAsync();
    }

    public async Task<Spool?> GetSpoolAsync(int id)
    {
        return await _db.Spools.FindAsync(id);
    }

    public async Task<SpoolResult> CreateSpoolAsync(int filamentId, SpoolInput input)
    {
        var spool = new Spool
        {
            FilamentId = filamentId,
            LotCode = input.LotCode,
            PurchasedAt = input.PurchasedAt,
            OpenedAt = input.OpenedAt,
            EmptiedAt = input.EmptiedAt,
            WeightGrams = input.WeightGrams,
            DiameterMm = input.DiameterMm,
            Notes = input.Notes
        };
        _db.Spools.Add(spool);
        await _db.SaveChangesAsync();

        return new SpoolResult { Ok = true, Spool = spool };
    }

    public async Task<SpoolResult> UpdateSpoolAsync(int id, SpoolInput input)
    {
        var spool = await _db.Spools.FindAsync(id);
        if (spool is null) throw new InvalidOperationException("Spool not found");

        spool.LotCode = input.LotCode;
        spool.PurchasedAt = input.PurchasedAt;
        spool.OpenedAt = input.OpenedAt;
        spool.EmptiedAt = input.EmptiedAt;
        spool.WeightGrams = input.WeightGrams;
        spool.DiameterMm = input.DiameterMm;
        spool.Notes = input.Notes;
        await _db.SaveChangesAsync();

        return new SpoolResult { Ok = true, Spool = spool };
    }

    public async Task<SpoolResult> DeleteSpoolAsync(int id)
    {
        var spool = await _db.Spools.FindAsync(id);
        if (spool is null) throw new InvalidOperationException("Spool not found");

        if (await _db.PrintProfiles.AnyAsync(p => p.SpoolId == id))
            return new SpoolResult { Ok = false, Error = "has_profiles" };
        if (await _db.Prints.AnyAsync(p => p.SpoolId == id))
            return new SpoolResult { Ok = false, Error = "has_prints" };

        _db.Spools.Remove(spool);
        await _db.SaveChangesAsync();

        return new SpoolResult { Ok = true };
    }
}
