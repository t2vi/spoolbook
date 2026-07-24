using Xunit.Abstractions;
using Spoolbook.Desktop.Features.Profiles;
namespace Spoolbook.Desktop.Tests;

public class SpecCoverageTests
{
    private readonly ITestOutputHelper _output;
    public SpecCoverageTests(ITestOutputHelper output) => _output = output;

    [Fact]
    public void ProfileFieldSpec_CoversAllDynamicProfileFields()
    {
        var dynamicFieldNames = ProfileFieldMapper.DynamicProperties.Select(p => p.Name).ToHashSet();
        var specFieldNames = ProfileFieldSpec.BuildGroups(null)
            .SelectMany(t => t.Sections)
            .SelectMany(g => g.Fields)
            .Select(f => f.Name)
            .ToHashSet();

        var missing = dynamicFieldNames.Except(specFieldNames).ToList();
        var extra = specFieldNames.Except(dynamicFieldNames).ToList();

        _output.WriteLine($"dynamic: {dynamicFieldNames.Count}, spec: {specFieldNames.Count}");
        _output.WriteLine($"missing: {string.Join(", ", missing)}");
        _output.WriteLine($"extra: {string.Join(", ", extra)}");

        Assert.Empty(missing);
        Assert.Empty(extra);
    }

    [Fact]
    public void ProfileFieldSpec_SplitsUnitSuffixFromLabel()
    {
        var fields = ProfileFieldSpec.BuildGroups(null)
            .SelectMany(t => t.Sections)
            .SelectMany(g => g.Fields)
            .ToDictionary(f => f.Name);

        var density = fields["DensityGCm3"];
        Assert.Equal("Density", density.Label);
        Assert.Equal("g/cm³", density.Unit);

        // "(experimental)" is a qualifier, not a unit — must stay in the label.
        var longRetraction = fields["LongRetractionsWhenCut"];
        Assert.Equal("Long retraction when cut (experimental)", longRetraction.Label);
        Assert.Equal("", longRetraction.Unit);

        // Fields with no parenthetical suffix get no unit.
        var hrc = fields["RequiredNozzleHrc"];
        Assert.Equal("Required nozzle HRC", hrc.Label);
        Assert.Equal("", hrc.Unit);
    }
}
