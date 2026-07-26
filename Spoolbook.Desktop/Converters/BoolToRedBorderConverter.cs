using System.Globalization;
using Avalonia.Data.Converters;
using Avalonia.Media;

namespace Spoolbook.Desktop.Converters;

public class BoolToRedBorderConverter : IValueConverter
{
    public static readonly BoolToRedBorderConverter Instance = new();

    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        value is true ? Brushes.Red : Brushes.Transparent;

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture) =>
        throw new NotSupportedException();
}
