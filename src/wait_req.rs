use std::time::Duration;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use coroutine;
use frame::Frame;
use errors::Error;
use coroutine::sync::{AtomicOption, Mutex, Blocker};

pub struct WaitReq {
    pub blocker: Blocker,
    pub rsp: AtomicOption<Frame>,
}

pub struct WaitReqMap {
    map: Mutex<HashMap<usize, *mut WaitReq>>,
}

unsafe impl Send for WaitReqMap {}
unsafe impl Sync for WaitReqMap {}

impl WaitReqMap {
    pub fn new() -> Self {
        WaitReqMap { map: Mutex::new(HashMap::new()) }
    }

    pub fn add(&self, id: usize, req: &mut WaitReq) {
        let mut m = self.map.lock().unwrap();
        m.insert(id, req as *mut _);
    }

    pub fn get(&self, id: usize) -> Option<&mut WaitReq> {
        let mut m = self.map.lock().unwrap();
        m.remove(&id).map(|v| { unsafe { &mut *v } })
    }
}

// wait for response
pub fn wait_rsp(req_map: &WaitReqMap,
                id: usize,
                timeout: Duration,
                req: &mut WaitReq)
                -> Result<Frame, Error> {
    use coroutine::ParkError;

    match req.blocker.park(Some(timeout)) {
        Ok(_) => {
            match req.rsp.take(Ordering::Acquire) {
                Some(frame) => Ok(frame),
                None => panic!("unable to get the rsp, id={}", id),
            }
        }
        Err(ParkError::Timeout) => {
            // remove the req from req map
            error!("timeout zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz, id={}", id);
            req_map.get(id);
            Err(Error::Timeout)
        }
        Err(ParkError::Canceled) => {
            error!("canceled id={}", id);
            req_map.get(id);
            coroutine::trigger_cancel_panic();
        }
    }
}
