using System.Text.Json;
using MQTTnet;

namespace Spoolbook.Web.Services;

public class PrinterControlResult
{
    public bool Ok { get; init; }
    public string? Error { get; init; }
}

// Publishes Bambu's print-control commands on the same live connection PrinterMqttHostedService
// already holds open for telemetry (docs/adr/0022) rather than opening a new one per action —
// so a command fails outright (rather than queuing) if that connection happens to be mid-reconnect.
public class PrinterControlService
{
    private readonly PrinterLiveStatusStore _liveStatusStore;

    public PrinterControlService(PrinterLiveStatusStore liveStatusStore)
    {
        _liveStatusStore = liveStatusStore;
    }

    public Task<PrinterControlResult> PauseAsync(int printerId, string serialNumber) =>
        SendCommandAsync(printerId, serialNumber, "pause");

    public Task<PrinterControlResult> ResumeAsync(int printerId, string serialNumber) =>
        SendCommandAsync(printerId, serialNumber, "resume");

    public Task<PrinterControlResult> StopAsync(int printerId, string serialNumber) =>
        SendCommandAsync(printerId, serialNumber, "stop");

    private async Task<PrinterControlResult> SendCommandAsync(int printerId, string serialNumber, string command)
    {
        var client = _liveStatusStore.GetConnectedClient(printerId);
        if (client is null)
            return new PrinterControlResult { Ok = false, Error = "Printer isn't connected — telemetry link is down or still reconnecting." };

        var payload = JsonSerializer.Serialize(new
        {
            print = new { command, sequence_id = Guid.NewGuid().ToString("N") }
        });
        var message = new MqttApplicationMessageBuilder()
            .WithTopic($"device/{serialNumber}/request")
            .WithPayload(payload)
            .Build();

        try
        {
            await client.PublishAsync(message);
            return new PrinterControlResult { Ok = true };
        }
        catch (Exception ex)
        {
            return new PrinterControlResult { Ok = false, Error = ex.Message };
        }
    }
}
