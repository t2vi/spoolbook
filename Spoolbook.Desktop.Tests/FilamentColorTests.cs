using Spoolbook.Desktop.Features.Settings.Colors;

namespace Spoolbook.Desktop.Tests;

public class FilamentColorTests
{
    [Fact]
    public void Hexes_SingleColor_ReturnsOneElementList()
    {
        var color = new FilamentColor { Name = "Black", Hex = "#1A1A1A" };

        Assert.Equal(["#1A1A1A"], color.Hexes);
    }

    [Fact]
    public void Hexes_MultiColor_SplitsAndTrimsCommaSeparatedValues()
    {
        var color = new FilamentColor { Name = "Black+Gold", Hex = "#1A1A1A, #D4AF37" };

        Assert.Equal(["#1A1A1A", "#D4AF37"], color.Hexes);
    }
}
