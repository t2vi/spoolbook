using System.Globalization;
using Avalonia.Data.Converters;
using Avalonia.Media;

namespace Spoolbook.Desktop.Converters;

public class HexToBrushConverter : IValueConverter
{
    public static readonly HexToBrushConverter Instance = new();

    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        value is string hex && !string.IsNullOrWhiteSpace(hex) ? HexBrush.Parse(hex) : Brushes.Transparent;

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        throw new NotSupportedException();
}

public class HexToBorderBrushConverter : IValueConverter
{
    public static readonly HexToBorderBrushConverter Instance = new();

    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        value is string hex && !string.IsNullOrWhiteSpace(hex) ? HexBrush.BorderFor(hex) : Brushes.Gray;

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        throw new NotSupportedException();
}
