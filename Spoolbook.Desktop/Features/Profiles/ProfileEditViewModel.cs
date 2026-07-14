using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.BambuImport;
using Spoolbook.Desktop.Features.Settings.Filaments;
namespace Spoolbook.Desktop.Features.Profiles;

public partial class ProfileEditViewModel : ViewModelBase
{
    private readonly PrintProfileService _profileService;
    private readonly BambuFilamentImportService _importService;
    private readonly FilamentService _filamentService;
    private readonly int? _id;

    [ObservableProperty]
    private ObservableCollection<Filament> filamentOptions = new();

    [ObservableProperty]
    private Filament? selectedFilament;

    [ObservableProperty]
    private string name = "";

    [ObservableProperty]
    private string nozzleTempC = "";

    [ObservableProperty]
    private string? nozzleTempInitialC;

    [ObservableProperty]
    private string? notes;

    [ObservableProperty]
    private string? errorMessage;

    [ObservableProperty]
    private string? linkHint;

    [ObservableProperty]
    private string? linkedFileLabel;

    [ObservableProperty]
    private ImportedPreset? selectedUserPreset;

    [ObservableProperty]
    private int versionNumber = 1;

    [ObservableProperty]
    private bool isDirty;

    [ObservableProperty]
    private ObservableCollection<PrintProfile> versions = new();

    [ObservableProperty]
    private bool pendingNewVersion;

    private bool _loaded;

    public List<ImportedPreset> UserPresets { get; }

    public List<ProfileFieldTab> FieldTabs { get; }

    public bool IsEdit { get; }
    public string PageTitle => IsEdit ? "Edit profile" : "Add profile";
    public bool CanCheckForUpdates => IsEdit && _sourcePresetPath is not null;
    public bool LinkSectionExpanded => _sourcePresetPath is null;
    public bool HasMultipleVersions => Versions.Count > 1;
    public Action? Close { get; set; }
    public Action<ProfileEditViewModel>? SwitchTo { get; set; }

    partial void OnVersionsChanged(ObservableCollection<PrintProfile> value) => OnPropertyChanged(nameof(HasMultipleVersions));

    private ProfileSource? _source;
    private SlicerType? _sourceSlicer;
    private string? _rawSettingsJson;
    private string? _sourcePresetPath;

    public ProfileEditViewModel(
        PrintProfileService profileService,
        BambuFilamentImportService importService,
        FilamentService filamentService,
        PrintProfile? existing)
    {
        _profileService = profileService;
        _importService = importService;
        _filamentService = filamentService;
        UserPresets = importService.ListUserPresets();
        _ = LoadFilamentOptionsAsync();

        if (existing is not null)
        {
            _id = existing.Id;
            IsEdit = true;
            SelectedFilament = existing.Filament;
            Name = existing.Name;
            NozzleTempC = existing.NozzleTempC.ToString();
            NozzleTempInitialC = existing.NozzleTempInitialC?.ToString();
            Notes = existing.Notes;
            _source = existing.Source;
            _sourceSlicer = existing.SourceSlicer;
            _rawSettingsJson = existing.RawSettingsJson;
            _sourcePresetPath = existing.SourcePresetPath;
            VersionNumber = existing.VersionNumber;
            LinkedFileLabel = _sourcePresetPath is not null
                ? $"Linked to: {Path.GetFileName(_sourcePresetPath)} — fully resolved via inherits chain"
                : null;
            FieldTabs = ProfileFieldSpec.BuildGroups(ProfileFieldMapper.ToFieldStrings(existing));
        }
        else
        {
            FieldTabs = ProfileFieldSpec.BuildGroups(null);
        }

        foreach (var entry in FieldTabs.SelectMany(t => t.Sections).SelectMany(g => g.Fields))
            entry.PropertyChanged += (_, _) => IsDirty = true;
        _loaded = true;

        _ = LoadVersionsAsync();
    }

    private async Task LoadFilamentOptionsAsync()
    {
        FilamentOptions = new ObservableCollection<Filament>(await _filamentService.ListAsync());
    }

    private async Task LoadVersionsAsync()
    {
        if (_sourcePresetPath is null || SelectedFilament is null) return;
        var list = await _profileService.ListVersionsAsync(SelectedFilament.Id, _sourcePresetPath);
        Versions = new ObservableCollection<PrintProfile>(list);
    }

    private void MarkDirty()
    {
        if (_loaded) IsDirty = true;
    }

    partial void OnNameChanged(string value) => MarkDirty();
    partial void OnNozzleTempCChanged(string value) => MarkDirty();
    partial void OnNozzleTempInitialCChanged(string? value) => MarkDirty();
    partial void OnNotesChanged(string? value) => MarkDirty();

    private Dictionary<string, string> CollectFields() =>
        FieldTabs.SelectMany(t => t.Sections).SelectMany(g => g.Fields).ToDictionary(f => f.Name, f => f.Value);

    private ProfileInput BuildInput() => new()
    {
        Name = Name,
        NozzleTempC = NozzleTempC,
        NozzleTempInitialC = NozzleTempInitialC,
        Notes = string.IsNullOrWhiteSpace(Notes) ? null : Notes,
        Source = _source,
        SourceSlicer = _sourceSlicer,
        RawSettingsJson = _rawSettingsJson,
        SourcePresetPath = _sourcePresetPath,
        VersionNumber = VersionNumber,
        Fields = CollectFields()
    };

    private async Task PushCurrentFieldsToFileAsync()
    {
        if (_sourcePresetPath is null) return;

        var fields = CollectFields();
        fields["NozzleTempC"] = NozzleTempC;
        if (NozzleTempInitialC is not null) fields["NozzleTempInitialC"] = NozzleTempInitialC;

        var result = await _importService.PushToFileAsync(_sourcePresetPath, fields);
        if (!result.Ok) ErrorMessage = $"Saved, but couldn't update the linked file: {result.Error}";
    }

    public async Task<ProfileResult> SaveInPlaceAsync()
    {
        if (SelectedFilament is null)
        {
            ErrorMessage = "Pick a filament.";
            return new ProfileResult { Ok = false, Errors = new Dictionary<string, string> { ["Filament"] = ErrorMessage } };
        }

        var input = BuildInput();
        var result = _id.HasValue
            ? await _profileService.UpdateProfileAsync(_id.Value, input)
            : await _profileService.CreateProfileAsync(SelectedFilament.Id, input);

        if (!result.Ok)
        {
            ErrorMessage = string.Join(", ", result.Errors!.Values);
            return result;
        }

        await PushCurrentFieldsToFileAsync();
        IsDirty = false;
        PendingNewVersion = false;
        Close?.Invoke();
        return result;
    }

    public async Task<ProfileResult> SaveAsNewVersionAsync(string versionName)
    {
        if (SelectedFilament is null)
        {
            ErrorMessage = "Pick a filament.";
            return new ProfileResult { Ok = false, Errors = new Dictionary<string, string> { ["Filament"] = ErrorMessage } };
        }

        var input = BuildInput();
        input.VersionNumber = VersionNumber + 1;
        input.VersionName = versionName;

        var result = await _profileService.CreateProfileAsync(SelectedFilament.Id, input);
        if (!result.Ok)
        {
            ErrorMessage = string.Join(", ", result.Errors!.Values);
            return result;
        }

        await PushCurrentFieldsToFileAsync();
        IsDirty = false;
        PendingNewVersion = false;
        Close?.Invoke();
        return result;
    }

    public Task RenameVersionAsync(PrintProfile version, string? versionName) =>
        _profileService.RenameVersionAsync(version.Id, versionName);

    [RelayCommand]
    private async Task DeleteAsync()
    {
        if (!_id.HasValue) return;
        await _profileService.DeleteProfileAsync(_id.Value);
        Close?.Invoke();
    }

    [RelayCommand]
    private async Task DuplicateAsync()
    {
        if (!_id.HasValue) return;
        await _profileService.DuplicateProfileAsync(_id.Value);
        Close?.Invoke();
    }

    [RelayCommand]
    private void Cancel() => Close?.Invoke();

    [RelayCommand]
    private async Task LinkToUserPresetAsync()
    {
        if (SelectedUserPreset is not null)
            await LinkToFileAsync(SelectedUserPreset.FilePath);
    }

    [RelayCommand]
    private async Task LinkToFileAsync(string filePath)
    {
        if (string.IsNullOrWhiteSpace(filePath)) return;

        var result = await _importService.ImportAsync(filePath);
        if (!result.Ok)
        {
            LinkHint = $"Link failed: {result.Error}";
            return;
        }

        if (string.IsNullOrWhiteSpace(Name)) Name = result.SuggestedName ?? "";
        foreach (var entry in FieldTabs.SelectMany(t => t.Sections).SelectMany(g => g.Fields))
        {
            if (result.Fields!.TryGetValue(entry.Name, out var value))
                entry.Value = value;
        }
        if (result.Fields!.TryGetValue("NozzleTempC", out var nozzle))
            NozzleTempC = nozzle;
        if (result.Fields!.TryGetValue("NozzleTempInitialC", out var initial))
            NozzleTempInitialC = initial;

        _source = ProfileSource.SlicerImport;
        _sourceSlicer = SlicerType.BambuStudio;
        _rawSettingsJson = result.RawSettingsJson;
        _sourcePresetPath = filePath;
        LinkedFileLabel = $"Linked to: {Path.GetFileName(filePath)} — fully resolved via inherits chain";
        LinkHint = "Parsed — review before saving.";
    }

    [RelayCommand]
    private async Task CheckForUpdatesAsync()
    {
        if (_sourcePresetPath is null) return;

        var result = await _importService.ImportAsync(_sourcePresetPath);
        if (!result.Ok)
        {
            LinkHint = $"Check failed: {result.Error}";
            return;
        }

        if (result.RawSettingsJson == _rawSettingsJson)
        {
            LinkHint = "No changes since last check — already up to date with the linked file.";
            return;
        }

        foreach (var entry in FieldTabs.SelectMany(t => t.Sections).SelectMany(g => g.Fields))
        {
            if (result.Fields!.TryGetValue(entry.Name, out var value))
                entry.Value = value;
        }
        if (result.Fields!.TryGetValue("NozzleTempC", out var nozzle))
            NozzleTempC = nozzle;
        if (result.Fields!.TryGetValue("NozzleTempInitialC", out var initial))
            NozzleTempInitialC = initial;

        _rawSettingsJson = result.RawSettingsJson;
        PendingNewVersion = true;
        IsDirty = true;
        LinkHint = "Linked file changed — save to create a new version.";
    }

    public async Task LoadVersionAsync(PrintProfile version)
    {
        var target = new ProfileEditViewModel(_profileService, _importService, _filamentService, version);
        await target.PushCurrentFieldsToFileAsync();
        SwitchTo?.Invoke(target);
    }
}
