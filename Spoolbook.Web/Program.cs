using Microsoft.EntityFrameworkCore;
using Spoolbook.Desktop.Data;
using Spoolbook.Desktop.Features.BambuImport;
using Spoolbook.Desktop.Features.Dashboard;
using Spoolbook.Desktop.Features.Profiles;
using Spoolbook.Desktop.Features.Prints;
using Spoolbook.Desktop.Features.Settings.Colors;
using Spoolbook.Desktop.Features.Settings.Filaments;
using Spoolbook.Desktop.Features.Settings.General;
using Spoolbook.Desktop.Features.Settings.Printers;
using Spoolbook.Desktop.Features.Spools;
using Spoolbook.Desktop.Services.Weather;
using Spoolbook.Web.Components;

var builder = WebApplication.CreateBuilder(args);

// Same DB file the Avalonia desktop app uses — parallel-run migration, docs/adr/0018.
var dataDir = Path.Combine(
    Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
    "spoolbook");
Directory.CreateDirectory(dataDir);
var dbPath = Path.Combine(dataDir, "spoolbook.db");

builder.Services.AddDbContext<SpoolbookDbContext>(options => options.UseSqlite($"Data Source={dbPath}"));

builder.Services.AddScoped<FilamentService>();
builder.Services.AddScoped<FilamentColorService>();
builder.Services.AddScoped<SpoolService>();
builder.Services.AddScoped<PrintProfileService>();
builder.Services.AddScoped<PrintService>();
builder.Services.AddScoped<PrinterService>();
builder.Services.AddScoped<PrinterTelemetryService>();
builder.Services.AddScoped<ProjectService>();
builder.Services.AddScoped<AppSettingsService>();
builder.Services.AddScoped<DashboardMetricsService>();
builder.Services.AddScoped<BambuFilamentImportService>(_ => new BambuFilamentImportService(
    new BambuPresetResolver(BambuPaths.FindUserFilamentPresetsDir() ?? "", BambuPaths.FindSystemProfilesDir() ?? "")));
builder.Services.AddScoped<IWeatherService, OpenMeteoWeatherService>();

builder.Services.AddRazorComponents()
    .AddInteractiveServerComponents();

var app = builder.Build();

using (var scope = app.Services.CreateScope())
    scope.ServiceProvider.GetRequiredService<SpoolbookDbContext>().Database.Migrate();

// Configure the HTTP request pipeline.
if (!app.Environment.IsDevelopment())
{
    app.UseExceptionHandler("/Error", createScopeForErrors: true);
    // The default HSTS value is 30 days. You may want to change this for production scenarios, see https://aka.ms/aspnetcore-hsts.
    app.UseHsts();
}
app.UseStatusCodePagesWithReExecute("/not-found", createScopeForStatusCodePages: true);
app.UseHttpsRedirection();

app.UseAntiforgery();

app.MapStaticAssets();
app.MapRazorComponents<App>()
    .AddInteractiveServerRenderMode();

app.Run();
