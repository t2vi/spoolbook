using Avalonia.Controls;
using Avalonia.Input;
namespace Spoolbook.Desktop.Common;

// Wires ESC on a modal edit window: closes immediately if there's nothing unsaved,
// otherwise prompts Save/Discard/Keep editing rather than silently dropping edits.
public static class EditWindowEscape
{
    public static void Attach(Window window, Func<bool> isDirty, Func<Task> saveAsync, Action cancel)
    {
        window.KeyDown += async (_, e) =>
        {
            if (e.Key != Key.Escape) return;
            e.Handled = true;

            if (!isDirty())
            {
                cancel();
                return;
            }

            var choice = await new UnsavedChangesWindow().ShowDialog<UnsavedChangesChoice>(window);
            switch (choice)
            {
                case UnsavedChangesChoice.Save:
                    await saveAsync();
                    break;
                case UnsavedChangesChoice.Discard:
                    cancel();
                    break;
                case UnsavedChangesChoice.KeepEditing:
                    break;
            }
        };
    }
}
