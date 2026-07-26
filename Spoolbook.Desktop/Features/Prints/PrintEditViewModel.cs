using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Features.Settings.Printers;
namespace Spoolbook.Desktop.Features.Prints;

public partial class PrintEditViewModel : EditViewModelBase
{
    private readonly PrintService _printService;
    private readonly SpoolService _spoolService;
    private readonly PrintProfileService _profileService;
    private readonly PrinterService _printerService;
    private readonly ProjectService _projectService;
    private readonly int? _id;

    [ObservableProperty]
    private ObservableCollection<Spool> spoolOptions = new();

    [ObservableProperty]
    private Spool? selectedSpool;

    [ObservableProperty]
    private ObservableCollection<PrintProfile> profileOptions = new();

    [ObservableProperty]
    private PrintProfile? selectedProfile;

    [ObservableProperty]
    private ObservableCollection<Printer> printerOptions = new();

    [ObservableProperty]
    private Printer? selectedPrinter;

    [ObservableProperty]
    private ObservableCollection<Project> projectOptions = new();

    [ObservableProperty]
    private Project? selectedProject;

    [ObservableProperty]
    private string? projectStatusText;

    [ObservableProperty]
    private ObservableCollection<ProjectPlate> plateOptions = new();

    [ObservableProperty]
    private ProjectPlate? selectedPlate;

    [ObservableProperty]
    private bool isLoadingPlates;

    public bool HasMultiplePlates => PlateOptions.Count > 1;
    public string PlateIndexText => SelectedPlate is null ? "" : $"{PlateOptions.IndexOf(SelectedPlate) + 1} / {PlateOptions.Count}";

    private string? _preselectPlaterId;
    private Task _plateLoadTask = Task.CompletedTask;

    [ObservableProperty]
    private DateTime? startedDate;

    [ObservableProperty]
    private TimeSpan? startedTime;

    [ObservableProperty]
    private DateTime? endedDate;

    [ObservableProperty]
    private TimeSpan? endedTime;

    [ObservableProperty]
    private PrintStatus status;

    [ObservableProperty]
    private string? notes;

    [ObservableProperty]
    private decimal? amsHumidityPct;

    [ObservableProperty]
    private decimal? actualRoomTempC;

    [ObservableProperty]
    private bool? cleanBuildPlate;

    [ObservableProperty]
    private string? errorMessage;

    [ObservableProperty]
    private bool spoolInvalid;

    [ObservableProperty]
    private bool profileInvalid;

    [ObservableProperty]
    private bool printerInvalid;

    [ObservableProperty]
    private bool startedInvalid;

    [ObservableProperty]
    private bool endedInvalid;

    public static PrintStatus[] StatusOptions { get; } = Enum.GetValues<PrintStatus>();

    public bool IsEdit { get; }
    public string PageTitle => IsEdit ? "Edit print" : "Add print";
    public Action? Close { get; set; }

    public PrintEditViewModel(PrintService printService, SpoolService spoolService, PrintProfileService profileService, PrinterService printerService, ProjectService projectService, Print? existing)
    {
        _printService = printService;
        _spoolService = spoolService;
        _profileService = profileService;
        _printerService = printerService;
        _projectService = projectService;

        if (existing is not null)
        {
            _id = existing.Id;
            IsEdit = true;
            SelectedSpool = existing.Spool;
            StartedDate = existing.StartedAt.Date;
            StartedTime = existing.StartedAt.TimeOfDay;
            EndedDate = existing.EndedAt.Date;
            EndedTime = existing.EndedAt.TimeOfDay;
            Status = existing.Status;
            Notes = existing.Notes;
            AmsHumidityPct = existing.AmsHumidityPct;
            ActualRoomTempC = existing.ActualRoomTempC;
            CleanBuildPlate = existing.CleanBuildPlate;
            _preselectPlaterId = existing.ProjectPlaterId;
        }

        _ = InitializeOptionsAsync(existing);
    }

    // Awaits all option-list loads before flipping Loaded — several of them (printer, project)
    // preselect a value asynchronously, which would otherwise falsely mark the form dirty the
    // moment the load lands rather than only on an actual user edit.
    private async Task InitializeOptionsAsync(Print? existing)
    {
        await Task.WhenAll(
            LoadSpoolOptionsAsync(existing?.Profile),
            LoadPrinterOptionsAsync(existing?.Printer),
            LoadProjectOptionsAsync(existing?.Project));
        await _plateLoadTask;
        Loaded = true;
    }

    private async Task LoadSpoolOptionsAsync(PrintProfile? existingProfile)
    {
        SpoolOptions = new ObservableCollection<Spool>(await _spoolService.ListAllAsync());
        if (SelectedSpool is not null)
            await LoadProfileOptionsAsync(SelectedSpool.FilamentId, existingProfile);
    }

    private async Task LoadPrinterOptionsAsync(Printer? preselect)
    {
        PrinterOptions = new ObservableCollection<Printer>(await _printerService.ListAsync());
        SelectedPrinter = preselect is not null
            ? PrinterOptions.FirstOrDefault(p => p.Id == preselect.Id) ?? preselect
            : PrinterOptions.FirstOrDefault();
    }

    partial void OnSelectedSpoolChanged(Spool? value)
    {
        if (value is not null)
        {
            SpoolInvalid = false;
            _ = LoadProfileOptionsAsync(value.FilamentId, null);
        }
        MarkDirty();
    }

    partial void OnSelectedProjectChanged(Project? value)
    {
        ProjectStatusText = value is null ? null : DescribeStatus(ProjectService.GetFileStatus(value));
        _plateLoadTask = LoadPlatesAsync(value);
        MarkDirty();
    }

    partial void OnSelectedPlateChanged(ProjectPlate? value)
    {
        OnPropertyChanged(nameof(PlateIndexText));
        PreviousPlateCommand.NotifyCanExecuteChanged();
        NextPlateCommand.NotifyCanExecuteChanged();
        MarkDirty();
    }

    [RelayCommand(CanExecute = nameof(CanGoPreviousPlate))]
    private void PreviousPlate()
    {
        var index = SelectedPlate is null ? -1 : PlateOptions.IndexOf(SelectedPlate);
        if (index > 0) SelectedPlate = PlateOptions[index - 1];
    }

    private bool CanGoPreviousPlate() => SelectedPlate is not null && PlateOptions.IndexOf(SelectedPlate) > 0;

    [RelayCommand(CanExecute = nameof(CanGoNextPlate))]
    private void NextPlate()
    {
        var index = SelectedPlate is null ? -1 : PlateOptions.IndexOf(SelectedPlate);
        if (index >= 0 && index < PlateOptions.Count - 1) SelectedPlate = PlateOptions[index + 1];
    }

    private bool CanGoNextPlate() => SelectedPlate is not null && PlateOptions.IndexOf(SelectedPlate) < PlateOptions.Count - 1;

    private async Task LoadPlatesAsync(Project? project)
    {
        if (project is null || ProjectService.GetFileStatus(project) == ProjectFileStatus.Missing)
        {
            PlateOptions = new ObservableCollection<ProjectPlate>();
            SelectedPlate = null;
            return;
        }

        IsLoadingPlates = true;
        List<ProjectPlate> plates;
        try
        {
            plates = await Task.Run(() => ProjectService.ReadPlates(project.FilePath));
        }
        catch (Exception)
        {
            plates = [];
        }
        finally
        {
            IsLoadingPlates = false;
        }

        PlateOptions = new ObservableCollection<ProjectPlate>(plates);
        OnPropertyChanged(nameof(HasMultiplePlates));

        var preselectId = _preselectPlaterId;
        _preselectPlaterId = null;
        SelectedPlate = preselectId is not null
            ? PlateOptions.FirstOrDefault(p => p.PlaterId == preselectId)
            : PlateOptions.FirstOrDefault();
    }

    partial void OnSelectedProfileChanged(PrintProfile? value)
    {
        if (value is not null) ProfileInvalid = false;
        MarkDirty();
    }

    partial void OnSelectedPrinterChanged(Printer? value)
    {
        if (value is not null) PrinterInvalid = false;
        MarkDirty();
    }

    partial void OnStartedDateChanged(DateTime? value)
    {
        if (value is not null && StartedTime is not null) StartedInvalid = false;
        MarkDirty();
    }

    partial void OnStartedTimeChanged(TimeSpan? value)
    {
        if (StartedDate is not null && value is not null) StartedInvalid = false;
        MarkDirty();
    }

    partial void OnEndedDateChanged(DateTime? value)
    {
        if (value is not null && EndedTime is not null) EndedInvalid = false;
        MarkDirty();
    }

    partial void OnEndedTimeChanged(TimeSpan? value)
    {
        if (EndedDate is not null && value is not null) EndedInvalid = false;
        MarkDirty();
    }
    partial void OnStatusChanged(PrintStatus value) => MarkDirty();
    partial void OnNotesChanged(string? value) => MarkDirty();
    partial void OnAmsHumidityPctChanged(decimal? value) => MarkDirty();
    partial void OnActualRoomTempCChanged(decimal? value) => MarkDirty();
    partial void OnCleanBuildPlateChanged(bool? value) => MarkDirty();

    private static string? DescribeStatus(ProjectFileStatus status) => status switch
    {
        ProjectFileStatus.Missing => "File not found at the linked path.",
        ProjectFileStatus.Changed => "File may have changed since it was attached.",
        _ => null
    };

    private async Task LoadProjectOptionsAsync(Project? preselect)
    {
        ProjectOptions = new ObservableCollection<Project>(await _projectService.ListAsync());
        if (preselect is not null && !ProjectOptions.Any(p => p.Id == preselect.Id))
            ProjectOptions.Add(preselect);
        SelectedProject = preselect is not null
            ? ProjectOptions.FirstOrDefault(p => p.Id == preselect.Id)
            : null;
    }

    [RelayCommand]
    private async Task AttachProjectFileAsync(string filePath)
    {
        if (string.IsNullOrWhiteSpace(filePath)) return;

        var result = await _projectService.UpsertByPathAsync(filePath);
        if (!result.Ok)
        {
            ErrorMessage = result.Error == "file_not_found" ? "That file could not be found." : result.Error;
            return;
        }

        if (!ProjectOptions.Any(p => p.Id == result.Project!.Id))
            ProjectOptions.Add(result.Project!);
        SelectedProject = ProjectOptions.First(p => p.Id == result.Project!.Id);
    }

    private async Task LoadProfileOptionsAsync(int filamentId, PrintProfile? preselect)
    {
        ProfileOptions = new ObservableCollection<PrintProfile>(await _profileService.ListProfilesForFilamentAsync(filamentId));
        if (preselect is not null && !ProfileOptions.Any(p => p.Id == preselect.Id))
            ProfileOptions.Add(preselect);
        SelectedProfile = preselect ?? ProfileOptions.FirstOrDefault();
    }

    [RelayCommand]
    private async Task SaveAsync()
    {
        if (IsLoadingPlates)
        {
            ErrorMessage = "Still loading plate thumbnails — wait for that to finish.";
            return;
        }

        SpoolInvalid = SelectedSpool is null;
        ProfileInvalid = SelectedProfile is null;
        PrinterInvalid = SelectedPrinter is null;
        StartedInvalid = StartedDate is null || StartedTime is null;
        EndedInvalid = EndedDate is null || EndedTime is null;

        if (SelectedSpool is null)
        {
            ErrorMessage = "Pick a spool.";
            return;
        }
        if (SelectedProfile is null)
        {
            ErrorMessage = "Pick a profile.";
            return;
        }
        if (SelectedPrinter is null)
        {
            ErrorMessage = "Pick a printer.";
            return;
        }
        if (StartedDate is null || StartedTime is null || EndedDate is null || EndedTime is null)
        {
            ErrorMessage = "Enter both start and end date/time.";
            return;
        }

        var input = new PrintInput
        {
            StartedAt = StartedDate.Value.Date + StartedTime.Value,
            EndedAt = EndedDate.Value.Date + EndedTime.Value,
            Status = Status,
            Notes = string.IsNullOrWhiteSpace(Notes) ? null : Notes,
            AmsHumidityPct = AmsHumidityPct.HasValue ? (int)Math.Round(AmsHumidityPct.Value) : null,
            ActualRoomTempC = ActualRoomTempC,
            CleanBuildPlate = CleanBuildPlate,
            ProjectId = SelectedProject?.Id,
            ProjectPlaterId = SelectedProject is not null ? SelectedPlate?.PlaterId : null
        };

        var result = _id.HasValue
            ? await _printService.UpdateAsync(_id.Value, SelectedPrinter.Id, input)
            : await _printService.CreateAsync(SelectedProfile.Id, SelectedSpool.Id, SelectedPrinter.Id, input);

        if (!result.Ok)
        {
            ErrorMessage = result.Error;
            return;
        }

        Close?.Invoke();
    }

    [RelayCommand]
    private async Task DeleteAsync()
    {
        if (!_id.HasValue) return;
        await _printService.DeleteAsync(_id.Value);
        Close?.Invoke();
    }

    [RelayCommand]
    private void Cancel() => Close?.Invoke();
}
