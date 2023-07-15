use std::error::Error;
use std::ops::{Deref, DerefMut};

cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        mod windows;
        use windows as os;
    } else if #[cfg(target_family = "unix")] {
        mod unix;
        use unix as os;
    } else {
        unimplemented!("This crate does not support your OS yet !");
    }
}
use crate::{Result, Timeout};
pub use os::*;

pub enum LockResult<'a> {
    Ok(LockGuard<'a>),
    Abandoned(LockGuard<'a>),
    Failed(Box<dyn Error>),
}

pub enum ReadLockResult<'a> {
    Ok(ReadLockGuard<'a>),
    Abandoned(ReadLockGuard<'a>),
    Failed(Box<dyn Error>),
}

impl <'a> LockResult<'a> {
    pub fn deny_abandoned(self) -> Result<LockGuard<'a>> {
        match self {
            LockResult::Ok(guard) => Ok(guard),
            LockResult::Abandoned(_) => Err(From::from("A thread holding the mutex has left it in a poisened state")),
            LockResult::Failed(err) => Err(err),
        }
    }
}

impl <'a> ReadLockResult<'a> {
    pub fn deny_abandoned(self) -> Result<ReadLockGuard<'a>> {
        match self {
            ReadLockResult::Ok(guard) => Ok(guard),
            ReadLockResult::Abandoned(_) => Err(From::from("A thread holding the mutex has left it in a poisened state")),
            ReadLockResult::Failed(err) => Err(err),
        }
    }
}

pub trait LockInit {
    /// Size required for the lock's internal representation
    fn size_of(addr: Option<*mut u8>) -> usize;

    /// Initializes a new instance of the lock in the provided buffer and returns the number of used bytes
    /// # Safety
    /// This function is unsafe because it cannot guarantee that the provided memory is valid.
    #[allow(clippy::new_ret_no_self)]
    unsafe fn new(mem: *mut u8, data: *mut u8) -> Result<(Box<dyn LockImpl>, usize)>;

    /// Re-uses a lock from an already initialized location and returns the number of used bytes
    /// # Safety
    /// This function is unsafe because it cannot guarantee that the provided memory is valid.
    #[allow(clippy::new_ret_no_self)]
    unsafe fn from_existing(mem: *mut u8, data: *mut u8) -> Result<(Box<dyn LockImpl>, usize)>;
}

pub trait LockImpl {
    fn as_raw(&self) -> *mut std::ffi::c_void;
    /// Acquires the lock
    fn lock(&self) -> LockResult;

    /// Acquires lock with timeout
    fn try_lock(&self, timeout: Timeout) -> LockResult;

    /// Release the lock
    fn release(&self) -> Result<()>;

    /// Acquires the lock for read access only. This method uses `lock()` as a fallback
    fn rlock(&self) -> ReadLockResult {
        match self.lock() {
            LockResult::Ok(guard) => ReadLockResult::Ok(guard.into_read_guard()),
            LockResult::Abandoned(guard) => ReadLockResult::Abandoned(guard.into_read_guard()),
            LockResult::Failed(err) => ReadLockResult::Failed(err),
        }
    }

    /// Acquires the lock for read access only with timeout. This method uses `lock()` as a fallback
    fn try_rlock(&self, timeout: Timeout) -> ReadLockResult {
        match self.try_lock(timeout) {
            LockResult::Ok(guard) => ReadLockResult::Ok(guard.into_read_guard()),
            LockResult::Abandoned(guard) => ReadLockResult::Abandoned(guard.into_read_guard()),
            LockResult::Failed(err) => ReadLockResult::Failed(err),
        }
    }

    /// Leaks the inner data without acquiring the lock
    #[doc(hidden)]
    #[allow(clippy::mut_from_ref)]
    unsafe fn get_inner(&self) -> &mut *mut u8;
}

/// Used to wrap an acquired lock's data. Lock is automatically released on `Drop`
pub struct LockGuard<'t> {
    lock: &'t dyn LockImpl,
}
impl<'t> Drop for LockGuard<'t> {
    fn drop(&mut self) {
        self.lock.release().unwrap();
    }
}
impl<'t> LockGuard<'t> {
    fn new(lock_impl: &'t dyn LockImpl) -> Self {
        Self { lock: lock_impl }
    }
    pub fn into_read_guard(self) -> ReadLockGuard<'t> {
        let inner_lock = self.lock;
        std::mem::forget(self);
        ReadLockGuard::new(inner_lock)
    }
}
impl<'t> Deref for LockGuard<'t> {
    type Target = *mut u8;
    fn deref(&self) -> &Self::Target {
        unsafe { self.lock.get_inner() }
    }
}
impl<'t> DerefMut for LockGuard<'t> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.lock.get_inner() }
    }
}

/// Used to wrap an acquired lock's read only data. Lock is automatically released on `Drop`
pub struct ReadLockGuard<'t> {
    lock: &'t dyn LockImpl,
}
impl<'t> ReadLockGuard<'t> {
    fn new(lock_impl: &'t dyn LockImpl) -> Self {
        Self { lock: lock_impl }
    }
}

impl<'t> Drop for ReadLockGuard<'t> {
    fn drop(&mut self) {
        self.lock.release().unwrap();
    }
}
impl<'t> Deref for ReadLockGuard<'t> {
    type Target = *const u8;
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.lock.get_inner() as *mut *mut u8 as *const *const u8) }
    }
}
