namespace Spoolbook.Desktop.Services.Printer;

// Splits a raw byte stream (ffmpeg's `-f mjpeg` stdout) into individual JPEG frames by
// scanning for the standard SOI/EOI markers (0xFFD8/0xFFD9) — mirrors bambuddy's inline
// buffer-scanning loop for the same ffmpeg output shape. Stateful: frames commonly split
// across multiple stdout reads, so partial bytes are retained across Feed() calls.
public class JpegFrameExtractor
{
    private static readonly byte[] Start = [0xFF, 0xD8];
    private static readonly byte[] End = [0xFF, 0xD9];

    private readonly List<byte> _buffer = [];

    public List<byte[]> Feed(ReadOnlySpan<byte> chunk)
    {
        _buffer.AddRange(chunk.ToArray());

        var frames = new List<byte[]>();
        while (true)
        {
            var startIdx = IndexOf(_buffer, Start, 0);
            if (startIdx == -1)
            {
                // No start marker at all — nothing usable is buffered except possibly a
                // lone leading 0xFF waiting for its 0xD8, so keep at most the last byte.
                if (_buffer.Count > 1)
                    _buffer.RemoveRange(0, _buffer.Count - 1);
                break;
            }

            if (startIdx > 0)
                _buffer.RemoveRange(0, startIdx);

            var endIdx = IndexOf(_buffer, End, Start.Length);
            if (endIdx == -1)
                break; // frame not complete yet — wait for more data

            var frameLength = endIdx + End.Length;
            frames.Add(_buffer.GetRange(0, frameLength).ToArray());
            _buffer.RemoveRange(0, frameLength);
        }

        return frames;
    }

    private static int IndexOf(List<byte> haystack, byte[] needle, int from)
    {
        for (var i = from; i <= haystack.Count - needle.Length; i++)
        {
            var match = true;
            for (var j = 0; j < needle.Length; j++)
            {
                if (haystack[i + j] != needle[j]) { match = false; break; }
            }
            if (match) return i;
        }
        return -1;
    }
}
