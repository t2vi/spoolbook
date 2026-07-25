namespace Spoolbook.Desktop.Services.Weather;

public interface IWeatherService
{
    Task<(decimal? TempC, decimal? HumidityPct)> GetAmbientAsync(DateTime startedAt, DateTime endedAt);
}
