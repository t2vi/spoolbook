namespace Spoolbook.Desktop.Features.Prints;

// Melbourne lat/long hardcoded — single-user app, no location picker.
public class OpenMeteoWeatherService : IWeatherService
{
    private const decimal Latitude = -37.8136m;
    private const decimal Longitude = 144.9631m;

    private readonly HttpClient _http = new();

    public async Task<(decimal? TempC, decimal? HumidityPct)> GetAmbientAsync(DateTime startedAt, DateTime endedAt)
    {
        var url = "https://archive-api.open-meteo.com/v1/archive" +
                  $"?latitude={Latitude}&longitude={Longitude}" +
                  $"&start_date={startedAt:yyyy-MM-dd}&end_date={endedAt:yyyy-MM-dd}" +
                  "&hourly=temperature_2m,relative_humidity_2m&timezone=Australia%2FMelbourne";

        try
        {
            var json = await _http.GetStringAsync(url);
            return OpenMeteoParser.ParseAmbient(json, startedAt, endedAt);
        }
        catch (HttpRequestException)
        {
            return (null, null);
        }
    }
}
