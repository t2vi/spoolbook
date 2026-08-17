// Port of JpegFrameExtractor.cs — splits a raw byte stream (ffmpeg's `-f mjpeg` stdout) into
// individual JPEG frames by scanning for the standard SOI/EOI markers (0xFFD8/0xFFD9). Stateful:
// frames commonly split across multiple stdout reads, so partial bytes are retained across
// feed() calls.
const START: [u8; 2] = [0xFF, 0xD8];
const END: [u8; 2] = [0xFF, 0xD9];

#[derive(Default)]
pub struct JpegFrameExtractor {
    buffer: Vec<u8>,
}

impl JpegFrameExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, chunk: &[u8]) -> Vec<Vec<u8>> {
        self.buffer.extend_from_slice(chunk);

        let mut frames = Vec::new();
        loop {
            let Some(start_idx) = find(&self.buffer, &START, 0) else {
                // No start marker at all — nothing usable is buffered except possibly a lone
                // leading 0xFF waiting for its 0xD8, so keep at most the last byte.
                if self.buffer.len() > 1 {
                    let keep_from = self.buffer.len() - 1;
                    self.buffer.drain(0..keep_from);
                }
                break;
            };

            if start_idx > 0 {
                self.buffer.drain(0..start_idx);
            }

            let Some(end_idx) = find(&self.buffer, &END, START.len()) else {
                break; // frame not complete yet — wait for more data
            };

            let frame_length = end_idx + END.len();
            frames.push(self.buffer[0..frame_length].to_vec());
            self.buffer.drain(0..frame_length);
        }

        frames
    }
}

fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if haystack.len() < needle.len() {
        return None;
    }
    (from..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}
