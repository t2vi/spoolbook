using CommunityToolkit.Mvvm.ComponentModel;
namespace Spoolbook.Desktop.Common;

// Shared dirty-tracking for the app's modal edit windows — lets ESC (or any other
// close path) know whether to prompt Save/Discard/Keep editing instead of just closing.
public abstract partial class EditViewModelBase : ViewModelBase
{
    [ObservableProperty]
    private bool isDirty;

    // Guards against constructor-time population (and async option-list loads) marking
    // the form dirty before the user has actually touched anything.
    protected bool Loaded { get; set; }

    protected void MarkDirty()
    {
        if (Loaded) IsDirty = true;
    }
}
