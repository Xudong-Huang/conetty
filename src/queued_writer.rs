use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam::queue::SegQueue;

#[derive(Debug)]
pub struct QueuedWriter<W: Write> {
    data_count: AtomicUsize,
    data_queue: SegQueue<Vec<u8>>,
    writer: W,
}

unsafe impl<W: Write + Send> Send for QueuedWriter<W> {}
unsafe impl<W: Write + Sync> Sync for QueuedWriter<W> {}

impl<W: Write> QueuedWriter<W> {
    pub fn new(writer: W) -> Self {
        QueuedWriter {
            data_count: AtomicUsize::new(0),
            data_queue: SegQueue::new(),
            writer,
        }
    }

    pub fn write(&self, data: Vec<u8>) {
        self.data_queue.push(data);
        let mut cnt = self.data_count.fetch_add(1, Ordering::AcqRel);
        if cnt == 0 {
            while let Ok(data) = self.data_queue.pop() {
                #[allow(clippy::cast_ref_to_mut)]
                let writer = unsafe { &mut *(&self.writer as *const _ as *mut W) };
                writer
                    .write_all(&data)
                    .unwrap_or_else(|e| panic!("QueuedWriter failed, err={}", e));
                cnt += 1;
            }
            self.data_count.fetch_sub(cnt, Ordering::AcqRel);
        }
    }
}
