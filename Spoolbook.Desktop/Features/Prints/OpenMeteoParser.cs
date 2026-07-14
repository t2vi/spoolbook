using System.Text.Json;

namespace Spoolbook.Desktop.Features.Prints;

public static class OpenMeteoParser
{
    public static (decimal? TempC, decimal? HumidityPct) ParseAmbient(string json, DateTime start, DateTime end)
    {
        JsonDocument doc;
        try
        {
            doc = JsonDocument.Parse(json);
        }
        catch (JsonException)
        {
            return (null, null);
        }

        if (!doc.RootElement.TryGetProperty("hourly", out var hourly)) return (null, null);
        if (!hourly.TryGetProperty("time", out var times)) return (null, null);
        if (!hourly.TryGetProperty("temperature_2m", out var temps)) return (null, null);
        if (!hourly.TryGetProperty("relative_humidity_2m", out var humidities)) return (null, null);

        var matchedTemps = new List<decimal>();
        var matchedHumidities = new List<decimal>();

        for (var i = 0; i < times.GetArrayLength(); i++)
        {
            if (!DateTime.TryParse(times[i].GetString(), out var hour)) continue;
            if (hour < start || hour > end) continue;

            matchedTemps.Add(temps[i].GetDecimal());
            matchedHumidities.Add(humidities[i].GetDecimal());
        }

        if (matchedTemps.Count == 0) return (null, null);

        return (matchedTemps.Average(), matchedHumidities.Average());
    }
}
