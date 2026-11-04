use anyhow::{Result, ensure};
const FRAME_LIMIT: usize = 4 * 1024 * 1024;

fn integer(data: &[u8]) -> Option<(u32, usize)> {
    let first = *data.first()?;
    match first {
        0..=127 => Some((u32::from(first), 1)),
        128..=191 => Some((u32::from(first & 63) + 64 * u32::from(*data.get(1)?), 2)),
        192..=223 => Some((
            u32::from(first & 31)
                + 32 * (u32::from(*data.get(1)?) + 256 * u32::from(*data.get(2)?)),
            3,
        )),
        224..=239 => Some((
            u32::from(first & 15)
                + 16 * (u32::from(*data.get(1)?)
                    + 256 * (u32::from(*data.get(2)?) + 256 * u32::from(*data.get(3)?))),
            4,
        )),
        _ => Some((u32::from_le_bytes(data.get(1..5)?.try_into().ok()?), 5)),
    }
}

#[derive(Default)]
pub(super) struct Reader {
    buffer: Vec<u8>,
    cursor: usize,
}
impl Reader {
    pub(super) fn push(&mut self, data: &[u8]) -> Result<()> {
        if self.cursor > 0 {
            self.buffer.drain(..self.cursor);
            self.cursor = 0;
        }
        ensure!(
            self.buffer.len().saturating_add(data.len()) <= FRAME_LIMIT + 10,
            "Слишком большой UMP-буфер"
        );
        self.buffer.extend_from_slice(data);
        Ok(())
    }
    pub(super) fn next(&mut self) -> Result<Option<(u32, Vec<u8>)>> {
        let data = &self.buffer[self.cursor..];
        let Some((kind, n)) = integer(data) else {
            return Ok(None);
        };
        let Some((size, m)) = integer(&data[n..]) else {
            return Ok(None);
        };
        ensure!(size as usize <= FRAME_LIMIT, "Слишком большой UMP-кадр");
        let end = n + m + size as usize;
        if data.len() < end {
            return Ok(None);
        }
        let output = (kind, data[n + m..end].to_vec());
        self.cursor += end;
        Ok(Some(output))
    }
    pub(super) fn finish(&self) -> Result<()> {
        ensure!(self.cursor == self.buffer.len(), "Оборванный UMP-кадр");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn integer_golden_boundaries() {
        for (encoded, value) in [
            (vec![0x7f], 127),
            (vec![0x80, 2], 128),
            (vec![0xbf, 0xff], 16383),
            (vec![0xc0, 0, 2], 16384),
            (vec![0xdf, 0xff, 0xff], 2097151),
            (vec![0xe0, 0, 0, 2], 2097152),
            (vec![0xef, 0xff, 0xff, 0xff], 268435455),
            (vec![0xf0, 0, 0, 0, 0x10], 268435456),
        ] {
            assert_eq!(integer(&encoded), Some((value, encoded.len())));
            for n in 0..encoded.len() {
                assert_eq!(integer(&encoded[..n]), None);
            }
        }
    }
    #[test]
    fn every_chunk_boundary_and_truncated_eof() {
        let wire = [20, 3, 1, 2, 3, 21, 2, 7, 8, 22, 1, 7];
        for cut in 0..=wire.len() {
            let mut r = Reader::default();
            let mut frames = vec![];
            for chunk in [&wire[..cut], &wire[cut..]] {
                r.push(chunk).unwrap();
                while let Some(part) = r.next().unwrap() {
                    frames.push(part);
                }
            }
            r.finish().unwrap();
            assert_eq!(
                frames,
                vec![(20, vec![1, 2, 3]), (21, vec![7, 8]), (22, vec![7])]
            );
        }
        for cut in [1, 2, 3, 4, 6, 7, 8, 10, 11] {
            let mut r = Reader::default();
            r.push(&wire[..cut]).unwrap();
            while r.next().unwrap().is_some() {}
            assert!(r.finish().is_err());
        }
    }
    #[test]
    fn oversized_frame_rejected_from_header() {
        let mut r = Reader::default();
        r.push(&[21, 0xf0, 0xff, 0xff, 0xff, 0x7f]).unwrap();
        assert!(r.next().is_err());
    }
}
