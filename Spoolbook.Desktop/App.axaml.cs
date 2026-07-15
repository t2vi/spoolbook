using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Input;
using Avalonia.Markup.Xaml;
using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Features.Prints;
using Spoolbook.Desktop.Features.Settings.General;
using Spoolbook.Desktop.Features.Settings.Colors;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Settings.Printers;
using Spoolbook.Desktop.Features.BambuImport;
using Spoolbook.Desktop.Features.Dashboard;
using Spoolbook.Desktop.Shell;
namespace Spoolbook.Desktop;

public partial class App : Application
{
    public override void Initialize()
    {
        AvaloniaXamlLoader.Load(this);
    }

    public override void OnFrameworkInitializationCompleted()
    {
        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
        {
            SetupNativeMenu(desktop);

            var dataDir = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
                "spoolbook");
            Directory.CreateDirectory(dataDir);
            var dbPath = Path.Combine(dataDir, "spoolbook.db");

            var options = new DbContextOptionsBuilder<SpoolbookDbContext>()
                .UseSqlite($"Data Source={dbPath}")
                .Options;

            var db = new SpoolbookDbContext(options);
            db.Database.Migrate();

            var filamentService = new FilamentService(db);
            var spoolService = new SpoolService(db);
            var profileService = new PrintProfileService(db);
            var spoolInventoryService = new SpoolInventoryService(db);
            var profileInventoryService = new ProfileInventoryService(db);
            var printService = new PrintService(db, new OpenMeteoWeatherService());
            var printInventoryService = new PrintInventoryService(db);
            var colorService = new FilamentColorService(db);
            var appSettingsService = new AppSettingsService(db);
            var metricsService = new DashboardMetricsService(db, appSettingsService);
            var printerService = new PrinterService(db);
            var projectService = new ProjectService(db);

            Converters.ColorSwatchConverter.SetPalette(colorService.ListAsync().GetAwaiter().GetResult());

            var appSettings = appSettingsService.GetAsync().GetAwaiter().GetResult();
            var resolver = new BambuPresetResolver(
                appSettings.BambuUserPresetsDir ?? BambuPaths.FindUserFilamentPresetsDir() ?? "",
                appSettings.BambuSystemProfilesDir ?? BambuPaths.FindSystemProfilesDir() ?? "");
            var importService = new BambuFilamentImportService(resolver);

            desktop.MainWindow = new MainWindow
            {
                DataContext = new MainWindowViewModel(
                    filamentService, spoolService, profileService, importService,
                    spoolInventoryService, profileInventoryService, printService, printInventoryService, colorService,
                    appSettingsService, metricsService, printerService, projectService),
            };

            if (appSettings.LastFilamentSyncAt is null || DateTime.UtcNow - appSettings.LastFilamentSyncAt.Value > TimeSpan.FromHours(24))
                _ = SyncFilamentCatalogInBackgroundAsync(filamentService, appSettingsService);
        }

        base.OnFrameworkInitializationCompleted();
    }

    // Throttled to once/24h via AppSettings.LastFilamentSyncAt — matches the scraper's own
    // daily cadence, no point checking more often than the source updates.
    // Silent on failure: offline is a normal state for a local desktop app, and the manual
    // "Sync filament catalog" button still surfaces errors for an explicit attempt.
    private static async Task SyncFilamentCatalogInBackgroundAsync(FilamentService filamentService, AppSettingsService appSettingsService)
    {
        var additionalSources = await appSettingsService.GetAdditionalFilamentSourceUrlsAsync();
        var result = await new FilamentCatalogSyncService().FetchAsync(additionalSources);
        if (!result.Ok) return;

        await filamentService.ImportManyAsync(result.Entries);
        await appSettingsService.RecordFilamentSyncAsync();
    }

    // macOS ignores CFBundleName for the menu-bar app menu title — it's driven by the
    // first NativeMenuItem instead, which otherwise defaults to "Avalonia Application".
    private void SetupNativeMenu(IClassicDesktopStyleApplicationLifetime desktop)
    {
        var quit = new NativeMenuItem("Quit Spoolbook") { Gesture = new KeyGesture(Key.Q, KeyModifiers.Meta) };
        quit.Click += (_, _) => desktop.Shutdown();

        var appMenu = new NativeMenuItem("Spoolbook")
        {
            Menu = new NativeMenu { Items = { quit } }
        };

        NativeMenu.SetMenu(this, new NativeMenu { Items = { appMenu } });
    }
}