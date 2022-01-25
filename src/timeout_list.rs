use std::cmp;
use std::collections::{BinaryHeap, HashMap};
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use crossbeam::atomic::AtomicCell;
use crate::std::queue::seg_queue::SegQueue as mpsc;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use crate::std::queue::mpsc_list_v1::Entry;
use crate::std::queue::mpsc_list_v1::Queue as TimeoutQueue;

const NANOS_PER_MILLI: u64 = 1_000_000;
const NANOS_PER_SEC: u64 = 1_000_000_000;

const HASH_CAP: usize = 1024;

#[inline]
fn dur_to_ns(dur: Duration) -> u64 {
    // Note that a duration is a (u64, u32) (seconds, nanoseconds) pair
    dur.as_secs()
        .saturating_mul(NANOS_PER_SEC)
        .saturating_add(u64::from(dur.subsec_nanos()))
}

#[inline]
pub fn ns_to_dur(ns: u64) -> Duration {
    Duration::new(ns / NANOS_PER_SEC, (ns % NANOS_PER_SEC) as u32)
}

#[allow(dead_code)]
#[inline]
pub fn ns_to_ms(ns: u64) -> u64 {
    (ns + NANOS_PER_MILLI - 1) / NANOS_PER_MILLI
}

pub static START_TIME: Lazy<Instant> = Lazy::new(|| { Instant::now() });


// get the current wall clock in ns
#[inline]
pub fn now() -> u64 {
    // we need a Monotonic Clock here
    START_TIME.elapsed().as_nanos() as u64
}

// timeout event data
pub struct TimeoutData<T> {
    time: u64,
    // the wall clock in ns that the timer expires
    pub data: T, // the data associate with the timeout event
}

// timeout handler which can be removed/cancelled
pub type TimeoutHandle<T> = Entry<TimeoutData<T>>;

struct TimeoutQueueWrapper<T> {
    inner: TimeoutQueue<TimeoutData<T>>,
    in_use: AtomicUsize,
}

impl<T> TimeoutQueueWrapper<T> {
    fn new() -> Self {
        TimeoutQueueWrapper {
            inner: TimeoutQueue::new(),
            in_use: AtomicUsize::new(0),
        }
    }
}

type IntervalList<T> = Arc<TimeoutQueueWrapper<T>>;

// this is the data type that used by the binary heap to get the latest timer
struct IntervalEntry<T> {
    time: u64,
    // the head timeout value in the list, should be latest
    list: IntervalList<T>,
    // point to the interval list
    interval: u64,
}

impl<T> IntervalEntry<T> {
    // trigger the timeout event with the supplying function
    // return next expire time
    pub fn pop_timeout<F>(&self, now: u64, f: &F) -> Option<u64>
        where
            F: Fn(T),
    {
        let p = |v: &TimeoutData<T>| v.time <= now;
        while let Some(timeout) = self.list.inner.pop_if(&p) {
            f(timeout.data);
        }
        self.list.inner.peek().map(|t| t.time)
    }
}

impl<T> PartialEq for IntervalEntry<T> {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl<T> Eq for IntervalEntry<T> {}

impl<T> PartialOrd for IntervalEntry<T> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> cmp::Ord for IntervalEntry<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        other.time.cmp(&self.time)
    }
}

// the timeout list data structure
pub struct TimeOutList<T> {
    // interval based hash map, protected by rw lock
    interval_map: RwLock<HashMap<u64, IntervalList<T>>>,
    // a priority queue, each element is the head of a mpsc queue
    timer_bh: Mutex<BinaryHeap<IntervalEntry<T>>>,
}

impl<T> TimeOutList<T> {
    pub fn new() -> Self {
        TimeOutList {
            interval_map: RwLock::new(HashMap::with_capacity(HASH_CAP)),
            timer_bh: Mutex::new(BinaryHeap::new()),
        }
    }

    fn install_timer_bh(&self, entry: IntervalEntry<T>) {
        if entry.list.in_use.fetch_add(1, Ordering::AcqRel) == 0 {
            self.timer_bh.lock().push(entry);
        }
    }

    // add a timeout event to the list
    // this can be called in any thread
    // return true if we need to recall next expire
    pub fn add_timer(&self, dur: Duration, data: T) -> (TimeoutHandle<T>, bool) {
        let interval = dur_to_ns(dur);
        let time = now() + interval; // TODO: deal with overflow?
        //println!("add timer = {:?}", time);

        let timeout = TimeoutData { time, data };

        let interval_list = {
            // use the read lock protect
            let interval_map_r = self.interval_map.read().unwrap();
            (*interval_map_r).get(&interval).cloned()
            // drop the read lock here
        };

        if let Some(interval_list) = interval_list {
            let (handle, is_head) = interval_list.inner.push(timeout);
            if is_head {
                // install the interval list to the binary heap
                self.install_timer_bh(IntervalEntry {
                    time,
                    interval,
                    list: interval_list,
                });
            }
            return (handle, is_head);
        }

        // if the interval list is not there, get the write locker to install the list
        // use the write lock protect
        let mut interval_map_w = self.interval_map.write().unwrap();
        // recheck the interval list in case other thread may install it
        if let Some(interval_list) = (*interval_map_w).get(&interval) {
            let (handle, is_head) = interval_list.inner.push(timeout);
            if is_head {
                // this rarely happens
                self.install_timer_bh(IntervalEntry {
                    time,
                    interval,
                    list: interval_list.clone(),
                });
            }
            return (handle, is_head);
        }

        let interval_list = Arc::new(TimeoutQueueWrapper::<T>::new());
        let ret = interval_list.inner.push(timeout).0;
        (*interval_map_w).insert(interval, interval_list.clone());
        // drop the write lock here
        mem::drop(interval_map_w);

        // install the new interval list to the binary heap
        self.install_timer_bh(IntervalEntry {
            time,
            interval,
            list: interval_list,
        });

        (ret, true)
    }

    // schedule in the timer thread
    // this will remove all the expired timeout event
    // and call the supplied function with registered data
    // return the time in ns for the next expiration
    pub fn schedule_timer<F: Fn(T)>(&self, now: u64, f: &F) -> Option<u64> {
        loop {
            // first peek the BH to see if there is any timeout event
            let mut entry = {
                let mut timer_bh = self.timer_bh.lock();
                match timer_bh.peek() {
                    // the latest timeout event not happened yet
                    Some(entry) => {
                        if entry.time > now {
                            return Some(entry.time - now);
                        } else {
                            // find out one entry
                        }
                    }
                    None => return None,
                }
                let entry = timer_bh.pop().unwrap();
                entry.list.in_use.store(0, Ordering::Release);
                entry
            };

            // consume all the timeout event
            // the binary heap can be modified here
            // during running the timeout handler
            match entry.pop_timeout(now, f) {
                Some(time) => {
                    if entry.list.in_use.fetch_add(1, Ordering::AcqRel) == 0 {
                        // re-push the entry
                        entry.time = time;
                        self.timer_bh.lock().push(entry);
                    }
                }

                None => {
                    // if the interval list is empty, need to delete it
                    let mut interval_map_w = self.interval_map.write().unwrap();
                    // recheck if the interval list is empty, other thread may append data to it
                    if entry.list.inner.is_empty() {
                        // if the len of the hash map is big enough just leave the queue there
                        if (*interval_map_w).len() > HASH_CAP {
                            // the list is really empty now, we can safely remove it
                            (*interval_map_w).remove(&entry.interval);
                        }
                    } else if entry.list.in_use.fetch_add(1, Ordering::AcqRel) == 0 {
                        // release the w lock first, we don't need it any more
                        mem::drop(interval_map_w);
                        // the list is push some data by other thread
                        entry.time = entry.list.inner.peek().unwrap().time;
                        self.timer_bh.lock().push(entry);
                    }
                }
            }
        }
    }
}

pub struct TimerThread<T> {
    timer_list: TimeOutList<T>,
    // collect the remove request
    remove_list: mpsc<TimeoutHandle<T>>,
    // the timer thread wakeup handler
    wakeup: AtomicCell<Option<thread::Thread>>,
}

impl<T> TimerThread<T> {
    pub fn new() -> Self {
        TimerThread {
            timer_list: TimeOutList::new(),
            remove_list: mpsc::new(),
            wakeup: AtomicCell::new(None),
        }
    }

    pub fn add_timer(&self, dur: Duration, data: T) -> TimeoutHandle<T> {
        let (h, is_recal) = self.timer_list.add_timer(dur, data);
        // wake up the timer thread if it's a new queue
        if is_recal {
            if let Some(t) = self.wakeup.take() {
                t.unpark();
            }
        }
        h
    }

    pub fn del_timer(&self, handle: TimeoutHandle<T>) {
        self.remove_list.push(handle);
        if let Some(t) = self.wakeup.take() {
            t.unpark();
        }
    }

    // the timer thread function
    pub fn run<F: Fn(T)>(&self, f: &F) {
        let current_thread = thread::current();
        loop {
            while let Some(h) = self.remove_list.pop() {
                h.remove();
            }
            // we must register the thread handle first
            // or there will be no signal to wakeup the timer thread
            self.wakeup.swap(Some(current_thread.clone()));

            if !self.remove_list.is_empty() {
                if let Some(t) = self.wakeup.take() {
                    t.unpark();
                }
            }

            match self.timer_list.schedule_timer(now(), f) {
                Some(time) => thread::park_timeout(ns_to_dur(time)),
                None => thread::park(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_timeout_list() {
        let timer = Arc::new(TimerThread::<usize>::new());
        let t = timer.clone();
        let f = |data: usize| {
            println!("timeout data:{:?}", data);
        };
        thread::spawn(move || t.run(&f));
        let t1 = timer.clone();
        thread::spawn(move || {
            t1.add_timer(Duration::from_millis(1000), 50);
            t1.add_timer(Duration::from_millis(1000), 60);
            t1.add_timer(Duration::from_millis(1400), 70);
        });
        thread::sleep(Duration::from_millis(10));
        timer.add_timer(Duration::from_millis(1000), 10);
        timer.add_timer(Duration::from_millis(500), 40);
        timer.add_timer(Duration::from_millis(1200), 20);
        thread::sleep(Duration::from_millis(100));
        timer.add_timer(Duration::from_millis(1000), 30);

        thread::sleep(Duration::from_millis(1500));
    }
}
