using Xunit.Abstractions;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Features.BambuImport;
namespace Spoolbook.Desktop.Tests;

public class FieldCoverageTests
{
    private readonly ITestOutputHelper _output;
    public FieldCoverageTests(ITestOutputHelper output) => _output = output;

    [Fact]
    public void KeyMap_CoversAllDynamicProfileFields_ExceptPrintSpeedMmS()
    {
        var dynamicFieldNames = ProfileFieldMapper.DynamicProperties.Select(p => p.Name).ToHashSet();
        dynamicFieldNames.Remove("PrintSpeedMmS"); // process-only, no filament-level source

        var mappedFieldsField = typeof(BambuFilamentImportService)
            .GetField("KeyMap", System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static)!;
        var keyMap = (Dictionary<string, string>)mappedFieldsField.GetValue(null)!;
        var mappedFields = keyMap.Values.ToHashSet();
        mappedFields.Remove("NozzleTempC");
        mappedFields.Remove("NozzleTempInitialC");

        var missing = dynamicFieldNames.Except(mappedFields).ToList();
        var extra = mappedFields.Except(dynamicFieldNames).ToList();

        _output.WriteLine($"dynamic: {dynamicFieldNames.Count}, mapped: {mappedFields.Count}");
        _output.WriteLine($"missing: {string.Join(", ", missing)}");
        _output.WriteLine($"extra: {string.Join(", ", extra)}");

        Assert.Empty(missing);
        Assert.Empty(extra);
    }
}
