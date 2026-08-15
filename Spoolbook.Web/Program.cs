using System.Security.Claims;
using System.Text;
using System.Text.Json.Serialization;
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
using Microsoft.Extensions.FileProviders;
using Spoolbook.Web.Api;
using Spoolbook.Web.Services;

var builder = WebApplication.CreateBuilder(args);

// Same path the Avalonia desktop app used to write to, back when Spoolbook.Web was still a
// second, web-hosted front end sharing that app's local SQLite file — kept as-is now that
// Spoolbook.Desktop's UI is gone too, so existing installs don't need a data migration.
// SPOOLBOOK_DB_PATH overrides this for a dev instance run alongside the live one — otherwise a
// second `dotnet run` double-connects to real printer MQTT and double-writes telemetry into the
// same production DB the live instance uses. Unset in production; only ever set for local dev.
var dbPath = Environment.GetEnvironmentVariable("SPOOLBOOK_DB_PATH");
if (string.IsNullOrEmpty(dbPath))
{
    var dataDir = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
        "spoolbook");
    Directory.CreateDirectory(dataDir);
    dbPath = Path.Combine(dataDir, "spoolbook.db");
}

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
builder.Services.AddHttpClient<ProjectUploadService>();
builder.Services.AddSingleton<PrinterLiveStatusStore>();
builder.Services.AddHostedService<PrinterMqttHostedService>();
builder.Services.AddScoped<PrinterConnectionTestService>();
builder.Services.AddScoped<PrinterControlService>();
builder.Services.AddScoped<PrinterPrintService>();
builder.Services.AddSingleton<PrinterCameraService>();
// Standalone OrcaSlicer wrapper (slicer-service/) — separate deployable, not a project in this
// solution. Defaults to localhost for local dev against a co-located instance; point
// RESLICE_SERVICE_URL at the real LXC's address in production.
builder.Services.AddHttpClient<ReslicingService>(client =>
    client.BaseAddress = new Uri(Environment.GetEnvironmentVariable("RESLICE_SERVICE_URL") ?? "http://localhost:8100"));
builder.Services.AddScoped<AppSettingsService>();
builder.Services.AddScoped<DashboardMetricsService>();
builder.Services.AddScoped<BambuFilamentImportService>(_ => new BambuFilamentImportService(
    new BambuPresetResolver(BambuPaths.FindUserFilamentPresetsDir() ?? "", BambuPaths.FindSystemProfilesDir() ?? "")));
builder.Services.AddScoped<IWeatherService, OpenMeteoWeatherService>();

// Single shared-secret login gating mutating pages — reactivates the v2 model from
// docs/adr/0005-access-control-v1-vercel-gate-v2-mutation-lock.md for the LAN pivot
// (docs/adr/0018). Still single-editor: no user table, no OAuth.
builder.Services.AddAuthentication(CookieAuthenticationDefaults.AuthenticationScheme)
    .AddCookie(options =>
    {
        options.LoginPath = "/login";
        // The JSON API (/api/**) wants a clean 401, not the cookie middleware's default
        // redirect-to-/login response — not something a fetch()-based client should have to
        // special-case. Non-API auth-gated routes (e.g. the camera stream) keep the redirect.
        options.Events.OnRedirectToLogin = ctx =>
        {
            if (ctx.Request.Path.StartsWithSegments("/api"))
            {
                ctx.Response.StatusCode = StatusCodes.Status401Unauthorized;
                return ctx.Response.WriteAsJsonAsync(new { authenticated = false });
            }
            ctx.Response.Redirect(ctx.RedirectUri);
            return Task.CompletedTask;
        };
    });
builder.Services.AddAuthorization();

// SvelteKit migration's JSON API is starting to return entities directly (no DTO layer, per
// the migration plan) — Print.FailureModes <-> PrintFailureMode.Print is a genuine reference
// cycle in the entity graph, not a hypothetical one. Enums (PrintStatus, AmbientSource, ...)
// serialize as their string name, not the default numeric value — PrinterCard needs to show
// "Success"/"Failed"/etc., not a bare 0-3 the frontend would have to keep a private mapping for.
builder.Services.ConfigureHttpJsonOptions(options =>
{
    options.SerializerOptions.ReferenceHandler = ReferenceHandler.IgnoreCycles;
    options.SerializerOptions.Converters.Add(new JsonStringEnumConverter());
});

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
    // The default HSTS value is 30 days. You may want to change this for production scenarios, see https://aka.ms/aspnetcore-hsts.
    app.UseHsts();
}
app.UseHttpsRedirection();

// Svelte's static build (spoolbook-web-svelte's adapter-static output) — a sibling checkout
// directory, not copied into wwwroot, so a rebuild of the frontend doesn't require rebuilding
// Spoolbook.Web. SPOOLBOOK_STATIC_ROOT overrides this for Docker/LXC packaging later, where the
// build output won't sit next to the .NET project on disk the way it does in a plain repo pull.
var staticRoot = Environment.GetEnvironmentVariable("SPOOLBOOK_STATIC_ROOT")
    ?? Path.Combine(app.Environment.ContentRootPath, "..", "spoolbook-web-svelte", "build");
var svelteFileProvider = new PhysicalFileProvider(staticRoot);
app.UseStaticFiles(); // wwwroot — still serves /css/tailwind.css for the plain-HTML /login page below
app.UseStaticFiles(new StaticFileOptions { FileProvider = svelteFileProvider });

app.UseAuthentication();
app.UseAuthorization();

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

// Shared by /login's form POST and the SvelteKit-facing /api/login below — same shared-secret
// check, same cookie, two different response shapes (redirect vs JSON) for two different callers.
async Task<bool> TrySignInEditorAsync(HttpContext ctx, string password)
{
    var expected = Environment.GetEnvironmentVariable("SPOOLBOOK_ADMIN_PASSWORD");
    if (string.IsNullOrEmpty(expected) || password != expected) return false;

    var identity = new ClaimsIdentity([new Claim(ClaimTypes.Name, "editor")], CookieAuthenticationDefaults.AuthenticationScheme);
    await ctx.SignInAsync(CookieAuthenticationDefaults.AuthenticationScheme, new ClaimsPrincipal(identity));
    return true;
}

app.MapPost("/login", async (HttpContext ctx, string? returnUrl) =>
{
    var form = await ctx.Request.ReadFormAsync();
    var password = form["password"].ToString();

    if (!await TrySignInEditorAsync(ctx, password))
        return Results.Redirect($"/login?returnUrl={Uri.EscapeDataString(returnUrl ?? "/")}&error=1");

    return Results.Redirect(returnUrl ?? "/");
});

app.MapPost("/logout", async (HttpContext ctx) =>
{
    await ctx.SignOutAsync(CookieAuthenticationDefaults.AuthenticationScheme);
    return Results.Redirect("/");
});

// SvelteKit-facing counterparts of the two endpoints above — same cookie, JSON in/out instead
// of a form post + redirect, so the Svelte app can render its own login form.
app.MapPost("/api/login", async (HttpContext ctx, ApiLoginRequest req) =>
    await TrySignInEditorAsync(ctx, req.Password) ? Results.Ok(new { ok = true }) : Results.Json(new { ok = false }, statusCode: 401));

app.MapPost("/api/logout", async (HttpContext ctx) =>
{
    await ctx.SignOutAsync(CookieAuthenticationDefaults.AuthenticationScheme);
    return Results.Ok(new { ok = true });
});

app.MapGet("/api/me", (HttpContext ctx) => Results.Ok(new { authenticated = ctx.User.Identity?.IsAuthenticated == true }));

app.MapPrinterEndpoints();
app.MapProjectEndpoints();
app.MapPrintEndpoints();
app.MapSpoolEndpoints();
app.MapProfileEndpoints();
app.MapDashboardEndpoints();
app.MapSettingsEndpoints();
app.MapFilamentEndpoints();

// MJPEG relay for a printer's live camera (docs/adr/0024) — plain minimal API so a browser
// <img> tag can point straight at it; a Blazor component can't stream a raw multipart
// response the way an ordinary endpoint can. Gated behind the shared secret (carve-out from
// the usual "reads are open" policy — a live camera feed is a view into physical space, not
// just app data).
app.MapGet("/printers/{id:int}/camera", async (int id, HttpContext ctx, PrinterService printerService, PrinterCameraService cameraService) =>
{
    var printer = (await printerService.ListAsync()).FirstOrDefault(p => p.Id == id);
    if (printer?.IpAddress is null || printer.AccessCode is null)
    {
        ctx.Response.StatusCode = StatusCodes.Status404NotFound;
        return;
    }

    ctx.Response.ContentType = "multipart/x-mixed-replace; boundary=frame";
    ctx.Response.Headers.CacheControl = "no-cache";

    try
    {
        await foreach (var frame in cameraService.SubscribeAsync(id, printer.IpAddress, printer.AccessCode, ctx.RequestAborted))
        {
            var header = Encoding.ASCII.GetBytes($"--frame\r\nContent-Type: image/jpeg\r\nContent-Length: {frame.Length}\r\n\r\n");
            await ctx.Response.Body.WriteAsync(header, ctx.RequestAborted);
            await ctx.Response.Body.WriteAsync(frame, ctx.RequestAborted);
            await ctx.Response.Body.WriteAsync("\r\n"u8.ToArray(), ctx.RequestAborted);
            await ctx.Response.Body.FlushAsync(ctx.RequestAborted);
        }
    }
    catch (OperationCanceledException)
    {
        // Client navigated away / closed the tab — expected teardown, not an error.
    }
}).RequireAuthorization();

// SPA fallback — anything not matched by an API/login/camera route above (i.e. every Svelte
// client-side route: /prints/edit/5, /settings, ...) serves the same index.html and lets
// SvelteKit's own client-side router take over.
app.MapFallbackToFile("index.html", new StaticFileOptions { FileProvider = svelteFileProvider });

app.Run();

public record ApiLoginRequest(string Password);
