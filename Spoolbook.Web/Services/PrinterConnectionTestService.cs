using MQTTnet;

namespace Spoolbook.Web.Services;

public class PrinterConnectionTestResult
{
    public bool Ok { get; init; }
    public string? Error { get; init; }
}

// One-shot diagnostic: connects and disconnects immediately, no subscribe. A successful CONNACK
// already proves both the IP is reachable and the access code is correct — Bambu's broker
// rejects auth at connect time — so there's nothing a subscribe would add here. Deliberately not
// a live status view (that was explicitly ruled out earlier — see PrinterMqttHostedService for
// the actual telemetry-buffering client); this only ever runs once, on click.
public class PrinterConnectionTestService
{
    public async Task<PrinterConnectionTestResult> TestAsync(string ipAddress, string accessCode)
    {
        var factory = new MqttClientFactory();
        using var client = factory.CreateMqttClient();

        var options = new MqttClientOptionsBuilder()
            .WithTcpServer(ipAddress, 8883)
            .WithCredentials("bblp", accessCode)
            .WithTlsOptions(o => o
                .UseTls()
                .WithCertificateValidationHandler(_ => true))
            .WithClientId($"spoolbook-test-{Guid.NewGuid():N}")
            .Build();

        using var cts = new CancellationTokenSource(TimeSpan.FromSeconds(6));
        try
        {
            var result = await client.ConnectAsync(options, cts.Token);
            if (client.IsConnected)
                await client.DisconnectAsync(cancellationToken: CancellationToken.None);

            return result.ResultCode == MqttClientConnectResultCode.Success
                ? new PrinterConnectionTestResult { Ok = true }
                : new PrinterConnectionTestResult { Ok = false, Error = result.ResultCode.ToString() };
        }
        catch (OperationCanceledException)
        {
            return new PrinterConnectionTestResult { Ok = false, Error = "Timed out — check the IP address and that LAN mode is enabled on the printer." };
        }
        catch (Exception ex)
        {
            return new PrinterConnectionTestResult { Ok = false, Error = ex.Message };
        }
    }
}
