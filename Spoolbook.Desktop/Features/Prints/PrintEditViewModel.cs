using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Features.Spools;
namespace Spoolbook.Desktop.Features.Prints;

public partial class PrintEditViewModel : ViewModelBase
{
    private readonly PrintService _printService;
    private readonly SpoolService _spoolService;
    private readonly PrintProfileService _profileService;
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
    private string printer = "Bambu Lab P2S";

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

    public PrintEditViewModel(PrintService printService, SpoolService spoolService, PrintProfileService profileService, Print? existing)
    {
        _printService = printService;
        _spoolService = spoolService;
        _profileService = profileService;

        if (existing is not null)
        {
            _id = existing.Id;
            IsEdit = true;
            SelectedSpool = existing.Spool;
            Printer = existing.Printer;
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
    }

    private async Task LoadSpoolOptionsAsync(PrintProfile? existingProfile)
    {
        SpoolOptions = new ObservableCollection<Spool>(await _spoolService.ListAllAsync());
        if (SelectedSpool is not null)
            await LoadProfileOptionsAsync(SelectedSpool.FilamentId, existingProfile);
    }

    partial void OnSelectedSpoolChanged(Spool? value)
    {
        if (value is not null) _ = LoadProfileOptionsAsync(value.FilamentId, null);
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
            Printer = Printer,
            StartedAt = StartedDate.Value.Date + StartedTime.Value,
            EndedAt = EndedDate.Value.Date + EndedTime.Value,
            Status = Status,
            Notes = string.IsNullOrWhiteSpace(Notes) ? null : Notes,
            AmsHumidityPct = amsHumidity,
            ActualRoomTempC = actualRoomTemp,
            CleanBuildPlate = CleanBuildPlate
        };

        var result = _id.HasValue
            ? await _printService.UpdateAsync(_id.Value, input)
            : await _printService.CreateAsync(SelectedProfile.Id, SelectedSpool.Id, input);

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
