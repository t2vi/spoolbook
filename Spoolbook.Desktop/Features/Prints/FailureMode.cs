namespace Spoolbook.Desktop.Features.Prints;

// Fixed vocabulary, not free text — see docs/adr/0019-failure-mode-fixed-vocabulary.md.
public enum FailureMode { Stringing, LayerAdhesion, Warping, UnderExtrusion, OverExtrusion, LayerShift, Clog, Other }

public class PrintFailureMode
{
    public int Id { get; set; }
    public int PrintId { get; set; }
    public Print? Print { get; set; }
    public FailureMode Mode { get; set; }
}
