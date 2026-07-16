using System.Collections.Concurrent;
using System.Globalization;
using Avalonia;
using Avalonia.Data.Converters;
using Avalonia.Media;
using Spoolbook.Desktop.Features.Settings.Colors;
namespace Spoolbook.Desktop.Converters;

public class ColorSwatchConverter : IValueConverter
{
    public static readonly ColorSwatchConverter Instance = new();

    private static readonly ConcurrentDictionary<string, string> HexByName = new();

    public static void SetPalette(IEnumerable<FilamentColor> colors)
    {
        foreach (var color in colors)
            HexByName[color.Name] = color.Hex;
    }

    public static bool TryGetHex(string name, out string hex) => HexByName.TryGetValue(name, out hex!);

    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        if (value is not string name || !HexByName.TryGetValue(name, out var hex)) return Brushes.Transparent;
        return HexBrush.Parse(hex);
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        throw new NotSupportedException();
}

// Swatch borders default to mid-gray, which is nearly invisible against a dark theme when
// the swatch itself is a near-black color (e.g. "Black") — swap to a lighter border for dark
// fills so every swatch stays visible regardless of how dark its color is.
public class ColorSwatchBorderConverter : IValueConverter
{
    public static readonly ColorSwatchBorderConverter Instance = new();

    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        if (value is not string name || !ColorSwatchConverter.TryGetHex(name, out var hex)) return Brushes.Gray;
        return HexBrush.BorderFor(hex);
    }

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        throw new NotSupportedException();
}

internal static class HexBrush
{
    private static string[] SplitParts(string hex) =>
        hex.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);

    public static IBrush Parse(string hex)
    {
        var parts = SplitParts(hex);
        try
        {
            if (parts.Length <= 1) return new SolidColorBrush(Color.Parse(hex));

            // Diagonal hard-stop split (two stops per color, same offset) so each color
            // renders as a flat wedge rather than blending into its neighbor.
            var colors = parts.Select(Color.Parse).ToArray();
            var brush = new LinearGradientBrush
            {
                StartPoint = new RelativePoint(0, 0, RelativeUnit.Relative),
                EndPoint = new RelativePoint(1, 1, RelativeUnit.Relative)
            };
            for (var i = 0; i < colors.Length; i++)
            {
                brush.GradientStops.Add(new GradientStop(colors[i], (double)i / colors.Length));
                brush.GradientStops.Add(new GradientStop(colors[i], (double)(i + 1) / colors.Length));
            }
            return brush;
        }
        catch (FormatException)
        {
            return Brushes.Transparent;
        }
    }

    public static IBrush BorderFor(string hex)
    {
        try
        {
            var avgLuminance = SplitParts(hex).Select(Color.Parse)
                .Average(c => (0.299 * c.R + 0.587 * c.G + 0.114 * c.B) / 255.0);
            return avgLuminance < 0.3 ? Brushes.LightGray : Brushes.Gray;
        }
        catch (FormatException)
        {
            return Brushes.Gray;
        }
    }
}
