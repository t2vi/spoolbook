namespace Spoolbook.Desktop.Features.Prints;

public interface IWeatherService
{
    Task<(decimal? TempC, decimal? HumidityPct)> GetAmbientAsync(DateTime startedAt, DateTime endedAt);
}
