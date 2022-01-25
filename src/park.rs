use std::fmt;
use std::fmt::Debug;
use std::io::ErrorKind;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};
use std::sync::{Arc, PoisonError};
use std::time::Duration;

use crate::cancel::Cancel;
use crate::coroutine_impl::{co_cancel_data, run_coroutine, CoroutineImpl, EventSource};
use crate::scheduler::get_scheduler;
use crate::std::queue::spsc;
use crate::std::queue::spsc::Queue;
use crate::std::sync::atomic_dur::AtomicDuration;
use crate::std::sync::AtomicOption;
use crate::timeout_list::TimeoutHandle;
use crate::yield_now::{get_co_para, yield_now, yield_with};


pub trait Park: Send + Sync + Debug {
    fn park(&self) -> Result<(), ParkError>;
    fn park_timeout(&self, d: Duration) -> Result<(), ParkError>;
}

pub trait UnPark: Send + Sync + Debug {
    fn unpark(&self);
}

pub trait ParkUnPark: Park + UnPark {
    fn park_option(&self, d: Option<Duration>) -> Result<(), ParkError> {
        match d {
            None => { self.park() }
            Some(d) => { self.park_timeout(d) }
        }
    }
}


#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ParkError {
    Canceled,
    Timeout,
}

impl<T> From<PoisonError<T>> for ParkError {
    fn from(_: PoisonError<T>) -> Self {
        Self::Timeout
    }
}


pub struct DropGuard<'a>(&'a ParkImpl);

pub struct ParkImpl {
    // the coroutine that waiting for this park instance
    wait_co: Arc<AtomicOption<CoroutineImpl>>,
    // when odd means the Park no need to block
    // the low bit used as flag, and higher bits used as flag to check the kernel delay drop
    state: AtomicUsize,
    // control how to deal with the cancellation, usually init one time
    check_cancel: AtomicBool,
    // timeout settings in ms, 0 is none (park forever)
    timeout: AtomicDuration,
    // timer handle, can be null
    timeout_handle: AtomicPtr<TimeoutHandle<Arc<AtomicOption<CoroutineImpl>>>>,
    // a flag if kernel is entered
    wait_kernel: AtomicBool,
}

impl Park for ParkImpl {
    fn park(&self) -> Result<(), ParkError> {
        self.park_timeout(None)
    }

    fn park_timeout(&self, d: Duration) -> Result<(), ParkError> {
        self.park_timeout(Some(d))
    }
}

impl UnPark for ParkImpl {
    fn unpark(&self) {
        self.unpark()
    }
}

impl Default for ParkImpl {
    fn default() -> Self {
        ParkImpl::new()
    }
}

impl ParkUnPark for ParkImpl {}

// this is the park resource type (spmc style)
impl ParkImpl {
    pub fn new() -> Self {
        ParkImpl {
            wait_co: Arc::new(AtomicOption::none()),
            state: AtomicUsize::new(0),
            check_cancel: true.into(),
            timeout: AtomicDuration::new(None),
            timeout_handle: AtomicPtr::new(ptr::null_mut()),
            wait_kernel: AtomicBool::new(false),
        }
    }

    // ignore cancel, if true, caller have to do the check instead
    pub fn ignore_cancel(&self, ignore: bool) {
        self.check_cancel.store(!ignore, Ordering::Relaxed);
    }

    #[inline]
    fn set_timeout_handle(
        &self,
        handle: Option<TimeoutHandle<Arc<AtomicOption<CoroutineImpl>>>>,
    ) -> Option<TimeoutHandle<Arc<AtomicOption<CoroutineImpl>>>> {
        let ptr = match handle {
            None => ptr::null_mut(),
            Some(h) => h.into_ptr(),
        };

        let old_ptr = self.timeout_handle.swap(ptr, Ordering::Relaxed);
        if old_ptr.is_null() {
            None
        } else {
            Some(unsafe { TimeoutHandle::from_ptr(old_ptr) })
        }
    }

    // return true if need park the coroutine
    // when the state is true, we clear it and indicate not to block
    // when the state is false, means we need real park
    #[inline]
    fn check_park(&self) -> bool {
        let mut state = self.state.load(Ordering::Acquire);
        if state & 1 == 0 {
            return true;
        }

        loop {
            match self.state.compare_exchange_weak(
                state,
                state - 1,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return false, // successfully consume the state, no need to block
                Err(x) => {
                    if x & 1 == 0 {
                        return true;
                    }
                    state = x;
                }
            }
        }
    }

    // unpark the underlying coroutine if any
    #[inline]
    pub(crate) fn unpark_impl(&self, b_sync: bool) {
        let mut state = self.state.load(Ordering::Acquire);
        if state & 1 == 1 {
            // the state is already set do nothing here
            return;
        }

        loop {
            match self.state.compare_exchange_weak(
                state,
                state + 1,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return self.wake_up(b_sync),
                Err(x) => {
                    if x & 1 == 1 {
                        break; // already set, do nothing
                    }
                    state = x;
                }
            }
        }
    }

    // unpark the underlying coroutine if any, push to the ready task queue
    #[inline]
    pub fn unpark(&self) {
        self.unpark_impl(false);
    }

    // remove the timeout handle after return back to user space
    #[inline]
    fn remove_timeout_handle(&self) {
        if let Some(h) = self.set_timeout_handle(None) {
            if h.is_link() {
                get_scheduler().del_timer(h);
            }
            // when timeout the node is unlinked
            // just drop it to release memory
        }
    }

    #[inline]
    fn wake_up(&self, b_sync: bool) {
        if let Some(co) = self.wait_co.take(Ordering::Acquire) {
            if b_sync {
                run_coroutine(co);
            } else {
                get_scheduler().schedule(co);
            }
        }
    }

    /// park current coroutine with specified timeout
    /// if timeout happens, return Err(ParkError::Timeout)
    /// if cancellation detected, return Err(ParkError::Canceled)
    pub fn park_timeout(&self, dur: Option<Duration>) -> Result<(), ParkError> {
        self.timeout.swap(dur);

        // if the state is not set, need to wait
        if !self.check_park() {
            return Ok(());
        }

        // before a new yield wait the kernel done
        if self.wait_kernel.swap(false, Ordering::AcqRel) {
            while self.state.load(Ordering::Acquire) & 0x02 == 0x02 {
                yield_now();
            }
        } else {
            // should clear the generation
            self.state.fetch_and(!0x02, Ordering::Release);
        }

        // what if the state is set before yield?
        // the subscribe would re-check it
        yield_with(self);
        // clear the trigger state
        self.check_park();
        // remove timer handle
        self.remove_timeout_handle();

        // let _gen = self.state.load(Ordering::Acquire);
        // println!("unparked gen={}, self={:p}", gen, self);

        if let Some(err) = get_co_para() {
            match err.kind() {
                ErrorKind::TimedOut => return Err(ParkError::Timeout),
                ErrorKind::Other => return Err(ParkError::Canceled),
                _ => unreachable!("unexpected return error kind"),
            }
        }

        Ok(())
    }

    fn delay_drop(&self) -> DropGuard {
        self.wait_kernel.store(true, Ordering::Release);
        DropGuard(self)
    }
}

impl<'a> Drop for DropGuard<'a> {
    fn drop(&mut self) {
        // we would inc the state by 2 in kernel if all is done
        self.0.state.fetch_add(0x02, Ordering::Release);
    }
}

impl Drop for ParkImpl {
    fn drop(&mut self) {
        // wait the kernel finish
        if !self.wait_kernel.load(Ordering::Acquire) {
            return;
        }

        while self.state.load(Ordering::Acquire) & 0x02 == 0x02 {
            yield_now();
        }
        self.set_timeout_handle(None);
    }
}

impl EventSource for ParkImpl {
    // register the coroutine to the park
    fn subscribe(&mut self, co: CoroutineImpl) {
        let cancel = co_cancel_data(&co);
        // if we share the same park, the previous timer may wake up it by false
        // if we not deleted the timer in time
        let timeout_handle = self
            .timeout
            .swap(None)
            .map(|dur| get_scheduler().add_timer(dur, self.wait_co.clone()));
        self.set_timeout_handle(timeout_handle);

        let _g = self.delay_drop();

        // register the coroutine
        self.wait_co.swap(co, Ordering::Release);

        // re-check the state, only clear once after resume
        if self.state.load(Ordering::Acquire) & 1 == 1 {
            // here may have recursive call for subscribe
            // normally the recursion depth is not too deep
            return self.wake_up(true);
        }

        // register the cancel data
        cancel.set_co(self.wait_co.clone());
        // re-check the cancel status
        if cancel.is_canceled() {
            unsafe { cancel.cancel() };
        }
    }

    // when the cancel is true we check the panic or do nothing
    fn yield_back(&self, cancel: &'static Cancel) {
        // we would inc the generation by 2 to another generation
        self.state.fetch_add(0x02, Ordering::Release);

        if self.check_cancel.load(Ordering::Relaxed) {
            cancel.check_cancel();
        }
    }
}

impl fmt::Debug for ParkImpl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Park {{ co: {:?}, state: {:?} }}",
            self.wait_co, self.state
        )
    }
}
