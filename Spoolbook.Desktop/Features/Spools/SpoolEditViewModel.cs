using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.Settings.Filaments;
namespace Spoolbook.Desktop.Features.Spools;

public partial class SpoolEditViewModel : ViewModelBase
{
    private readonly SpoolService _service;
    private readonly FilamentService _filamentService;
    private readonly int? _id;

    [ObservableProperty]
    private ObservableCollection<Filament> filamentOptions = new();

    [ObservableProperty]
    private Filament? selectedFilament;

    [ObservableProperty]
    private string? lotCode;

    [ObservableProperty]
    private DateTimeOffset? purchasedAt;

    [ObservableProperty]
    private DateTimeOffset? openedAt;

    [ObservableProperty]
    private DateTimeOffset? emptiedAt;

    [ObservableProperty]
    private string? weightGramsText;

    [ObservableProperty]
    private decimal? selectedDiameterMm;

    [ObservableProperty]
    private string? notes;

    public static decimal[] DiameterOptions { get; } = [1.75m, 2.85m];

    [ObservableProperty]
    private string? errorMessage;

    public bool IsEdit { get; }
    public string PageTitle => IsEdit ? "Edit spool" : "Add spool";
    public Action? Close { get; set; }

    public SpoolEditViewModel(SpoolService service, FilamentService filamentService, Spool? existing)
    {
        _service = service;
        _filamentService = filamentService;

        if (existing is not null)
        {
            _id = existing.Id;
            IsEdit = true;
            SelectedFilament = existing.Filament;
            LotCode = existing.LotCode;
            PurchasedAt = ToOffset(existing.PurchasedAt);
            OpenedAt = ToOffset(existing.OpenedAt);
            EmptiedAt = ToOffset(existing.EmptiedAt);
            WeightGramsText = existing.WeightGrams?.ToString();
            SelectedDiameterMm = existing.DiameterMm;
            Notes = existing.Notes;
        }

        _ = LoadFilamentOptionsAsync();
    }

    private async Task LoadFilamentOptionsAsync()
    {
        FilamentOptions = new ObservableCollection<Filament>(await _filamentService.ListAsync());
    }

    private static DateTimeOffset? ToOffset(DateOnly? date) =>
        date is { } d ? new DateTimeOffset(d.ToDateTime(TimeOnly.MinValue), TimeSpan.Zero) : null;

    private static DateOnly? ToDateOnly(DateTimeOffset? offset) =>
        offset is { } o ? DateOnly.FromDateTime(o.DateTime) : null;

    [RelayCommand]
    private async Task SaveAsync()
    {
        if (SelectedFilament is null)
        {
            ErrorMessage = "Pick a filament.";
            return;
        }

        int? weightGrams = null;
        if (!string.IsNullOrWhiteSpace(WeightGramsText))
        {
            if (!int.TryParse(WeightGramsText, out var parsed))
            {
                ErrorMessage = "Weight must be a whole number of grams.";
                return;
            }
            weightGrams = parsed;
        }

        var input = new SpoolInput
        {
            LotCode = string.IsNullOrWhiteSpace(LotCode) ? null : LotCode,
            PurchasedAt = ToDateOnly(PurchasedAt),
            OpenedAt = ToDateOnly(OpenedAt),
            EmptiedAt = ToDateOnly(EmptiedAt),
            WeightGrams = weightGrams,
            DiameterMm = SelectedDiameterMm,
            Notes = string.IsNullOrWhiteSpace(Notes) ? null : Notes
        };

        var result = _id.HasValue
            ? await _service.UpdateSpoolAsync(_id.Value, input)
            : await _service.CreateSpoolAsync(SelectedFilament.Id, input);

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
        await _service.DeleteSpoolAsync(_id.Value);
        Close?.Invoke();
    }

    [RelayCommand]
    private void Cancel() => Close?.Invoke();
}
