using Avalonia;
using Avalonia.Controls.Primitives;
using Avalonia.Data;

namespace Spoolbook.Desktop.Controls;

public class TextBoxWithUnit : TemplatedControl
{
    public static readonly StyledProperty<string?> TextProperty =
        AvaloniaProperty.Register<TextBoxWithUnit, string?>(nameof(Text), defaultBindingMode: BindingMode.TwoWay);

    public static readonly StyledProperty<string?> UnitProperty =
        AvaloniaProperty.Register<TextBoxWithUnit, string?>(nameof(Unit));

    public static readonly StyledProperty<bool> IsNumericProperty =
        AvaloniaProperty.Register<TextBoxWithUnit, bool>(nameof(IsNumeric));

    // Backs the NumericUpDown template part when IsNumeric — kept in sync with Text (the single
    // external binding point, e.g. ProfileFieldEntry.Value) rather than exposing a second bindable
    // property callers need to wire up.
    public static readonly StyledProperty<decimal?> NumericValueProperty =
        AvaloniaProperty.Register<TextBoxWithUnit, decimal?>(nameof(NumericValue), defaultBindingMode: BindingMode.TwoWay);

    public string? Text
    {
        get => GetValue(TextProperty);
        set => SetValue(TextProperty, value);
    }

    public string? Unit
    {
        get => GetValue(UnitProperty);
        set => SetValue(UnitProperty, value);
    }

    public bool IsNumeric
    {
        get => GetValue(IsNumericProperty);
        set => SetValue(IsNumericProperty, value);
    }

    public decimal? NumericValue
    {
        get => GetValue(NumericValueProperty);
        set => SetValue(NumericValueProperty, value);
    }

    private bool _syncing;

    static TextBoxWithUnit()
    {
        TextProperty.Changed.AddClassHandler<TextBoxWithUnit>((c, _) => c.SyncNumericFromText());
        NumericValueProperty.Changed.AddClassHandler<TextBoxWithUnit>((c, _) => c.SyncTextFromNumeric());
    }

    private void SyncNumericFromText()
    {
        if (_syncing) return;
        _syncing = true;
        NumericValue = decimal.TryParse(Text, out var d) ? d : null;
        _syncing = false;
    }

    private void SyncTextFromNumeric()
    {
        if (_syncing) return;
        _syncing = true;
        Text = NumericValue?.ToString() ?? "";
        _syncing = false;
    }
}
