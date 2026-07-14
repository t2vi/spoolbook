namespace Spoolbook.Desktop.Features.Profiles;

// Mirrors Bambu Studio's own tab/section layout (Filament/Cooling/Setting Overrides/
// Advanced/Notes/Multi Filament), same grouping used in the SvelteKit version's
// ProfileFields.svelte, ported field-for-field.
public static class ProfileFieldSpec
{
    private record FieldDef(string Name, string Label, bool IsBool = false, bool IsTextArea = false);

    private static readonly (string Tab, string Section, FieldDef[] Fields)[] Groups =
    {
        ("Filament", "Basic information", new[]
        {
            new FieldDef("Soluble", "Soluble material", IsBool: true),
            new FieldDef("IsSupport", "Support material", IsBool: true),
            new FieldDef("ImpactStrengthZ", "Impact Strength Z"),
            new FieldDef("RequiredNozzleHrc", "Required nozzle HRC"),
            new FieldDef("DefaultColourHex", "Default color"),
            new FieldDef("DiameterMm", "Diameter (mm)"),
            new FieldDef("DiameterLimitMm", "Diameter limit (mm)"),
            new FieldDef("AdhesivenessCategory", "Adhesiveness Category"),
            new FieldDef("FlowRatio", "Flow ratio"),
            new FieldDef("DensityGCm3", "Density (g/cm³)"),
            new FieldDef("ShrinkPct", "Shrinkage (%)"),
            new FieldDef("VelocityAdaptationFactor", "Velocity Adaptation Factor"),
            new FieldDef("CostPerKg", "Price (money/kg)"),
            new FieldDef("SofteningTempC", "Softening temperature (°C)"),
            new FieldDef("Printable", "Filament printable"),
            new FieldDef("ExtruderVariant", "Extruder variant"),
            new FieldDef("TowerIroningAreaMm2", "Tower ironing area (mm²)"),
            new FieldDef("PrimeVolumeMm3", "Filament prime volume (mm³)"),
            new FieldDef("RammingTravelTimeS", "Travel time after ramming (s)"),
            new FieldDef("RammingTravelTimeNcS", "Travel time after ramming — High Flow (s)"),
            new FieldDef("NozzleTempRangeLowC", "Recommended nozzle temperature — Min (°C)"),
            new FieldDef("NozzleTempRangeHighC", "Recommended nozzle temperature — Max (°C)")
        }),
        ("Filament", "Print temperature", new[]
        {
            new FieldDef("SupertackPlateTempInitialC", "Cool Plate SuperTack — Initial layer (°C)"),
            new FieldDef("SupertackPlateTempC", "Cool Plate SuperTack — Other layers (°C)"),
            new FieldDef("CoolPlateTempInitialC", "Cool Plate — Initial layer (°C)"),
            new FieldDef("CoolPlateTempC", "Cool Plate — Other layers (°C)"),
            new FieldDef("EngPlateTempInitialC", "Engineering Plate — Initial layer (°C)"),
            new FieldDef("EngPlateTempC", "Engineering Plate — Other layers (°C)"),
            new FieldDef("HotPlateTempInitialC", "Smooth PEI / High Temp Plate — Initial layer (°C)"),
            new FieldDef("HotPlateTempC", "Smooth PEI / High Temp Plate — Other layers (°C)"),
            new FieldDef("TexturedPlateTempInitialC", "Textured PEI Plate — Initial layer (°C)"),
            new FieldDef("TexturedPlateTempC", "Textured PEI Plate — Other layers (°C)")
        }),
        ("Filament", "Volumetric speed / scarf seam", new[]
        {
            new FieldDef("AdaptiveVolumetricSpeed", "Adaptive volumetric speed", IsBool: true),
            new FieldDef("MaxVolumetricSpeedMm3S", "Max volumetric speed (mm³/s)"),
            new FieldDef("RammingVolumetricSpeedMm3S", "Ramming volumetric speed — Extruder change (mm³/s)"),
            new FieldDef("RammingVolumetricSpeedNcMm3S", "Ramming volumetric speed — Hotend change (mm³/s)"),
            new FieldDef("ScarfSeamType", "Scarf seam type"),
            new FieldDef("ScarfHeightPct", "Scarf start height"),
            new FieldDef("ScarfGapPct", "Scarf slope gap"),
            new FieldDef("ScarfLengthMm", "Scarf length (mm)")
        }),
        ("Cooling", "Part cooling fan", new[]
        {
            new FieldDef("CloseFanFirstXLayers", "Initial layer fan — For the first N layers"),
            new FieldDef("FirstXLayerFanSpeedPct", "Initial layer fan — Fan speed (%)"),
            new FieldDef("FullFanSpeedLayer", "Linear ramp up to (layers)"),
            new FieldDef("FanMinSpeedPct", "Min fan speed threshold — Fan speed (%)"),
            new FieldDef("FanMaxSpeedPct", "Max fan speed threshold — Fan speed (%)"),
            new FieldDef("FanCoolingLayerTimeS", "Layer time (s)"),
            new FieldDef("SlowDownForLayerCooling", "Slow printing down for better layer cooling", IsBool: true),
            new FieldDef("NoSlowDownForCoolingOnOutwalls", "Don't slow down outer walls", IsBool: true),
            new FieldDef("CoolingSlowdownLogic", "Cooling slowdown logic"),
            new FieldDef("CoolingPerimeterTransitionDistanceMm", "Perimeter transition distance (mm)"),
            new FieldDef("SlowDownMinSpeedMmS", "Min print speed (mm/s)"),
            new FieldDef("SlowDownLayerTimeS", "Slow down layer time (s)"),
            new FieldDef("OverhangFanThreshold", "Cooling overhang threshold"),
            new FieldDef("OverhangThresholdParticipatingCooling", "Overhang threshold for participating cooling"),
            new FieldDef("OverhangFanSpeedPct", "Fan speed for overhangs (%)"),
            new FieldDef("PreStartFanTimeS", "Pre start fan time (s)"),
            new FieldDef("EnableOverhangBridgeFan", "Keep fan always on", IsBool: true)
        }),
        ("Cooling", "Auxiliary / exhaust", new[]
        {
            new FieldDef("AdditionalCoolingFanSpeedPct", "Auxiliary fan speed (%)"),
            new FieldDef("DuringPrintExhaustFanSpeedPct", "During print exhaust fan speed (%)"),
            new FieldDef("CompletePrintExhaustFanSpeedPct", "Complete print exhaust fan speed (%)"),
            new FieldDef("ChamberTemperatureC", "Chamber temperature (°C)"),
            new FieldDef("ActivateAirFiltration", "Activate air filtration", IsBool: true),
            new FieldDef("ReduceFanStopStartFreq", "Reduce fan stop/start frequency", IsBool: true)
        }),
        ("Setting Overrides", "Retraction", new[]
        {
            new FieldDef("RetractionMm", "Length (mm)"),
            new FieldDef("ZHopMm", "Z hop when retract (mm)"),
            new FieldDef("ZHopType", "Z Hop Type"),
            new FieldDef("RetractionSpeedMmS", "Retraction Speed (mm/s)"),
            new FieldDef("DeretractionSpeedMmS", "Deretraction Speed (mm/s)"),
            new FieldDef("ChangeLengthMm", "Length when change hotend (mm)"),
            new FieldDef("RetractRestartExtraMm", "Extra length on restart (mm)"),
            new FieldDef("RetractionMinimumTravelMm", "Travel distance threshold (mm)"),
            new FieldDef("RetractWhenChangingLayer", "Retract when change layer", IsBool: true),
            new FieldDef("WipeEnabled", "Wipe while retracting", IsBool: true),
            new FieldDef("WipeDistanceMm", "Wipe Distance (mm)"),
            new FieldDef("RetractBeforeWipe", "Retract amount before wipe (%)"),
            new FieldDef("LongRetractionsWhenCut", "Long retraction when cut (experimental)", IsBool: true),
            new FieldDef("RetractionDistancesWhenCutMm", "Retraction distance when cut (mm)"),
            new FieldDef("ChangeLengthNcMm", "Length when change hotend — High Flow (mm)"),
            new FieldDef("RetractLengthNcMm", "Extra length on restart — High Flow (mm)"),
            new FieldDef("LongRetractionsWhenEc", "Long retraction when cut — High Flow", IsBool: true),
            new FieldDef("RetractionDistancesWhenEcMm", "Retraction distance when cut — High Flow (mm)")
        }),
        ("Setting Overrides", "Speed", new[]
        {
            new FieldDef("PrintSpeedMmS", "Print speed — manual only (mm/s)"),
            new FieldDef("OverrideProcessOverhangSpeed", "Override overhang speed", IsBool: true),
            new FieldDef("EnableOverhangSpeed", "Slow down for overhangs", IsBool: true),
            new FieldDef("Overhang14SpeedMmS", "10% (mm/s)"),
            new FieldDef("Overhang24SpeedMmS", "25% (mm/s)"),
            new FieldDef("Overhang34SpeedMmS", "50% (mm/s)"),
            new FieldDef("Overhang44SpeedMmS", "75% (mm/s)"),
            new FieldDef("OverhangTotallySpeedMmS", "100% (mm/s)"),
            new FieldDef("BridgeSpeedMmS", "Bridge (mm/s)")
        }),
        ("Advanced", "Advanced", new[]
        {
            new FieldDef("EnablePressureAdvance", "Enable pressure advance", IsBool: true),
            new FieldDef("PressureAdvance", "Pressure advance"),
            new FieldDef("CircleCompensationSpeedMmS", "Circle compensation speed (mm/s)"),
            new FieldDef("HoleCoef1", "Hole coefficient 1"),
            new FieldDef("HoleCoef2", "Hole coefficient 2"),
            new FieldDef("HoleCoef3", "Hole coefficient 3"),
            new FieldDef("HoleLimitMax", "Hole limit max"),
            new FieldDef("HoleLimitMin", "Hole limit min"),
            new FieldDef("CounterCoef1", "Counter coefficient 1"),
            new FieldDef("CounterCoef2", "Counter coefficient 2"),
            new FieldDef("CounterCoef3", "Counter coefficient 3"),
            new FieldDef("CounterLimitMax", "Counter limit max"),
            new FieldDef("CounterLimitMin", "Counter limit min"),
            new FieldDef("DryingAmsLimitations", "AMS drying limitations"),
            new FieldDef("DryingAmsHeatDistortionTempC", "AMS drying heat distortion temp (°C)"),
            new FieldDef("DryingAmsTempC", "AMS drying temp (°C)"),
            new FieldDef("DryingAmsTimeH", "AMS drying time (h)"),
            new FieldDef("DryingChamberBedTempC", "Chamber drying bed temp (°C)"),
            new FieldDef("DryingChamberTimeH", "Chamber drying time (h)"),
            new FieldDef("DryingCoolingTempC", "Drying cooling temp (°C)"),
            new FieldDef("DryingSofteningTempC", "Drying softening temp (°C)"),
            new FieldDef("FlushTempC", "Flush temp (°C)"),
            new FieldDef("FlushVolumetricSpeedMm3S", "Flush volumetric speed (mm³/s)"),
            new FieldDef("VolumetricSpeedCoefficients", "Volumetric speed coefficients"),
            new FieldDef("StartGcode", "Filament start G-code", IsTextArea: true),
            new FieldDef("EndGcode", "Filament end G-code", IsTextArea: true)
        }),
        ("Notes", "Notes", new[] { new FieldDef("SlicerNotes", "Filament notes", IsTextArea: true) }),
        ("Multi Filament", "Multi Filament", new[]
        {
            new FieldDef("TowerInterfacePrintTempC", "Purge temperature (°C)"),
            new FieldDef("TowerInterfacePurgeVolumeMm3", "Purge volumetric speed (mm³/s)"),
            new FieldDef("TowerInterfacePreExtrusionDistMm", "Tower interface pre-extrusion distance (mm)"),
            new FieldDef("TowerInterfacePreExtrusionLengthMm", "Tower interface pre-extrusion length (mm)"),
            new FieldDef("MinimalPurgeOnWipeTowerMm3", "Minimal purge on wipe tower (mm³)"),
            new FieldDef("PrimeVolumeNcMm3", "Filament prime volume — High Flow (mm³)"),
            new FieldDef("CoolingBeforeTowerS", "Cooling before tower (s)")
        })
    };

    public static List<ProfileFieldTab> BuildGroups(IReadOnlyDictionary<string, string>? initialValues)
    {
        return Groups
            .GroupBy(g => g.Tab)
            .Select(tabGroup => new ProfileFieldTab
            {
                Title = tabGroup.Key,
                Sections = tabGroup.Select(g => new ProfileFieldGroup
                {
                    Title = g.Section,
                    Fields = g.Fields.Select(f => new ProfileFieldEntry
                    {
                        Name = f.Name,
                        Label = f.Label,
                        IsBool = f.IsBool,
                        IsTextArea = f.IsTextArea,
                        Value = initialValues?.GetValueOrDefault(f.Name) ?? ""
                    }).ToList()
                }).ToList()
            }).ToList();
    }
}
