using System.Text.Json;
using System.Text.Json.Nodes;
namespace Spoolbook.Desktop.Features.BambuImport;

public class ImportedPreset
{
    public required string Name { get; init; }
    public required string FilePath { get; init; }

    public override string ToString() => Name;
}

public class ImportResult
{
    public bool Ok { get; init; }
    public string? Error { get; init; }
    public string? SuggestedName { get; init; }
    public Dictionary<string, string>? Fields { get; init; }
    public string? RawSettingsJson { get; init; }
}

public class PushResult
{
    public bool Ok { get; init; }
    public string? Error { get; init; }
}

public class BambuFilamentImportService
{
    // Maps Bambu's raw JSON key -> our PrintProfile property name.
    private static readonly Dictionary<string, string> KeyMap = new()
    {
        ["nozzle_temperature"] = "NozzleTempC",
        ["nozzle_temperature_initial_layer"] = "NozzleTempInitialC",
        ["nozzle_temperature_range_high"] = "NozzleTempRangeHighC",
        ["nozzle_temperature_range_low"] = "NozzleTempRangeLowC",
        ["cool_plate_temp"] = "CoolPlateTempC",
        ["cool_plate_temp_initial_layer"] = "CoolPlateTempInitialC",
        ["hot_plate_temp"] = "HotPlateTempC",
        ["hot_plate_temp_initial_layer"] = "HotPlateTempInitialC",
        ["textured_plate_temp"] = "TexturedPlateTempC",
        ["textured_plate_temp_initial_layer"] = "TexturedPlateTempInitialC",
        ["eng_plate_temp"] = "EngPlateTempC",
        ["eng_plate_temp_initial_layer"] = "EngPlateTempInitialC",
        ["supertack_plate_temp"] = "SupertackPlateTempC",
        ["supertack_plate_temp_initial_layer"] = "SupertackPlateTempInitialC",
        ["fan_min_speed"] = "FanMinSpeedPct",
        ["fan_max_speed"] = "FanMaxSpeedPct",
        ["additional_cooling_fan_speed"] = "AdditionalCoolingFanSpeedPct",
        ["close_fan_the_first_x_layers"] = "CloseFanFirstXLayers",
        ["complete_print_exhaust_fan_speed"] = "CompletePrintExhaustFanSpeedPct",
        ["during_print_exhaust_fan_speed"] = "DuringPrintExhaustFanSpeedPct",
        ["chamber_temperatures"] = "ChamberTemperatureC",
        ["cooling_perimeter_transition_distance"] = "CoolingPerimeterTransitionDistanceMm",
        ["cooling_slowdown_logic"] = "CoolingSlowdownLogic",
        ["enable_overhang_bridge_fan"] = "EnableOverhangBridgeFan",
        ["fan_cooling_layer_time"] = "FanCoolingLayerTimeS",
        ["first_x_layer_fan_speed"] = "FirstXLayerFanSpeedPct",
        ["full_fan_speed_layer"] = "FullFanSpeedLayer",
        ["no_slow_down_for_cooling_on_outwalls"] = "NoSlowDownForCoolingOnOutwalls",
        ["overhang_fan_speed"] = "OverhangFanSpeedPct",
        ["overhang_fan_threshold"] = "OverhangFanThreshold",
        ["overhang_threshold_participating_cooling"] = "OverhangThresholdParticipatingCooling",
        ["override_process_overhang_speed"] = "OverrideProcessOverhangSpeed",
        ["pre_start_fan_time"] = "PreStartFanTimeS",
        ["reduce_fan_stop_start_freq"] = "ReduceFanStopStartFreq",
        ["slow_down_for_layer_cooling"] = "SlowDownForLayerCooling",
        ["slow_down_layer_time"] = "SlowDownLayerTimeS",
        ["slow_down_min_speed"] = "SlowDownMinSpeedMmS",
        ["activate_air_filtration"] = "ActivateAirFiltration",
        ["filament_retraction_length"] = "RetractionMm",
        ["filament_retraction_speed"] = "RetractionSpeedMmS",
        ["filament_deretraction_speed"] = "DeretractionSpeedMmS",
        ["filament_retraction_minimum_travel"] = "RetractionMinimumTravelMm",
        ["filament_retract_before_wipe"] = "RetractBeforeWipe",
        ["filament_retract_restart_extra"] = "RetractRestartExtraMm",
        ["filament_retract_when_changing_layer"] = "RetractWhenChangingLayer",
        ["filament_retraction_distances_when_cut"] = "RetractionDistancesWhenCutMm",
        ["filament_retract_length_nc"] = "RetractLengthNcMm",
        ["filament_long_retractions_when_cut"] = "LongRetractionsWhenCut",
        ["long_retractions_when_ec"] = "LongRetractionsWhenEc",
        ["retraction_distances_when_ec"] = "RetractionDistancesWhenEcMm",
        ["filament_wipe"] = "WipeEnabled",
        ["filament_wipe_distance"] = "WipeDistanceMm",
        ["filament_z_hop"] = "ZHopMm",
        ["filament_z_hop_types"] = "ZHopType",
        ["filament_change_length"] = "ChangeLengthMm",
        ["filament_change_length_nc"] = "ChangeLengthNcMm",
        ["filament_cooling_before_tower"] = "CoolingBeforeTowerS",
        ["filament_minimal_purge_on_wipe_tower"] = "MinimalPurgeOnWipeTowerMm3",
        ["filament_prime_volume"] = "PrimeVolumeMm3",
        ["filament_prime_volume_nc"] = "PrimeVolumeNcMm3",
        ["filament_ramming_travel_time"] = "RammingTravelTimeS",
        ["filament_ramming_travel_time_nc"] = "RammingTravelTimeNcS",
        ["filament_ramming_volumetric_speed"] = "RammingVolumetricSpeedMm3S",
        ["filament_ramming_volumetric_speed_nc"] = "RammingVolumetricSpeedNcMm3S",
        ["filament_tower_interface_pre_extrusion_dist"] = "TowerInterfacePreExtrusionDistMm",
        ["filament_tower_interface_pre_extrusion_length"] = "TowerInterfacePreExtrusionLengthMm",
        ["filament_tower_interface_print_temp"] = "TowerInterfacePrintTempC",
        ["filament_tower_interface_purge_volume"] = "TowerInterfacePurgeVolumeMm3",
        ["filament_tower_ironing_area"] = "TowerIroningAreaMm2",
        ["filament_flush_temp"] = "FlushTempC",
        ["filament_flush_volumetric_speed"] = "FlushVolumetricSpeedMm3S",
        ["filament_adaptive_volumetric_speed"] = "AdaptiveVolumetricSpeed",
        ["filament_max_volumetric_speed"] = "MaxVolumetricSpeedMm3S",
        ["filament_bridge_speed"] = "BridgeSpeedMmS",
        ["filament_enable_overhang_speed"] = "EnableOverhangSpeed",
        ["filament_overhang_1_4_speed"] = "Overhang14SpeedMmS",
        ["filament_overhang_2_4_speed"] = "Overhang24SpeedMmS",
        ["filament_overhang_3_4_speed"] = "Overhang34SpeedMmS",
        ["filament_overhang_4_4_speed"] = "Overhang44SpeedMmS",
        ["filament_overhang_totally_speed"] = "OverhangTotallySpeedMmS",
        ["circle_compensation_speed"] = "CircleCompensationSpeedMmS",
        ["filament_velocity_adaptation_factor"] = "VelocityAdaptationFactor",
        ["volumetric_speed_coefficients"] = "VolumetricSpeedCoefficients",
        ["filament_density"] = "DensityGCm3",
        ["filament_diameter"] = "DiameterMm",
        ["diameter_limit"] = "DiameterLimitMm",
        ["filament_shrink"] = "ShrinkPct",
        ["filament_soluble"] = "Soluble",
        ["filament_is_support"] = "IsSupport",
        ["filament_printable"] = "Printable",
        ["filament_adhesiveness_category"] = "AdhesivenessCategory",
        ["impact_strength_z"] = "ImpactStrengthZ",
        ["filament_cost"] = "CostPerKg",
        ["filament_flow_ratio"] = "FlowRatio",
        ["filament_extruder_variant"] = "ExtruderVariant",
        ["filament_notes"] = "SlicerNotes",
        ["required_nozzle_HRC"] = "RequiredNozzleHrc",
        ["enable_pressure_advance"] = "EnablePressureAdvance",
        ["pressure_advance"] = "PressureAdvance",
        ["filament_dev_ams_drying_ams_limitations"] = "DryingAmsLimitations",
        ["filament_dev_ams_drying_heat_distortion_temperature"] = "DryingAmsHeatDistortionTempC",
        ["filament_dev_ams_drying_temperature"] = "DryingAmsTempC",
        ["filament_dev_ams_drying_time"] = "DryingAmsTimeH",
        ["filament_dev_chamber_drying_bed_temperature"] = "DryingChamberBedTempC",
        ["filament_dev_chamber_drying_time"] = "DryingChamberTimeH",
        ["filament_dev_drying_cooling_temperature"] = "DryingCoolingTempC",
        ["filament_dev_drying_softening_temperature"] = "DryingSofteningTempC",
        ["temperature_vitrification"] = "SofteningTempC",
        ["filament_scarf_seam_type"] = "ScarfSeamType",
        ["filament_scarf_gap"] = "ScarfGapPct",
        ["filament_scarf_height"] = "ScarfHeightPct",
        ["filament_scarf_length"] = "ScarfLengthMm",
        ["hole_coef_1"] = "HoleCoef1",
        ["hole_coef_2"] = "HoleCoef2",
        ["hole_coef_3"] = "HoleCoef3",
        ["hole_limit_max"] = "HoleLimitMax",
        ["hole_limit_min"] = "HoleLimitMin",
        ["counter_coef_1"] = "CounterCoef1",
        ["counter_coef_2"] = "CounterCoef2",
        ["counter_coef_3"] = "CounterCoef3",
        ["counter_limit_max"] = "CounterLimitMax",
        ["counter_limit_min"] = "CounterLimitMin",
        ["filament_start_gcode"] = "StartGcode",
        ["filament_end_gcode"] = "EndGcode",
        ["default_filament_colour"] = "DefaultColourHex"
    };

    private static readonly HashSet<string> BoolFields = new()
    {
        "EnableOverhangBridgeFan", "NoSlowDownForCoolingOnOutwalls", "OverrideProcessOverhangSpeed",
        "ReduceFanStopStartFreq", "SlowDownForLayerCooling", "ActivateAirFiltration",
        "RetractWhenChangingLayer", "LongRetractionsWhenCut", "LongRetractionsWhenEc", "WipeEnabled",
        "AdaptiveVolumetricSpeed", "EnableOverhangSpeed", "Soluble", "IsSupport", "EnablePressureAdvance"
    };

    // Reverse of KeyMap — our PrintProfile property name -> Bambu's raw JSON key.
    private static readonly Dictionary<string, string> ReverseKeyMap =
        KeyMap.ToDictionary(kv => kv.Value, kv => kv.Key);

    private BambuPresetResolver _resolver;

    public BambuFilamentImportService(BambuPresetResolver resolver)
    {
        _resolver = resolver;
    }

    public void UpdatePaths(string userPresetsDir, string systemProfilesDir) =>
        _resolver = new BambuPresetResolver(userPresetsDir, systemProfilesDir);

    public List<ImportedPreset> ListSystemPresets() => _resolver.ListSystemFilamentPresets();

    public List<ImportedPreset> ListUserPresets() => _resolver.ListUserFilamentPresets();

    public async Task<ImportResult> ImportAsync(string filePath)
    {
        string leafJson;
        try
        {
            leafJson = await File.ReadAllTextAsync(filePath);
        }
        catch (IOException ex)
        {
            return new ImportResult { Ok = false, Error = $"Couldn't read file: {ex.Message}" };
        }

        JsonDocument leafDoc;
        try
        {
            leafDoc = JsonDocument.Parse(leafJson);
        }
        catch (JsonException)
        {
            return new ImportResult { Ok = false, Error = "invalid_json" };
        }

        if (!leafDoc.RootElement.TryGetProperty("filament_settings_id", out _))
            return new ImportResult { Ok = false, Error = "not_filament_preset" };

        var suggestedName = leafDoc.RootElement.TryGetProperty("name", out var n)
            ? n.GetString()
            : Path.GetFileNameWithoutExtension(filePath);

        var merged = await _resolver.ResolveAsync(leafJson);

        var fields = new Dictionary<string, string>();
        foreach (var (bambuKey, ourField) in KeyMap)
        {
            if (!merged.TryGetValue(bambuKey, out var element)) continue;
            var raw = FirstValue(element);
            if (raw is null) continue;

            fields[ourField] = BoolFields.Contains(ourField) ? ToBoolString(raw) : raw;
        }

        var rawSettingsJson = JsonSerializer.Serialize(
            merged.ToDictionary(kv => kv.Key, kv => kv.Value.Clone()));

        return new ImportResult
        {
            Ok = true,
            SuggestedName = suggestedName,
            Fields = fields,
            RawSettingsJson = rawSettingsJson
        };
    }

    // Writes only the keys spoolbook manages back into the linked leaf file, leaving
    // inherits/instantiation/compatible_printers/unmanaged keys untouched. Blank values
    // and unmapped field names are skipped entirely rather than clobbering the file.
    public async Task<PushResult> PushToFileAsync(string filePath, IReadOnlyDictionary<string, string> fields)
    {
        string json;
        try
        {
            json = await File.ReadAllTextAsync(filePath);
        }
        catch (IOException ex)
        {
            return new PushResult { Ok = false, Error = $"Couldn't read file: {ex.Message}" };
        }

        JsonNode? root;
        try
        {
            root = JsonNode.Parse(json);
        }
        catch (JsonException)
        {
            return new PushResult { Ok = false, Error = "invalid_json" };
        }

        if (root is not JsonObject obj)
            return new PushResult { Ok = false, Error = "invalid_json" };

        foreach (var (ourField, value) in fields)
        {
            if (string.IsNullOrWhiteSpace(value)) continue;
            if (!ReverseKeyMap.TryGetValue(ourField, out var bambuKey)) continue;

            var raw = BoolFields.Contains(ourField) ? (value == "true" ? "1" : "0") : value;

            if (obj[bambuKey] is JsonArray { Count: > 0 } existingArray)
                existingArray[0] = JsonValue.Create(raw);
            else
                obj[bambuKey] = new JsonArray(JsonValue.Create(raw));
        }

        try
        {
            await File.WriteAllTextAsync(filePath, obj.ToJsonString(new JsonSerializerOptions { WriteIndented = true }));
        }
        catch (IOException ex)
        {
            return new PushResult { Ok = false, Error = $"Couldn't write file: {ex.Message}" };
        }

        return new PushResult { Ok = true };
    }

    private static string? FirstValue(JsonElement element)
    {
        var value = element.ValueKind == JsonValueKind.Array
            ? (element.GetArrayLength() > 0 ? element[0] : default)
            : element;

        if (value.ValueKind != JsonValueKind.String) return null;
        var s = value.GetString();
        return string.IsNullOrEmpty(s) || s == "nil" ? null : s;
    }

    private static string ToBoolString(string raw) => raw switch
    {
        "1" => "true",
        "0" => "false",
        "true" or "false" => raw,
        _ => "false"
    };
}
