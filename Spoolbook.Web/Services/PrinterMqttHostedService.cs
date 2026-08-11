using System.Text;
using MQTTnet;
using Spoolbook.Desktop.Features.Prints;
using Spoolbook.Desktop.Features.Settings.Printers;
using Spoolbook.Desktop.Services.Printer;

namespace Spoolbook.Web.Services;

// Auto-connects at launch to every configured Printer's local MQTT broker (docs/adr/0017) and
// buffers live telemetry via PrinterTelemetryService. Read-only: subscribes to device/{serial}/
// report only, never publishes to a printer's request/control topic.
public class PrinterMqttHostedService : BackgroundService
{
    private readonly IServiceScopeFactory _scopeFactory;
    private readonly ILogger<PrinterMqttHostedService> _logger;
    private readonly Dictionary<int, string?> _activeTaskIdByPrinter = new();

    public PrinterMqttHostedService(IServiceScopeFactory scopeFactory, ILogger<PrinterMqttHostedService> logger)
    {
        _scopeFactory = scopeFactory;
        _logger = logger;
    }

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        List<Printer> printers;
        using (var scope = _scopeFactory.CreateScope())
        {
            var printerService = scope.ServiceProvider.GetRequiredService<PrinterService>();
            printers = (await printerService.ListAsync())
                .Where(p => !string.IsNullOrWhiteSpace(p.IpAddress)
                    && !string.IsNullOrWhiteSpace(p.AccessCode)
                    && !string.IsNullOrWhiteSpace(p.SerialNumber))
                .ToList();
        }

        var connectTasks = printers.Select(p => ConnectAndSubscribeAsync(p, stoppingToken));
        await Task.WhenAll(connectTasks);
    }

    private async Task ConnectAndSubscribeAsync(Printer printer, CancellationToken stoppingToken)
    {
        var factory = new MqttClientFactory();
        using var client = factory.CreateMqttClient();

        var options = new MqttClientOptionsBuilder()
            .WithTcpServer(printer.IpAddress, 8883)
            .WithCredentials("bblp", printer.AccessCode)
            .WithTlsOptions(o => o
                .UseTls()
                // Bambu's LAN broker presents a self-signed cert — this is a local-network-only
                // credential exchange (ADR-0017), not a certificate-authenticated one.
                .WithCertificateValidationHandler(_ => true))
            .WithClientId($"spoolbook-{Guid.NewGuid():N}")
            .Build();

        client.ApplicationMessageReceivedAsync += async e =>
        {
            var payload = Encoding.UTF8.GetString(e.ApplicationMessage.Payload);
            await HandleMessageAsync(printer.Id, payload);
        };

        while (!stoppingToken.IsCancellationRequested)
        {
            try
            {
                if (!client.IsConnected)
                {
                    await client.ConnectAsync(options, stoppingToken);
                    await client.SubscribeAsync(
                        new MqttClientSubscribeOptionsBuilder()
                            .WithTopicFilter($"device/{printer.SerialNumber}/report")
                            .Build(),
                        stoppingToken);
                    _logger.LogInformation("Connected to printer {Name} telemetry", printer.Name);
                }
                await Task.Delay(TimeSpan.FromSeconds(10), stoppingToken);
            }
            catch (OperationCanceledException)
            {
                break;
            }
            catch (Exception ex)
            {
                _logger.LogWarning(ex, "Printer {Name} MQTT connection lost, retrying", printer.Name);
                await Task.Delay(TimeSpan.FromSeconds(15), stoppingToken);
            }
        }

        if (client.IsConnected)
            await client.DisconnectAsync(cancellationToken: CancellationToken.None);
    }

    private async Task HandleMessageAsync(int printerId, string payload)
    {
        var message = BambuMqttPayloadParser.Parse(payload);
        if (message is null) return;

        using var scope = _scopeFactory.CreateScope();
        var telemetryService = scope.ServiceProvider.GetRequiredService<PrinterTelemetryService>();

        if (BambuMqttPayloadParser.IsActiveState(message.GcodeState) && message.TaskId is not null)
        {
            _activeTaskIdByPrinter[printerId] = message.TaskId;
            await telemetryService.RecordReadingAsync(printerId, message.TaskId, message.Reading);
        }
        else if (_activeTaskIdByPrinter.TryGetValue(printerId, out var activeTaskId) && activeTaskId is not null)
        {
            await telemetryService.EndJobAsync(printerId, activeTaskId);
            _activeTaskIdByPrinter[printerId] = null;
        }
    }
}
