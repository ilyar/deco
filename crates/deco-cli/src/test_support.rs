use std::sync::{LazyLock, Mutex, MutexGuard};

static CURRENT_DIR_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

pub(crate) fn cwd_lock() -> MutexGuard<'static, ()> {
    CURRENT_DIR_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
