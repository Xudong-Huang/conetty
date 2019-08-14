use std::io::IoSlice;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam::queue::SegQueue;

struct VecBufs {
    block: usize,
    pos: usize,
    bufs: Vec<Vec<u8>>,
}

impl VecBufs {
    fn new(bufs: Vec<Vec<u8>>) -> Self {
        VecBufs {
            block: 0,
            pos: 0,
            bufs,
        }
    }

    fn get_io_slice(&self) -> Vec<IoSlice<'_>> {
        let mut ret = Vec::with_capacity(self.bufs.len());
        let first = IoSlice::new(&self.bufs[self.block][self.pos..]);
        ret.push(first);
        for buf in self.bufs.iter().skip(self.block + 1) {
            ret.push(IoSlice::new(buf))
        }
        ret
    }

    fn advance(&mut self, n: usize) {
        let mut lefted = n;
        for buf in self.bufs.iter() {
            let len = buf.len() - self.pos;
            if lefted >= len {
                lefted -= len;
                self.block += 1;
                self.pos = 0;
            } else {
                self.pos += lefted;
                break;
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.block == self.bufs.len()
    }

    fn to_writer<W: Write>(mut self, writer: &mut W) -> std::io::Result<()> {
        while !self.is_empty() {
            let n = writer.write_vectored(&self.get_io_slice())?;
            self.advance(n);
        }
        Ok(())
    }
}

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
            #[allow(clippy::cast_ref_to_mut)]
            let writer = unsafe { &mut *(&self.writer as *const _ as *mut W) };

            loop {
                let mut totoal_data = Vec::with_capacity(1024);
                while let Ok(data) = self.data_queue.pop() {
                    totoal_data.push(data);
                    cnt += 1;
                }

                let io_bufs = VecBufs::new(totoal_data);
                if let Err(e) = io_bufs.to_writer(writer) {
                    // FIXME: handle the error
                    error!("QueuedWriter failed, err={}", e);
                }

                if self.data_count.fetch_sub(cnt, Ordering::AcqRel) == cnt {
                    break;
                }

                cnt = 0;
            }
        }
    }
}
