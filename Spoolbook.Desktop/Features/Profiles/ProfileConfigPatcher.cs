using System.Globalization;
using System.Text.Json.Nodes;
namespace Spoolbook.Desktop.Features.Profiles;

// Patches a real Project's Metadata/project_settings.config (verbatim OrcaSlicer/Bambu Studio
// config JSON) with a PrintProfile's settings fields, for the re-slice-before-print feature.
// Patches, never regenerates — every key not in Map (machine/printer settings, start/end
// gcode, bed shape, nozzle diameter, everything PrintProfile doesn't track) survives untouched.
// Array-valued keys (per-filament) are patched at index 0 only — spoolbook only ever targets a
// single AMS slot per print (see PrinterPrintService's ams_mapping handling). Values are written
// as strings, matching the config's own convention (even numeric/boolean settings are quoted
// strings, e.g. "layer_height": "0.2", booleans as "0"/"1" not "true"/"false").
//
// Mapping verified against a real exported project_settings.config, not guessed — every key
// below was confirmed present in a real sample (see ProfileConfigPatcherTests' fixture). Four
// PrintProfile fields have no confirmed key and are intentionally left unmapped:
//   - PrintSpeedMmS: no single "overall speed" key exists in the resolved config — Orca/Bambu
//     splits print speed into ~15 separate per-feature settings (outer_wall_speed, etc.).
//   - LongRetractionsWhenEc: no "long_retractions_when_ec" key found in a real sample, despite
//     "retraction_distances_when_ec" existing as its sibling — may not exist, or simply wasn't
//     present in this particular profile's resolved key set.
//   - SlicerNotes: ambiguous between filament_notes/process_notes/printer_notes in the real
//     config — no way to tell which one PrintProfile.SlicerNotes means without the original
//     .3mf-import mapping code, which doesn't exist as a reusable artifact (checked).
//   - SofteningTempC: possibly temperature_vitrification (a material glass-transition proxy),
//     but unconfirmed — distinct from DryingSofteningTempC, which does map confidently.
public static class ProfileConfigPatcher
{
    // PropertyName -> (config JSON key, is this key array-valued in the real config).
    private static readonly Dictionary<string, (string Key, bool IsArray)> Map = new()
    {
        ["ActivateAirFiltration"] = ("activate_air_filtration", true),  // bool?
        ["AdaptiveVolumetricSpeed"] = ("filament_adaptive_volumetric_speed", true),  // bool?
        ["AdditionalCoolingFanSpeedPct"] = ("additional_cooling_fan_speed", true),  // int?
        ["AdhesivenessCategory"] = ("filament_adhesiveness_category", true),  // int?
        ["BridgeSpeedMmS"] = ("bridge_speed", true),  // decimal?
        ["ChamberTemperatureC"] = ("chamber_temperatures", true),  // int?
        ["ChangeLengthMm"] = ("filament_change_length", true),  // decimal?
        ["ChangeLengthNcMm"] = ("filament_change_length_nc", true),  // decimal?
        ["CircleCompensationSpeedMmS"] = ("circle_compensation_speed", true),  // decimal?
        ["CloseFanFirstXLayers"] = ("close_fan_the_first_x_layers", true),  // int?
        ["CompletePrintExhaustFanSpeedPct"] = ("complete_print_exhaust_fan_speed", true),  // int?
        ["CoolPlateTempC"] = ("cool_plate_temp", true),  // int?
        ["CoolPlateTempInitialC"] = ("cool_plate_temp_initial_layer", true),  // int?
        ["CoolingBeforeTowerS"] = ("filament_cooling_before_tower", true),  // int?
        ["CoolingPerimeterTransitionDistanceMm"] = ("cooling_perimeter_transition_distance", true),  // decimal?
        ["CoolingSlowdownLogic"] = ("cooling_slowdown_logic", true),  // string?
        ["CostPerKg"] = ("filament_cost", true),  // decimal?
        ["CounterCoef1"] = ("counter_coef_1", true),  // decimal?
        ["CounterCoef2"] = ("counter_coef_2", true),  // decimal?
        ["CounterCoef3"] = ("counter_coef_3", true),  // decimal?
        ["CounterLimitMax"] = ("counter_limit_max", true),  // decimal?
        ["CounterLimitMin"] = ("counter_limit_min", true),  // decimal?
        ["DefaultColourHex"] = ("default_filament_colour", true),  // string?
        ["DensityGCm3"] = ("filament_density", true),  // decimal?
        ["DeretractionSpeedMmS"] = ("deretraction_speed", true),  // decimal?
        ["DiameterLimitMm"] = ("diameter_limit", true),  // decimal?
        ["DiameterMm"] = ("filament_diameter", true),  // decimal?
        ["DryingAmsHeatDistortionTempC"] = ("filament_dev_ams_drying_heat_distortion_temperature", true),  // int?
        ["DryingAmsLimitations"] = ("filament_dev_ams_drying_ams_limitations", true),  // string?
        ["DryingAmsTempC"] = ("filament_dev_ams_drying_temperature", true),  // int?
        ["DryingAmsTimeH"] = ("filament_dev_ams_drying_time", true),  // decimal?
        ["DryingChamberBedTempC"] = ("filament_dev_chamber_drying_bed_temperature", true),  // int?
        ["DryingChamberTimeH"] = ("filament_dev_chamber_drying_time", true),  // decimal?
        ["DryingCoolingTempC"] = ("filament_dev_drying_cooling_temperature", true),  // int?
        ["DryingSofteningTempC"] = ("filament_dev_drying_softening_temperature", true),  // int?
        ["DuringPrintExhaustFanSpeedPct"] = ("during_print_exhaust_fan_speed", true),  // int?
        ["EnableOverhangBridgeFan"] = ("enable_overhang_bridge_fan", true),  // bool?
        ["EnableOverhangSpeed"] = ("enable_overhang_speed", true),  // bool?
        ["EnablePressureAdvance"] = ("enable_pressure_advance", true),  // bool?
        ["EndGcode"] = ("filament_end_gcode", true),  // string?
        ["EngPlateTempC"] = ("eng_plate_temp", true),  // int?
        ["EngPlateTempInitialC"] = ("eng_plate_temp_initial_layer", true),  // int?
        ["ExtruderVariant"] = ("filament_extruder_variant", true),  // string?
        ["FanCoolingLayerTimeS"] = ("fan_cooling_layer_time", true),  // int?
        ["FanMaxSpeedPct"] = ("fan_max_speed", true),  // int?
        ["FanMinSpeedPct"] = ("fan_min_speed", true),  // int?
        ["FirstXLayerFanSpeedPct"] = ("first_x_layer_fan_speed", true),  // int?
        ["FlowRatio"] = ("filament_flow_ratio", true),  // decimal?
        ["FlushTempC"] = ("filament_flush_temp", true),  // int?
        ["FlushVolumetricSpeedMm3S"] = ("filament_flush_volumetric_speed", true),  // decimal?
        ["FullFanSpeedLayer"] = ("full_fan_speed_layer", true),  // int?
        ["HoleCoef1"] = ("hole_coef_1", true),  // decimal?
        ["HoleCoef2"] = ("hole_coef_2", true),  // decimal?
        ["HoleCoef3"] = ("hole_coef_3", true),  // decimal?
        ["HoleLimitMax"] = ("hole_limit_max", true),  // decimal?
        ["HoleLimitMin"] = ("hole_limit_min", true),  // decimal?
        ["HotPlateTempC"] = ("hot_plate_temp", true),  // int?
        ["HotPlateTempInitialC"] = ("hot_plate_temp_initial_layer", true),  // int?
        ["ImpactStrengthZ"] = ("impact_strength_z", true),  // decimal?
        ["IsSupport"] = ("filament_is_support", true),  // bool?
        ["LongRetractionsWhenCut"] = ("long_retractions_when_cut", true),  // bool?
        ["MaxVolumetricSpeedMm3S"] = ("filament_max_volumetric_speed", true),  // decimal?
        ["MinimalPurgeOnWipeTowerMm3"] = ("filament_minimal_purge_on_wipe_tower", true),  // decimal?
        ["NoSlowDownForCoolingOnOutwalls"] = ("no_slow_down_for_cooling_on_outwalls", true),  // bool?
        ["NozzleTempC"] = ("nozzle_temperature", true),  // int
        ["NozzleTempInitialC"] = ("nozzle_temperature_initial_layer", true),  // int?
        ["NozzleTempRangeHighC"] = ("nozzle_temperature_range_high", true),  // int?
        ["NozzleTempRangeLowC"] = ("nozzle_temperature_range_low", true),  // int?
        ["Overhang14SpeedMmS"] = ("overhang_1_4_speed", true),  // decimal?
        ["Overhang24SpeedMmS"] = ("overhang_2_4_speed", true),  // decimal?
        ["Overhang34SpeedMmS"] = ("overhang_3_4_speed", true),  // decimal?
        ["Overhang44SpeedMmS"] = ("overhang_4_4_speed", true),  // decimal?
        ["OverhangFanSpeedPct"] = ("overhang_fan_speed", true),  // int?
        ["OverhangFanThreshold"] = ("overhang_fan_threshold", true),  // string?
        ["OverhangThresholdParticipatingCooling"] = ("overhang_threshold_participating_cooling", true),  // string?
        ["OverhangTotallySpeedMmS"] = ("overhang_totally_speed", true),  // decimal?
        ["OverrideProcessOverhangSpeed"] = ("override_process_overhang_speed", true),  // bool?
        ["PreStartFanTimeS"] = ("pre_start_fan_time", true),  // int?
        ["PressureAdvance"] = ("pressure_advance", true),  // decimal?
        ["PrimeVolumeMm3"] = ("filament_prime_volume", true),  // decimal?
        ["PrimeVolumeNcMm3"] = ("filament_prime_volume_nc", true),  // decimal?
        ["Printable"] = ("filament_printable", true),  // int?
        ["RammingTravelTimeNcS"] = ("filament_ramming_travel_time_nc", true),  // decimal?
        ["RammingTravelTimeS"] = ("filament_ramming_travel_time", true),  // decimal?
        ["RammingVolumetricSpeedMm3S"] = ("filament_ramming_volumetric_speed", true),  // decimal?
        ["RammingVolumetricSpeedNcMm3S"] = ("filament_ramming_volumetric_speed_nc", true),  // decimal?
        ["ReduceFanStopStartFreq"] = ("reduce_fan_stop_start_freq", true),  // bool?
        ["RequiredNozzleHrc"] = ("required_nozzle_HRC", true),  // int?
        ["RetractBeforeWipe"] = ("retract_before_wipe", true),  // string?
        ["RetractLengthNcMm"] = ("retract_length_toolchange", true),  // decimal?
        ["RetractRestartExtraMm"] = ("retract_restart_extra", true),  // decimal?
        ["RetractWhenChangingLayer"] = ("retract_when_changing_layer", true),  // bool?
        ["RetractionDistancesWhenCutMm"] = ("retraction_distances_when_cut", true),  // decimal?
        ["RetractionDistancesWhenEcMm"] = ("retraction_distances_when_ec", true),  // decimal?
        ["RetractionMinimumTravelMm"] = ("retraction_minimum_travel", true),  // decimal?
        ["RetractionMm"] = ("retraction_length", true),  // decimal?
        ["RetractionSpeedMmS"] = ("retraction_speed", true),  // decimal?
        ["ScarfGapPct"] = ("filament_scarf_gap", true),  // string?
        ["ScarfHeightPct"] = ("filament_scarf_height", true),  // string?
        ["ScarfLengthMm"] = ("filament_scarf_length", true),  // decimal?
        ["ScarfSeamType"] = ("filament_scarf_seam_type", true),  // string?
        ["ShrinkPct"] = ("filament_shrink", true),  // string?
        ["SlowDownForLayerCooling"] = ("slow_down_for_layer_cooling", true),  // bool?
        ["SlowDownLayerTimeS"] = ("slow_down_layer_time", true),  // int?
        ["SlowDownMinSpeedMmS"] = ("slow_down_min_speed", true),  // int?
        ["Soluble"] = ("filament_soluble", true),  // bool?
        ["StartGcode"] = ("filament_start_gcode", true),  // string?
        ["SupertackPlateTempC"] = ("supertack_plate_temp", true),  // int?
        ["SupertackPlateTempInitialC"] = ("supertack_plate_temp_initial_layer", true),  // int?
        ["TexturedPlateTempC"] = ("textured_plate_temp", true),  // int?
        ["TexturedPlateTempInitialC"] = ("textured_plate_temp_initial_layer", true),  // int?
        ["TowerInterfacePreExtrusionDistMm"] = ("filament_tower_interface_pre_extrusion_dist", true),  // decimal?
        ["TowerInterfacePreExtrusionLengthMm"] = ("filament_tower_interface_pre_extrusion_length", true),  // decimal?
        ["TowerInterfacePrintTempC"] = ("filament_tower_interface_print_temp", true),  // int?
        ["TowerInterfacePurgeVolumeMm3"] = ("filament_tower_interface_purge_volume", true),  // decimal?
        ["TowerIroningAreaMm2"] = ("filament_tower_ironing_area", true),  // decimal?
        ["VelocityAdaptationFactor"] = ("filament_velocity_adaptation_factor", true),  // decimal?
        ["VolumetricSpeedCoefficients"] = ("volumetric_speed_coefficients", true),  // string?
        ["WipeDistanceMm"] = ("wipe_distance", true),  // decimal?
        ["WipeEnabled"] = ("wipe", true),  // bool?
        ["ZHopMm"] = ("z_hop", true),  // decimal?
        ["ZHopType"] = ("z_hop_types", true),  // string?
    };

    public static string Patch(string originalConfigJson, PrintProfile profile)
    {
        var root = JsonNode.Parse(originalConfigJson)!.AsObject();

        foreach (var (propName, (key, isArray)) in Map)
        {
            var prop = typeof(PrintProfile).GetProperty(propName)!;
            var stringValue = ToConfigString(prop.GetValue(profile));
            if (stringValue is null) continue; // null = leave the original value untouched

            if (isArray)
            {
                if (root[key] is JsonArray { Count: > 0 } arr)
                    arr[0] = JsonValue.Create(stringValue);
                else
                    root[key] = new JsonArray(JsonValue.Create(stringValue));
            }
            else
            {
                root[key] = JsonValue.Create(stringValue);
            }
        }

        return root.ToJsonString();
    }

    private static string? ToConfigString(object? value) => value switch
    {
        null => null,
        bool b => b ? "1" : "0",
        IFormattable f => f.ToString(null, CultureInfo.InvariantCulture),
        _ => value.ToString()
    };
}

