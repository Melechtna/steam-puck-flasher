//! HDLC framing, the layer under every bootloader message.

/// Framing bytes.
const SOF: u8 = 0xAD;
const EOF: u8 = 0xAE;
const ESC: u8 = 0xAC;

/// Wrap a payload in HDLC SOF/EOF, escaping the three reserved bytes.
pub fn encode(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() + 8);
    out.push(SOF);
    for &b in input {
        match b {
            SOF => out.extend_from_slice(&[ESC, 0x01]),
            EOF => out.extend_from_slice(&[ESC, 0x02]),
            ESC => out.extend_from_slice(&[ESC, 0x00]),
            _ => out.push(b),
        }
    }
    out.push(EOF);
    out
}

/// Incremental HDLC decoder.
#[derive(Default)]
pub struct Decoder {
    buf: Vec<u8>,
    escaped: bool,
}

impl Decoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, data: &[u8]) -> Vec<Vec<u8>> {
        let mut frames = Vec::new();
        for &b in data {
            if self.escaped {
                match b {
                    0x00 => self.buf.push(ESC),
                    0x01 => self.buf.push(SOF),
                    0x02 => self.buf.push(EOF),
                    _ => self.buf.clear(),
                }
                self.escaped = false;
            } else if b == ESC {
                self.escaped = true;
            } else if b == SOF {
                self.buf.clear();
            } else if b == EOF {
                if !self.buf.is_empty() {
                    frames.push(std::mem::take(&mut self.buf));
                } else {
                    self.buf.clear();
                }
            } else {
                self.buf.push(b);
            }
        }
        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_payload_round_trips() {
        let msg = b"hello world";
        let wire = encode(msg);
        assert_eq!(wire.first(), Some(&SOF));
        assert_eq!(wire.last(), Some(&EOF));

        let mut d = Decoder::new();
        let frames = d.push(&wire);
        assert_eq!(frames, vec![msg.to_vec()]);
    }

    #[test]
    fn reserved_bytes_are_escaped_and_restored() {
        let msg = [SOF, EOF, ESC, 0x00, 0xFF];
        let wire = encode(&msg);

        let mut d = Decoder::new();
        assert_eq!(d.push(&wire), vec![msg.to_vec()]);
    }

    #[test]
    fn a_frame_split_across_reads_is_reassembled() {
        let wire = encode(b"split me");
        let mut d = Decoder::new();

        assert!(d.push(&wire[..3]).is_empty(), "nothing completes yet");
        assert!(d.push(&wire[3..7]).is_empty(), "still incomplete");

        let frames = d.push(&wire[7..]);
        assert_eq!(frames, vec![b"split me".to_vec()]);
    }

    #[test]
    fn two_frames_in_one_read_both_come_out() {
        let mut wire = encode(b"first");
        wire.extend(encode(b"second"));

        let mut d = Decoder::new();
        let frames = d.push(&wire);
        assert_eq!(frames, vec![b"first".to_vec(), b"second".to_vec()]);
    }

    #[test]
    fn noise_before_a_frame_is_discarded() {
        let mut wire = vec![0x11, 0x22, 0x33];
        wire.extend(encode(b"real"));

        let mut d = Decoder::new();
        assert_eq!(d.push(&wire), vec![b"real".to_vec()]);
    }

    #[test]
    fn a_bogus_escape_does_not_yield_a_frame() {
        let wire = [SOF, 0x41, ESC, 0x7f, EOF];

        let mut d = Decoder::new();
        assert!(d.push(&wire).is_empty());
    }
}
