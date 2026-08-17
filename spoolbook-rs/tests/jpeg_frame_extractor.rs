use spoolbook_rs::jpeg_frame_extractor::JpegFrameExtractor;

fn jpeg(body: &[u8]) -> Vec<u8> {
    let mut frame = vec![0xFF, 0xD8];
    frame.extend_from_slice(body);
    frame.extend_from_slice(&[0xFF, 0xD9]);
    frame
}

#[test]
fn single_complete_frame_in_one_chunk_yields_that_frame() {
    let mut extractor = JpegFrameExtractor::new();
    let frame = jpeg(&[1, 2, 3]);

    let result = extractor.feed(&frame);

    assert_eq!(result, vec![frame]);
}

#[test]
fn frame_split_across_two_chunks_yields_nothing_then_the_frame() {
    let mut extractor = JpegFrameExtractor::new();
    let frame = jpeg(&[1, 2, 3, 4, 5]);

    let first = extractor.feed(&frame[..4]);
    assert!(first.is_empty());

    let second = extractor.feed(&frame[4..]);
    assert_eq!(second, vec![frame]);
}

#[test]
fn two_frames_in_one_chunk_yields_both_in_order() {
    let mut extractor = JpegFrameExtractor::new();
    let frame_a = jpeg(&[1, 2]);
    let frame_b = jpeg(&[9, 9, 9]);

    let mut chunk = frame_a.clone();
    chunk.extend_from_slice(&frame_b);
    let result = extractor.feed(&chunk);

    assert_eq!(result, vec![frame_a, frame_b]);
}

#[test]
fn garbage_before_start_marker_is_discarded() {
    let mut extractor = JpegFrameExtractor::new();
    let frame = jpeg(&[7, 7]);
    let mut chunk = vec![0x00, 0x11, 0x22];
    chunk.extend_from_slice(&frame);

    let result = extractor.feed(&chunk);

    assert_eq!(result, vec![frame]);
}

#[test]
fn no_end_marker_yet_yields_nothing_and_retains_data() {
    let mut extractor = JpegFrameExtractor::new();
    let partial = [0xFF, 0xD8, 1, 2, 3];

    let result = extractor.feed(&partial);
    assert!(result.is_empty());

    let completed = extractor.feed(&[0xFF, 0xD9]);
    let mut expected = partial.to_vec();
    expected.extend_from_slice(&[0xFF, 0xD9]);
    assert_eq!(completed, vec![expected]);
}
