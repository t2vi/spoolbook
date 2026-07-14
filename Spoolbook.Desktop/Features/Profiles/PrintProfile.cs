using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Spools;
namespace Spoolbook.Desktop.Features.Profiles;

public enum ProfileSource { Manual, SlicerImport }
public enum SlicerType { PrusaSlicer, OrcaSlicer, BambuStudio }

public class PrintProfile
{
    public int Id { get; set; }
    public int FilamentId { get; set; }
    public Filament? Filament { get; set; }
    public int? SpoolId { get; set; }
    public Spool? Spool { get; set; }
    public required string Name { get; set; }
    public int? PrintSpeedMmS { get; set; }

    // Temps — nozzle + all 5 plates, each with an other-layers and initial-layer value.
    public required int NozzleTempC { get; set; }
    public int? NozzleTempInitialC { get; set; }
    public int? NozzleTempRangeHighC { get; set; }
    public int? NozzleTempRangeLowC { get; set; }
    public int? CoolPlateTempC { get; set; }
    public int? CoolPlateTempInitialC { get; set; }
    public int? HotPlateTempC { get; set; }
    public int? HotPlateTempInitialC { get; set; }
    public int? TexturedPlateTempC { get; set; }
    public int? TexturedPlateTempInitialC { get; set; }
    public int? EngPlateTempC { get; set; }
    public int? EngPlateTempInitialC { get; set; }
    public int? SupertackPlateTempC { get; set; }
    public int? SupertackPlateTempInitialC { get; set; }

    // Cooling
    public int? FanMinSpeedPct { get; set; }
    public int? FanMaxSpeedPct { get; set; }
    public int? AdditionalCoolingFanSpeedPct { get; set; }
    public int? CloseFanFirstXLayers { get; set; }
    public int? CompletePrintExhaustFanSpeedPct { get; set; }
    public int? DuringPrintExhaustFanSpeedPct { get; set; }
    public int? ChamberTemperatureC { get; set; }
    public decimal? CoolingPerimeterTransitionDistanceMm { get; set; }
    public string? CoolingSlowdownLogic { get; set; }
    public bool? EnableOverhangBridgeFan { get; set; }
    public int? FanCoolingLayerTimeS { get; set; }
    public int? FirstXLayerFanSpeedPct { get; set; }
    public int? FullFanSpeedLayer { get; set; }
    public bool? NoSlowDownForCoolingOnOutwalls { get; set; }
    public int? OverhangFanSpeedPct { get; set; }
    public string? OverhangFanThreshold { get; set; }
    public string? OverhangThresholdParticipatingCooling { get; set; }
    public bool? OverrideProcessOverhangSpeed { get; set; }
    public int? PreStartFanTimeS { get; set; }
    public bool? ReduceFanStopStartFreq { get; set; }
    public bool? SlowDownForLayerCooling { get; set; }
    public int? SlowDownLayerTimeS { get; set; }
    public int? SlowDownMinSpeedMmS { get; set; }
    public bool? ActivateAirFiltration { get; set; }

    // Retraction
    public decimal? RetractionMm { get; set; }
    public decimal? RetractionSpeedMmS { get; set; }
    public decimal? DeretractionSpeedMmS { get; set; }
    public decimal? RetractionMinimumTravelMm { get; set; }
    public string? RetractBeforeWipe { get; set; }
    public decimal? RetractRestartExtraMm { get; set; }
    public bool? RetractWhenChangingLayer { get; set; }
    public decimal? RetractionDistancesWhenCutMm { get; set; }
    public decimal? RetractLengthNcMm { get; set; }
    public bool? LongRetractionsWhenCut { get; set; }
    public bool? LongRetractionsWhenEc { get; set; }
    public decimal? RetractionDistancesWhenEcMm { get; set; }

    // Wipe / prime / tower
    public bool? WipeEnabled { get; set; }
    public decimal? WipeDistanceMm { get; set; }
    public decimal? ZHopMm { get; set; }
    public string? ZHopType { get; set; }
    public decimal? ChangeLengthMm { get; set; }
    public decimal? ChangeLengthNcMm { get; set; }
    public int? CoolingBeforeTowerS { get; set; }
    public decimal? MinimalPurgeOnWipeTowerMm3 { get; set; }
    public decimal? PrimeVolumeMm3 { get; set; }
    public decimal? PrimeVolumeNcMm3 { get; set; }
    public decimal? RammingTravelTimeS { get; set; }
    public decimal? RammingTravelTimeNcS { get; set; }
    public decimal? RammingVolumetricSpeedMm3S { get; set; }
    public decimal? RammingVolumetricSpeedNcMm3S { get; set; }
    public decimal? TowerInterfacePreExtrusionDistMm { get; set; }
    public decimal? TowerInterfacePreExtrusionLengthMm { get; set; }
    public int? TowerInterfacePrintTempC { get; set; }
    public decimal? TowerInterfacePurgeVolumeMm3 { get; set; }
    public decimal? TowerIroningAreaMm2 { get; set; }
    public int? FlushTempC { get; set; }
    public decimal? FlushVolumetricSpeedMm3S { get; set; }

    // Speed / overhang
    public bool? AdaptiveVolumetricSpeed { get; set; }
    public decimal? MaxVolumetricSpeedMm3S { get; set; }
    public decimal? BridgeSpeedMmS { get; set; }
    public bool? EnableOverhangSpeed { get; set; }
    public decimal? Overhang14SpeedMmS { get; set; }
    public decimal? Overhang24SpeedMmS { get; set; }
    public decimal? Overhang34SpeedMmS { get; set; }
    public decimal? Overhang44SpeedMmS { get; set; }
    public decimal? OverhangTotallySpeedMmS { get; set; }
    public decimal? CircleCompensationSpeedMmS { get; set; }
    public decimal? VelocityAdaptationFactor { get; set; }
    public string? VolumetricSpeedCoefficients { get; set; }

    // Material properties
    public decimal? DensityGCm3 { get; set; }
    public decimal? DiameterMm { get; set; }
    public decimal? DiameterLimitMm { get; set; }
    public string? ShrinkPct { get; set; }
    public bool? Soluble { get; set; }
    public bool? IsSupport { get; set; }
    public int? Printable { get; set; }
    public int? AdhesivenessCategory { get; set; }
    public decimal? ImpactStrengthZ { get; set; }
    public decimal? CostPerKg { get; set; }
    public decimal? FlowRatio { get; set; }
    public string? ExtruderVariant { get; set; }
    public string? SlicerNotes { get; set; }
    public int? RequiredNozzleHrc { get; set; }

    // Pressure advance
    public bool? EnablePressureAdvance { get; set; }
    public decimal? PressureAdvance { get; set; }

    // Drying
    public string? DryingAmsLimitations { get; set; }
    public int? DryingAmsHeatDistortionTempC { get; set; }
    public int? DryingAmsTempC { get; set; }
    public decimal? DryingAmsTimeH { get; set; }
    public int? DryingChamberBedTempC { get; set; }
    public decimal? DryingChamberTimeH { get; set; }
    public int? DryingCoolingTempC { get; set; }
    public int? DryingSofteningTempC { get; set; }
    public decimal? SofteningTempC { get; set; }

    // Scarf seam
    public string? ScarfSeamType { get; set; }
    public string? ScarfGapPct { get; set; }
    public string? ScarfHeightPct { get; set; }
    public decimal? ScarfLengthMm { get; set; }

    // Z-compensation coefficients
    public decimal? HoleCoef1 { get; set; }
    public decimal? HoleCoef2 { get; set; }
    public decimal? HoleCoef3 { get; set; }
    public decimal? HoleLimitMax { get; set; }
    public decimal? HoleLimitMin { get; set; }
    public decimal? CounterCoef1 { get; set; }
    public decimal? CounterCoef2 { get; set; }
    public decimal? CounterCoef3 { get; set; }
    public decimal? CounterLimitMax { get; set; }
    public decimal? CounterLimitMin { get; set; }

    // G-code snippets
    public string? StartGcode { get; set; }
    public string? EndGcode { get; set; }

    // Color
    public string? DefaultColourHex { get; set; }

    public ProfileSource Source { get; set; } = ProfileSource.Manual;
    public SlicerType? SourceSlicer { get; set; }
    public string? RawSettingsJson { get; set; }
    public string? SourcePresetPath { get; set; }
    public int VersionNumber { get; set; } = 1;
    public string? VersionName { get; set; }
    public bool IsCurrentVersion { get; set; } = true;
    public string? Notes { get; set; }
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
}
