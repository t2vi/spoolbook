using Spoolbook.Desktop.Features.Settings.Filaments;
namespace Spoolbook.Desktop.Tests;

public class FilamentCatalogParserTests
{
    [Fact]
    public void Parse_ReturnsEntries()
    {
        const string json = """
            [
              { "Brand": "Bambu Lab", "Material": "PLA", "Variant": "Basic", "Color": "Black" },
              { "Brand": "Protopasta", "Material": "HTPLA", "Variant": null, "Color": "Natural" }
            ]
            """;

        var entries = FilamentCatalogParser.Parse(json);

        Assert.Equal(2, entries.Count);
        Assert.Equal("Bambu Lab", entries[0].Brand);
        Assert.Equal("Basic", entries[0].Variant);
        Assert.Null(entries[1].Variant);
    }

    [Fact]
    public void Parse_EmptyArray_ReturnsEmptyList()
    {
        Assert.Empty(FilamentCatalogParser.Parse("[]"));
    }
}
