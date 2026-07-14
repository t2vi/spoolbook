using Avalonia.Media;
using Spoolbook.Desktop.Converters;

namespace Spoolbook.Desktop.Tests;

public class HexToBrushConverterTests
{
    [Fact]
    public void Convert_SingleHex_ReturnsSolidColorBrush()
    {
        var brush = HexToBrushConverter.Instance.Convert("#1A1A1A", typeof(IBrush), null, null!);

        var solid = Assert.IsType<SolidColorBrush>(brush);
        Assert.Equal(Color.Parse("#1A1A1A"), solid.Color);
    }

    [Fact]
    public void Convert_MultiHex_ReturnsDiagonalGradientWithHardStopsPerColor()
    {
        var brush = HexToBrushConverter.Instance.Convert("#1A1A1A, #D4AF37", typeof(IBrush), null, null!);

        var gradient = Assert.IsType<LinearGradientBrush>(brush);
        // Hard-edge diagonal split: two stops per color at matching offsets, no blending.
        Assert.Equal(4, gradient.GradientStops.Count);
        Assert.Equal(Color.Parse("#1A1A1A"), gradient.GradientStops[0].Color);
        Assert.Equal(0.0, gradient.GradientStops[0].Offset);
        Assert.Equal(Color.Parse("#1A1A1A"), gradient.GradientStops[1].Color);
        Assert.Equal(0.5, gradient.GradientStops[1].Offset);
        Assert.Equal(Color.Parse("#D4AF37"), gradient.GradientStops[2].Color);
        Assert.Equal(0.5, gradient.GradientStops[2].Offset);
        Assert.Equal(Color.Parse("#D4AF37"), gradient.GradientStops[3].Color);
        Assert.Equal(1.0, gradient.GradientStops[3].Offset);
    }

    [Fact]
    public void Convert_InvalidHex_ReturnsTransparent()
    {
        var brush = HexToBrushConverter.Instance.Convert("not-a-color", typeof(IBrush), null, null!);

        Assert.Equal(Brushes.Transparent, brush);
    }
}
