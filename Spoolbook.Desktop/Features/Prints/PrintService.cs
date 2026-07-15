using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;
namespace Spoolbook.Desktop.Features.Prints;

public class PrintInput
{
    public DateTime StartedAt { get; set; }
    public DateTime EndedAt { get; set; }
    public PrintStatus Status { get; set; }
    public string? Notes { get; set; }
    public int? AmsHumidityPct { get; set; }
    public decimal? ActualRoomTempC { get; set; }
    public bool? CleanBuildPlate { get; set; }
    public int? ProjectId { get; set; }
}

public class PrintResult
{
    public bool Ok { get; init; }
    public Print? Print { get; init; }
    public string? Error { get; init; }
}

public class PrintService
{
    private readonly SpoolbookDbContext _db;
    private readonly IWeatherService _weatherService;

    public PrintService(SpoolbookDbContext db, IWeatherService weatherService)
    {
        _db = db;
        _weatherService = weatherService;
    }

    public async Task<List<Print>> ListAsync() =>
        await _db.Prints
            .Include(p => p.Profile)
            .Include(p => p.Spool).ThenInclude(s => s!.Filament)
            .Include(p => p.Printer)
            .Include(p => p.Project)
            .OrderByDescending(p => p.StartedAt)
            .ToListAsync();

    public async Task<Print?> GetAsync(int id) =>
        await _db.Prints
            .Include(p => p.Profile)
            .Include(p => p.Spool).ThenInclude(s => s!.Filament)
            .Include(p => p.Printer)
            .Include(p => p.Project)
            .FirstOrDefaultAsync(p => p.Id == id);

    public async Task<PrintResult> CreateAsync(int profileId, int spoolId, int printerId, PrintInput input)
    {
        var (tempC, humidityPct) = await _weatherService.GetAmbientAsync(input.StartedAt, input.EndedAt);

        var print = new Print
        {
            ProfileId = profileId,
            SpoolId = spoolId,
            PrinterId = printerId,
            ProjectId = input.ProjectId,
            StartedAt = input.StartedAt,
            EndedAt = input.EndedAt,
            Status = input.Status,
            Notes = input.Notes,
            AmsHumidityPct = input.AmsHumidityPct,
            ActualRoomTempC = input.ActualRoomTempC,
            CleanBuildPlate = input.CleanBuildPlate,
            AmbientTempC = tempC,
            AmbientHumidityPct = humidityPct,
            AmbientSource = tempC is not null ? AmbientSource.WeatherApi : null
        };

        _db.Prints.Add(print);
        await _db.SaveChangesAsync();

        return new PrintResult { Ok = true, Print = print };
    }

    public async Task<PrintResult> UpdateAsync(int id, int printerId, PrintInput input)
    {
        var print = await _db.Prints.FindAsync(id);
        if (print is null) throw new InvalidOperationException("Print not found");

        var (tempC, humidityPct) = await _weatherService.GetAmbientAsync(input.StartedAt, input.EndedAt);

        print.PrinterId = printerId;
        print.ProjectId = input.ProjectId;
        print.StartedAt = input.StartedAt;
        print.EndedAt = input.EndedAt;
        print.Status = input.Status;
        print.Notes = input.Notes;
        print.AmsHumidityPct = input.AmsHumidityPct;
        print.ActualRoomTempC = input.ActualRoomTempC;
        print.CleanBuildPlate = input.CleanBuildPlate;
        print.AmbientTempC = tempC;
        print.AmbientHumidityPct = humidityPct;
        print.AmbientSource = tempC is not null ? AmbientSource.WeatherApi : null;

        await _db.SaveChangesAsync();

        return new PrintResult { Ok = true, Print = print };
    }

    public async Task<PrintResult> DeleteAsync(int id)
    {
        var print = await _db.Prints.FindAsync(id);
        if (print is null) throw new InvalidOperationException("Print not found");

        _db.Prints.Remove(print);
        await _db.SaveChangesAsync();

        return new PrintResult { Ok = true };
    }
}
