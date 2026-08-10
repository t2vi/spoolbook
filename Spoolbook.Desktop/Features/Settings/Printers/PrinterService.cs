using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;
namespace Spoolbook.Desktop.Features.Settings.Printers;

public class PrinterInput
{
    public required string Name { get; set; }
    public string? Model { get; set; }
    public string? IpAddress { get; set; }
    public string? AccessCode { get; set; }
    public string? SerialNumber { get; set; }
}

public class PrinterResult
{
    public bool Ok { get; init; }
    public Printer? Printer { get; init; }
    public string? Error { get; init; }
}

public class PrinterService
{
    private readonly SpoolbookDbContext _db;

    public PrinterService(SpoolbookDbContext db)
    {
        _db = db;
    }

    public async Task<List<Printer>> ListAsync() =>
        await _db.Printers.OrderBy(p => p.Name).ToListAsync();

    public async Task<PrinterResult> CreateAsync(PrinterInput input)
    {
        if (string.IsNullOrWhiteSpace(input.Name))
            return new PrinterResult { Ok = false, Error = "Name is required" };

        if (await _db.Printers.AnyAsync(p => p.Name == input.Name))
            return new PrinterResult { Ok = false, Error = "duplicate" };

        var printer = new Printer
        {
            Name = input.Name,
            Model = input.Model,
            IpAddress = input.IpAddress,
            AccessCode = input.AccessCode,
            SerialNumber = input.SerialNumber
        };
        _db.Printers.Add(printer);
        await _db.SaveChangesAsync();

        return new PrinterResult { Ok = true, Printer = printer };
    }

    public async Task<PrinterResult> UpdateAsync(int id, PrinterInput input)
    {
        if (string.IsNullOrWhiteSpace(input.Name))
            return new PrinterResult { Ok = false, Error = "Name is required" };

        if (await _db.Printers.AnyAsync(p => p.Name == input.Name && p.Id != id))
            return new PrinterResult { Ok = false, Error = "duplicate" };

        var printer = await _db.Printers.FindAsync(id);
        if (printer is null) throw new InvalidOperationException("Printer not found");

        printer.Name = input.Name;
        printer.Model = input.Model;
        printer.IpAddress = input.IpAddress;
        printer.AccessCode = input.AccessCode;
        printer.SerialNumber = input.SerialNumber;
        await _db.SaveChangesAsync();

        return new PrinterResult { Ok = true, Printer = printer };
    }

    public async Task<PrinterResult> DeleteAsync(int id)
    {
        var printer = await _db.Printers.FindAsync(id);
        if (printer is null) throw new InvalidOperationException("Printer not found");

        if (await _db.Prints.AnyAsync(p => p.PrinterId == id))
            return new PrinterResult { Ok = false, Error = "has_prints" };

        _db.Printers.Remove(printer);
        await _db.SaveChangesAsync();

        return new PrinterResult { Ok = true };
    }
}
