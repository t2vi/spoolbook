using Spoolbook.Desktop.Services.Printer;

namespace Spoolbook.Desktop.Tests;

public class JpegFrameExtractorTests
{
    private static byte[] Jpeg(params byte[] body) => [0xFF, 0xD8, .. body, 0xFF, 0xD9];

    [Fact]
    public void SingleCompleteFrameInOneChunk_YieldsThatFrame()
    {
        var extractor = new JpegFrameExtractor();
        var frame = Jpeg(1, 2, 3);

        var result = extractor.Feed(frame);

        Assert.Single(result);
        Assert.Equal(frame, result[0]);
    }

    [Fact]
    public void FrameSplitAcrossTwoChunks_YieldsNothingThenTheFrame()
    {
        var extractor = new JpegFrameExtractor();
        var frame = Jpeg(1, 2, 3, 4, 5);

        var first = extractor.Feed(frame[..4]);
        Assert.Empty(first);

        var second = extractor.Feed(frame[4..]);
        Assert.Single(second);
        Assert.Equal(frame, second[0]);
    }

    [Fact]
    public void TwoFramesInOneChunk_YieldsBothInOrder()
    {
        var extractor = new JpegFrameExtractor();
        var frameA = Jpeg(1, 2);
        var frameB = Jpeg(9, 9, 9);

        var result = extractor.Feed([.. frameA, .. frameB]);

        Assert.Equal(2, result.Count);
        Assert.Equal(frameA, result[0]);
        Assert.Equal(frameB, result[1]);
    }

    [Fact]
    public void GarbageBeforeStartMarker_IsDiscarded()
    {
        var extractor = new JpegFrameExtractor();
        var frame = Jpeg(7, 7);
        byte[] noise = [0x00, 0x11, 0x22];

        var result = extractor.Feed([.. noise, .. frame]);

        Assert.Single(result);
        Assert.Equal(frame, result[0]);
    }

    [Fact]
    public void NoEndMarkerYet_YieldsNothingAndRetainsData()
    {
        var extractor = new JpegFrameExtractor();
        byte[] partial = [0xFF, 0xD8, 1, 2, 3];

        var result = extractor.Feed(partial);
        Assert.Empty(result);

        // Completing it later should still produce the full frame, proving the
        // partial bytes weren't dropped.
        var completed = extractor.Feed([0xFF, 0xD9]);
        Assert.Single(completed);
        Assert.Equal(partial.Concat<byte>([0xFF, 0xD9]), completed[0]);
    }
}
