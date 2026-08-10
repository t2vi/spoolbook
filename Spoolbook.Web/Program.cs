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
builder.Services.AddScoped<PrinterService>();
builder.Services.AddScoped<PrinterTelemetryService>();
builder.Services.AddScoped<ProjectService>();
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
    <!DOCTYPE html><html><body style="font-family:sans-serif;max-width:320px;margin:80px auto;">
    <h1>Spoolbook</h1>
    <form method="post" action="/login?returnUrl={Uri.EscapeDataString(returnUrl ?? "/")}">
        <input type="password" name="password" placeholder="Password" autofocus style="width:100%;padding:8px;" />
        <button type="submit" style="width:100%;padding:8px;margin-top:8px;">Sign in</button>
    </form>
    {(error is not null ? "<p style=\"color:red\">Wrong password.</p>" : "")}
    </body></html>
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
