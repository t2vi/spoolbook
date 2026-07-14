using Spoolbook.Desktop.Common;
using Spoolbook.Desktop.Features.BambuImport;
using Spoolbook.Desktop.Features.Settings.Colors;
using Spoolbook.Desktop.Features.Settings.General;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Settings.Printers;

namespace Spoolbook.Desktop.Features.Settings;

public class SettingsViewModel : ViewModelBase
{
    public GeneralSettingsViewModel General { get; }
    public ColorsViewModel Colors { get; }
    public FilamentsViewModel Filaments { get; }
    public PrintersViewModel Printers { get; }

    public SettingsViewModel(
        FilamentColorService colorService,
        FilamentService filamentService,
        AppSettingsService appSettingsService,
        BambuFilamentImportService importService,
        PrinterService printerService)
    {
        General = new GeneralSettingsViewModel(appSettingsService, importService);
        Colors = new ColorsViewModel(colorService);
        Filaments = new FilamentsViewModel(filamentService, colorService, appSettingsService);
        Printers = new PrintersViewModel(printerService);
    }
}
