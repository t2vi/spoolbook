using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Features.Settings.Printers;
namespace Spoolbook.Desktop.Features.Prints;

public partial class PrintEditViewModel : ViewModelBase
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
    private DateTimeOffset? startedDate;

    [ObservableProperty]
    private TimeSpan? startedTime;

    [ObservableProperty]
    private DateTimeOffset? endedDate;

    [ObservableProperty]
    private TimeSpan? endedTime;

    [ObservableProperty]
    private PrintStatus status;

    [ObservableProperty]
    private string? notes;

    [ObservableProperty]
    private string? amsHumidityText;

    [ObservableProperty]
    private string? actualRoomTempText;

    [ObservableProperty]
    private bool? cleanBuildPlate;

    [ObservableProperty]
    private string? errorMessage;

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
            StartedDate = new DateTimeOffset(existing.StartedAt.Date, TimeSpan.Zero);
            StartedTime = existing.StartedAt.TimeOfDay;
            EndedDate = new DateTimeOffset(existing.EndedAt.Date, TimeSpan.Zero);
            EndedTime = existing.EndedAt.TimeOfDay;
            Status = existing.Status;
            Notes = existing.Notes;
            AmsHumidityText = existing.AmsHumidityPct?.ToString();
            ActualRoomTempText = existing.ActualRoomTempC?.ToString();
            CleanBuildPlate = existing.CleanBuildPlate;
        }

        _ = LoadSpoolOptionsAsync(existing?.Profile);
        _ = LoadPrinterOptionsAsync(existing?.Printer);
        _ = LoadProjectOptionsAsync(existing?.Project);
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
        if (value is not null) _ = LoadProfileOptionsAsync(value.FilamentId, null);
    }

    partial void OnSelectedProjectChanged(Project? value)
    {
        ProjectStatusText = value is null ? null : DescribeStatus(ProjectService.GetFileStatus(value));
    }

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

        int? amsHumidity = null;
        if (!string.IsNullOrWhiteSpace(AmsHumidityText))
        {
            if (!int.TryParse(AmsHumidityText, out var parsed))
            {
                ErrorMessage = "AMS humidity must be a whole number.";
                return;
            }
            amsHumidity = parsed;
        }

        decimal? actualRoomTemp = null;
        if (!string.IsNullOrWhiteSpace(ActualRoomTempText))
        {
            if (!decimal.TryParse(ActualRoomTempText, out var parsedTemp))
            {
                ErrorMessage = "Room temp must be a number.";
                return;
            }
            actualRoomTemp = parsedTemp;
        }

        var input = new PrintInput
        {
            StartedAt = StartedDate.Value.Date + StartedTime.Value,
            EndedAt = EndedDate.Value.Date + EndedTime.Value,
            Status = Status,
            Notes = string.IsNullOrWhiteSpace(Notes) ? null : Notes,
            AmsHumidityPct = amsHumidity,
            ActualRoomTempC = actualRoomTemp,
            CleanBuildPlate = CleanBuildPlate,
            ProjectId = SelectedProject?.Id
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
