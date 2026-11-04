use super::proto::{FormatId, FormatInit, MediaHeader, TimeRange};
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(super) const SEGMENT_LIMIT: u64 = 256 * 1024 * 1024;
const SEGMENT_COUNT_LIMIT: u32 = 100_000;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Segment {
    pub id: u8,
    pub sequence: Option<u32>,
    pub start: i64,
    pub duration: i64,
    pub length: u64,
    pub digest: [u8; 32],
    pub duplicate: bool,
}
struct Pending {
    segment: Segment,
    expected: Option<u64>,
    hasher: Sha256,
}
pub(super) struct Track {
    pub format: FormatId,
    pub video_id: String,
    pub expected_bytes: u64,
    pub expected_duration: i64,
    pub bytes: u64,
    pub last_sequence: u32,
    pub end_ms: i64,
    init: Option<Segment>,
    committed: BTreeMap<u32, Segment>,
    pending: Option<Pending>,
    terminal: Option<(u32, i64)>,
}
impl Track {
    pub fn new(format: FormatId, video_id: String, bytes: u64, duration: i64) -> Result<Self> {
        ensure!(
            bytes > 0 && duration > 0,
            "YouTube не указал длину медиапотока"
        );
        Ok(Self {
            format,
            video_id,
            expected_bytes: bytes,
            expected_duration: duration,
            bytes: 0,
            last_sequence: 0,
            end_ms: 0,
            init: None,
            committed: BTreeMap::new(),
            pending: None,
            terminal: None,
        })
    }
    pub fn initialized(&self) -> bool {
        self.init.is_some()
    }
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn rollback(&mut self) {
        self.pending = None;
    }
    fn identity(&self, id: &FormatId) -> Result<()> {
        ensure!(
            id.itag == self.format.itag,
            "YouTube вернул другой формат SABR: ожидался {:?}, получен {:?}",
            self.format.itag,
            id.itag
        );
        if let Some(lmt) = id.last_modified {
            ensure!(
                Some(lmt) == self.format.last_modified,
                "Версия SABR-потока изменилась"
            );
        }
        if let Some(tags) = &id.xtags {
            ensure!(
                Some(tags) == self.format.xtags.as_ref(),
                "Аудиодорожка SABR изменилась"
            );
        }
        Ok(())
    }
    fn video(&self, id: Option<&str>) -> Result<()> {
        if let Some(id) = id {
            ensure!(id == self.video_id, "SABR вернул другое видео");
        }
        Ok(())
    }
    pub fn initialize(&mut self, meta: FormatInit) -> Result<()> {
        self.video(meta.video_id.as_deref())?;
        self.identity(
            meta.format_id
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Нет идентификатора SABR"))?,
        )?;
        let last = u32::try_from(meta.end_segment_number.unwrap_or(-1))
            .map_err(|_| anyhow::anyhow!("Нет конечного сегмента SABR"))?;
        let end = meta.end_time_ms.unwrap_or(-1);
        ensure!(
            (1..=SEGMENT_COUNT_LIMIT).contains(&last) && end > 0,
            "Неверная длина SABR"
        );
        ensure!(
            end.abs_diff(self.expected_duration) <= 3000u64.max(self.expected_duration as u64 / 50),
            "Длительность SABR не совпадает с метаданными"
        );
        if let Some(old) = self.terminal {
            ensure!(old == (last, end), "Конец SABR-потока изменился");
        }
        self.terminal = Some((last, end));
        Ok(())
    }
    pub fn header(&mut self, h: MediaHeader) -> Result<()> {
        ensure!(
            self.pending.is_none(),
            "SABR начал сегмент до завершения предыдущего"
        );
        self.video(h.video_id.as_deref())?;
        if let Some(id) = &h.format_id {
            self.identity(id)?;
        }
        let itag = h
            .itag
            .or_else(|| h.format_id.as_ref().and_then(|id| id.itag));
        ensure!(itag == self.format.itag, "Другой SABR-поток");
        if let Some(lmt) = h.lmt {
            ensure!(
                Some(lmt) == self.format.last_modified,
                "Версия сегмента SABR изменилась"
            );
        }
        if let Some(tags) = &h.xtags {
            ensure!(
                Some(tags) == self.format.xtags.as_ref(),
                "SABR вернул другую аудиодорожку"
            );
        }
        ensure!(
            h.compression_algorithm.is_none_or(|n| n == 0),
            "Сжатые сегменты SABR не поддерживаются"
        );
        let id = u8::try_from(
            h.header_id
                .ok_or_else(|| anyhow::anyhow!("Нет номера заголовка SABR"))?,
        )
        .map_err(|_| anyhow::anyhow!("Неверный номер заголовка SABR"))?;
        let expected = h
            .content_length
            .map(u64::try_from)
            .transpose()
            .map_err(|_| anyhow::anyhow!("Неверный размер сегмента SABR"))?;
        ensure!(
            expected.is_none_or(|n| n > 0 && n <= SEGMENT_LIMIT),
            "Сегмент SABR превышает предел"
        );
        let is_init = h.is_init_seg.unwrap_or(false);
        let (sequence, start, duration, duplicate) = if is_init {
            (None, 0, 0, self.init.is_some())
        } else {
            ensure!(self.init.is_some(), "Нет начального сегмента SABR");
            let sequence = u32::try_from(h.sequence_number.unwrap_or(-1))
                .map_err(|_| anyhow::anyhow!("Нет номера сегмента SABR"))?;
            let (last, _) = self
                .terminal
                .ok_or_else(|| anyhow::anyhow!("Нет конечных данных SABR"))?;
            ensure!(
                sequence > 0 && sequence <= last,
                "Неверный номер сегмента SABR"
            );
            let start = timing(h.start_ms, h.time_range.as_ref(), false)?;
            let duration = timing(h.duration_ms, h.time_range.as_ref(), true)?;
            ensure!(
                start >= 0 && duration > 0 && duration <= 3_600_000,
                "Неверное время сегмента SABR"
            );
            start
                .checked_add(duration)
                .ok_or_else(|| anyhow::anyhow!("Переполнение времени SABR"))?;
            let duplicate = self.committed.contains_key(&sequence);
            if !duplicate {
                ensure!(sequence == self.last_sequence + 1, "Пропущен сегмент SABR");
                ensure!(start.abs_diff(self.end_ms) <= 2, "Пропуск времени SABR");
            }
            (Some(sequence), start, duration, duplicate)
        };
        self.pending = Some(Pending {
            segment: Segment {
                id,
                sequence,
                start,
                duration,
                length: 0,
                digest: [0; 32],
                duplicate,
            },
            expected,
            hasher: Sha256::new(),
        });
        Ok(())
    }
    pub fn data(&mut self, id: u8, data: &[u8]) -> Result<()> {
        let p = self
            .pending
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Медиаданные SABR без заголовка"))?;
        ensure!(p.segment.id == id, "SABR смешал незавершённые сегменты");
        let length = p
            .segment
            .length
            .checked_add(data.len() as u64)
            .ok_or_else(|| anyhow::anyhow!("Переполнение размера SABR"))?;
        ensure!(
            length <= SEGMENT_LIMIT && p.expected.is_none_or(|n| length <= n),
            "Размер сегмента SABR не совпадает"
        );
        if !p.segment.duplicate {
            ensure!(
                length <= self.expected_bytes.saturating_sub(self.bytes),
                "SABR превысил размер файла"
            );
        }
        p.segment.length = length;
        p.hasher.update(data);
        Ok(())
    }
    pub fn finish_segment(&self, id: u8) -> Result<Segment> {
        let p = self
            .pending
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("SABR завершил неизвестный сегмент"))?;
        ensure!(
            p.segment.id == id && p.segment.length > 0,
            "Неверное завершение сегмента SABR"
        );
        ensure!(
            p.expected.is_none_or(|n| n == p.segment.length),
            "Оборванный сегмент SABR"
        );
        let mut s = p.segment.clone();
        s.digest = p.hasher.clone().finalize().into();
        if s.duplicate {
            let old = match s.sequence {
                None => self.init.as_ref(),
                Some(n) => self.committed.get(&n),
            }
            .ok_or_else(|| anyhow::anyhow!("Неизвестный повтор SABR"))?;
            ensure!(
                old.length == s.length
                    && old.digest == s.digest
                    && old.start == s.start
                    && old.duration == s.duration,
                "Повтор SABR содержит другие данные"
            );
        }
        Ok(s)
    }
    pub fn commit(&mut self, s: Segment) -> Result<()> {
        ensure!(self.pending.is_some(), "Нет подтверждаемого сегмента SABR");
        if !s.duplicate {
            self.bytes = self
                .bytes
                .checked_add(s.length)
                .ok_or_else(|| anyhow::anyhow!("Переполнение файла SABR"))?;
            if let Some(sequence) = s.sequence {
                self.last_sequence = sequence;
                self.end_ms = s.start + s.duration;
                self.committed.insert(sequence, s);
            } else {
                self.init = Some(s);
            }
        }
        self.pending = None;
        Ok(())
    }
    pub fn covered(&self) -> bool {
        self.pending.is_none()
            && self.init.is_some()
            && self.terminal.is_some_and(|(last, end)| {
                self.last_sequence == last
                    && self.committed.len() == last as usize
                    && self.end_ms.abs_diff(end) <= 2
            })
    }
    pub fn complete(&self) -> bool {
        self.covered() && self.bytes == self.expected_bytes
    }
}
fn timing(direct: Option<i64>, range: Option<&TimeRange>, duration: bool) -> Result<i64> {
    if let Some(value) = direct {
        return Ok(value);
    }
    let r = range.ok_or_else(|| anyhow::anyhow!("Нет времени сегмента SABR"))?;
    let scale = r
        .timescale
        .filter(|n| *n > 0)
        .ok_or_else(|| anyhow::anyhow!("Неверная шкала времени SABR"))?;
    let ticks = if duration {
        r.duration_ticks
    } else {
        r.start_ticks
    }
    .ok_or_else(|| anyhow::anyhow!("Нет времени сегмента SABR"))?;
    ticks
        .checked_mul(1000)
        .map(|n| n / i64::from(scale))
        .ok_or_else(|| anyhow::anyhow!("Переполнение времени SABR"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn track() -> Track {
        let id = FormatId {
            itag: Some(251),
            last_modified: Some(1),
            xtags: None,
        };
        let mut t = Track::new(id.clone(), "fixture0001".into(), 9, 2000).unwrap();
        t.initialize(FormatInit {
            video_id: Some("fixture0001".into()),
            format_id: Some(id),
            end_time_ms: Some(2000),
            end_segment_number: Some(2),
            mime_type: None,
        })
        .unwrap();
        let h = header(None, 0, 0, 1);
        t.header(h).unwrap();
        t.data(7, b"i").unwrap();
        let s = t.finish_segment(7).unwrap();
        t.commit(s).unwrap();
        t
    }
    fn header(sequence: Option<i32>, start: i64, duration: i64, length: i64) -> MediaHeader {
        MediaHeader {
            header_id: Some(7),
            format_id: Some(FormatId {
                itag: Some(251),
                last_modified: Some(1),
                xtags: None,
            }),
            is_init_seg: Some(sequence.is_none()),
            sequence_number: sequence,
            start_ms: Some(start),
            duration_ms: Some(duration),
            content_length: Some(length),
            ..Default::default()
        }
    }
    fn append(t: &mut Track, n: i32, start: i64, data: &[u8]) {
        t.header(header(Some(n), start, 1000, data.len() as i64))
            .unwrap();
        t.data(7, data).unwrap();
        let s = t.finish_segment(7).unwrap();
        t.commit(s).unwrap();
    }
    #[test]
    fn missing_middle_sequence_cannot_be_replaced_by_maximum() {
        let mut t = track();
        assert!(t.header(header(Some(2), 0, 1000, 4)).is_err());
        assert!(!t.complete());
    }
    #[test]
    fn partial_retry_and_duplicate_do_not_advance_or_duplicate_output() {
        let mut t = track();
        append(&mut t, 1, 0, b"abcd");
        assert_eq!(t.bytes, 5);
        t.header(header(Some(2), 1000, 1000, 4)).unwrap();
        t.data(7, b"xy").unwrap();
        assert!(t.finish_segment(7).is_err());
        t.rollback();
        assert_eq!(t.bytes, 5);
        append(&mut t, 1, 0, b"abcd");
        assert_eq!(t.bytes, 5);
        append(&mut t, 2, 1000, b"wxyz");
        assert!(t.complete());
        assert_eq!(t.bytes, 9);
    }
    #[test]
    fn terminal_and_declared_size_both_required() {
        let mut t = track();
        append(&mut t, 1, 0, b"abcd");
        assert!(!t.complete());
        t.expected_bytes = 10;
        append(&mut t, 2, 1000, b"wxyz");
        assert!(!t.complete());
    }
    #[test]
    fn two_jobs_with_same_itag_have_independent_coverage() {
        let mut a = track();
        let b = track();
        append(&mut a, 1, 0, b"abcd");
        append(&mut a, 2, 1000, b"wxyz");
        assert!(a.complete());
        assert!(!b.complete());
        assert_eq!(b.bytes, 1);
    }
    #[test]
    fn duplicate_with_different_bytes_is_rejected() {
        let mut t = track();
        append(&mut t, 1, 0, b"abcd");
        t.header(header(Some(1), 0, 1000, 4)).unwrap();
        t.data(7, b"evil").unwrap();
        assert!(t.finish_segment(7).is_err());
    }
}
