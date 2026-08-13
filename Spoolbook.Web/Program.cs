using System.Security.Claims;
using Microsoft.AspNetCore.Authentication;
using Microsoft.AspNetCore.Authentication.Cookies;
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
using Spoolbook.Web.Services;

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
builder.Services.AddScoped<ProfileInventoryService>();
builder.Services.AddScoped<PrintService>();
builder.Services.AddScoped<PrintInventoryService>();
builder.Services.AddScoped<PrinterService>();
builder.Services.AddScoped<PrinterTelemetryService>();
builder.Services.AddScoped<ProjectService>();
builder.Services.AddScoped<ProjectUploadService>();
builder.Services.AddSingleton<PrinterLiveStatusStore>();
builder.Services.AddHostedService<PrinterMqttHostedService>();
builder.Services.AddScoped<PrinterConnectionTestService>();
builder.Services.AddScoped<PrinterControlService>();
builder.Services.AddScoped<AppSettingsService>();
builder.Services.AddScoped<DashboardMetricsService>();
builder.Services.AddScoped<BambuFilamentImportService>(_ => new BambuFilamentImportService(
    new BambuPresetResolver(BambuPaths.FindUserFilamentPresetsDir() ?? "", BambuPaths.FindSystemProfilesDir() ?? "")));
builder.Services.AddScoped<IWeatherService, OpenMeteoWeatherService>();

// Single shared-secret login gating mutating pages — reactivates the v2 model from
// docs/adr/0005-access-control-v1-vercel-gate-v2-mutation-lock.md for the LAN pivot
// (docs/adr/0018). Still single-editor: no user table, no OAuth.
builder.Services.AddAuthentication(CookieAuthenticationDefaults.AuthenticationScheme)
    .AddCookie(options => options.LoginPath = "/login");
builder.Services.AddAuthorization();
builder.Services.AddCascadingAuthenticationState();

builder.Services.AddRazorComponents()
    .AddInteractiveServerComponents();

var app = builder.Build();

using (var scope = app.Services.CreateScope())
    scope.ServiceProvider.GetRequiredService<SpoolbookDbContext>().Database.Migrate();

// Throttled to once/24h via AppSettings.LastFilamentSyncAt, same as the desktop app's
// App.axaml.cs — silent on failure, the Filaments page's manual sync button surfaces errors
// for an explicit attempt.
using (var scope = app.Services.CreateScope())
{
    var appSettingsService = scope.ServiceProvider.GetRequiredService<AppSettingsService>();
    var appSettings = await appSettingsService.GetAsync();
    if (appSettings.LastFilamentSyncAt is null || DateTime.UtcNow - appSettings.LastFilamentSyncAt.Value > TimeSpan.FromHours(24))
    {
        _ = Task.Run(async () =>
        {
            using var syncScope = app.Services.CreateScope();
            var filamentService = syncScope.ServiceProvider.GetRequiredService<FilamentService>();
            var settingsService = syncScope.ServiceProvider.GetRequiredService<AppSettingsService>();
            var additionalSources = await settingsService.GetAdditionalFilamentSourceUrlsAsync();
            var result = await new FilamentCatalogSyncService().FetchAsync(additionalSources);
            if (!result.Ok) return;
            await filamentService.ImportManyAsync(result.Entries);
            await settingsService.RecordFilamentSyncAsync();
        });
    }
}

// Configure the HTTP request pipeline.
if (!app.Environment.IsDevelopment())
{
    app.UseExceptionHandler("/Error", createScopeForErrors: true);
    // The default HSTS value is 30 days. You may want to change this for production scenarios, see https://aka.ms/aspnetcore-hsts.
    app.UseHsts();
}
app.UseStatusCodePagesWithReExecute("/not-found", createScopeForStatusCodePages: true);
app.UseHttpsRedirection();

app.UseAuthentication();
app.UseAuthorization();
app.UseAntiforgery();

// Plain HTML form + minimal API, deliberately outside the Blazor component tree — an
// interactive component can't call HttpContext.SignInAsync (response has already started
// by the time its circuit runs), so login/logout stay ordinary HTTP endpoints.
app.MapGet("/login", (string? returnUrl, string? error) => Results.Content($"""
    <!DOCTYPE html>
    <html>
    <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>Spoolbook</title>
        <link rel="stylesheet" href="/css/tailwind.css" />
    </head>
    <body class="flex min-h-screen items-center justify-center bg-slate-50 font-sans">
        <div class="w-full max-w-sm rounded-lg border border-slate-200 bg-white p-8 shadow-sm">
            <h1 class="mb-6 text-xl font-semibold text-slate-900">Spoolbook</h1>
            <form method="post" action="/login?returnUrl={Uri.EscapeDataString(returnUrl ?? "/")}" class="space-y-3">
                <input type="password" name="password" placeholder="Password" autofocus
                       class="w-full rounded-md border border-slate-300 px-3 py-2 text-sm focus:border-slate-500 focus:outline-none focus:ring-1 focus:ring-slate-500" />
                <button type="submit" class="w-full rounded-md bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-700">Sign in</button>
            </form>
            {(error is not null ? "<p class=\"mt-3 text-sm text-red-600\">Wrong password.</p>" : "")}
        </div>
    </body>
    </html>
    """, "text/html"));

app.MapPost("/login", async (HttpContext ctx, string? returnUrl) =>
{
    var form = await ctx.Request.ReadFormAsync();
    var password = form["password"].ToString();
    var expected = Environment.GetEnvironmentVariable("SPOOLBOOK_ADMIN_PASSWORD");

    if (string.IsNullOrEmpty(expected) || password != expected)
        return Results.Redirect($"/login?returnUrl={Uri.EscapeDataString(returnUrl ?? "/")}&error=1");

    var identity = new ClaimsIdentity([new Claim(ClaimTypes.Name, "editor")], CookieAuthenticationDefaults.AuthenticationScheme);
    await ctx.SignInAsync(CookieAuthenticationDefaults.AuthenticationScheme, new ClaimsPrincipal(identity));
    return Results.Redirect(returnUrl ?? "/");
});

app.MapPost("/logout", async (HttpContext ctx) =>
{
    await ctx.SignOutAsync(CookieAuthenticationDefaults.AuthenticationScheme);
    return Results.Redirect("/");
});

app.MapStaticAssets();
app.MapRazorComponents<App>()
    .AddInteractiveServerRenderMode();

app.Run();
