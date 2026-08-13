using System.Collections.Concurrent;
using MQTTnet;
using Spoolbook.Desktop.Services.Printer;

namespace Spoolbook.Web.Services;

public class PrinterLiveStatus
{
    public IMqttClient? Client { get; set; }
    public List<AmsUnitReading> AmsUnits { get; set; } = [];
}

// Per-printer registry of the live MQTT connection PrinterMqttHostedService already holds open
// for telemetry, plus the latest AMS snapshot — reused by control actions (pause/resume/stop)
// instead of opening a second connection per command, and by the UI for AMS display.
// docs/adr/0022.
public class PrinterLiveStatusStore
{
    private readonly ConcurrentDictionary<int, PrinterLiveStatus> _byPrinterId = new();

    private PrinterLiveStatus GetOrAdd(int printerId) =>
        _byPrinterId.GetOrAdd(printerId, _ => new PrinterLiveStatus());

    public void SetClient(int printerId, IMqttClient? client) => GetOrAdd(printerId).Client = client;

    public void SetAmsUnits(int printerId, List<AmsUnitReading> amsUnits) => GetOrAdd(printerId).AmsUnits = amsUnits;

    public IMqttClient? GetConnectedClient(int printerId) =>
        _byPrinterId.TryGetValue(printerId, out var status) && status.Client?.IsConnected == true ? status.Client : null;

    public List<AmsUnitReading> GetAmsUnits(int printerId) =>
        _byPrinterId.TryGetValue(printerId, out var status) ? status.AmsUnits : [];
}
