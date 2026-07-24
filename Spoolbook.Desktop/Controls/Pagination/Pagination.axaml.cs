using System.Windows.Input;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Data;

namespace Spoolbook.Desktop.Controls;

public partial class Pagination : UserControl
{
    public static readonly StyledProperty<int> PageProperty =
        AvaloniaProperty.Register<Pagination, int>(nameof(Page));

    public static readonly StyledProperty<int> TotalPagesProperty =
        AvaloniaProperty.Register<Pagination, int>(nameof(TotalPages));

    public static readonly StyledProperty<int> PageSizeProperty =
        AvaloniaProperty.Register<Pagination, int>(nameof(PageSize), defaultBindingMode: BindingMode.TwoWay);

    public static readonly StyledProperty<ICommand?> PreviousCommandProperty =
        AvaloniaProperty.Register<Pagination, ICommand?>(nameof(PreviousCommand));

    public static readonly StyledProperty<ICommand?> NextCommandProperty =
        AvaloniaProperty.Register<Pagination, ICommand?>(nameof(NextCommand));

    public int Page
    {
        get => GetValue(PageProperty);
        set => SetValue(PageProperty, value);
    }

    public int TotalPages
    {
        get => GetValue(TotalPagesProperty);
        set => SetValue(TotalPagesProperty, value);
    }

    public int PageSize
    {
        get => GetValue(PageSizeProperty);
        set => SetValue(PageSizeProperty, value);
    }

    public ICommand? PreviousCommand
    {
        get => GetValue(PreviousCommandProperty);
        set => SetValue(PreviousCommandProperty, value);
    }

    public ICommand? NextCommand
    {
        get => GetValue(NextCommandProperty);
        set => SetValue(NextCommandProperty, value);
    }

    public Pagination()
    {
        InitializeComponent();
    }
}
