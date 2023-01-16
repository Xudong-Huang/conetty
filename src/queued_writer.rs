use may::queue::mpsc::{Queue, BLOCK_SIZE};
use may::sync::Mutex;
use smallvec::SmallVec;

use std::io::{IoSlice, Write};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
pub struct QueuedWriter<W: Write> {
    data_count: AtomicUsize,
    data_queue: Queue<Vec<u8>>,
    writer: Mutex<W>,
}

impl<W: Write> QueuedWriter<W> {
    pub fn new(writer: W) -> Self {
        QueuedWriter {
            data_count: AtomicUsize::new(0),
            data_queue: Queue::new(),
            writer: Mutex::new(writer),
        }
    }

    /// it's safe and efficient to call this API concurrently
    pub fn write(&self, data: Vec<u8>) {
        self.data_queue.push(data);
        // only allow the first writer perform the write operation
        // other concurrent writers would just push the data
        if self.data_count.fetch_add(1, Ordering::AcqRel) == 0 {
            // in any cases this should not block since we have only one writer
            let mut writer = self.writer.lock().unwrap();

            loop {
                let mut total_data = SmallVec::<[Vec<u8>; BLOCK_SIZE]>::new();
                while let Some(v) = self.data_queue.pop() {
                    total_data.push(v);
                }
                let cnt = total_data.len();

                let mut io_bufs: SmallVec<[_; BLOCK_SIZE]> =
                    total_data.iter().map(|v| IoSlice::new(v)).collect();

                if let Err(e) = writer.write_all_vectored(&mut io_bufs) {
                    // FIXME: handle the error
                    error!("QueuedWriter failed, err={}", e);
                }

                // detect if there are more packet need to deal with
                if self.data_count.fetch_sub(cnt, Ordering::AcqRel) == cnt {
                    break;
                }
            }
        }
    }
}
