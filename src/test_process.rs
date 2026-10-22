//! Own the test command's process group, not just its outer launcher.
use std::io;
use std::ops::{Deref, DerefMut};
use std::process::{Child, Command};

#[cfg(unix)]
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicUsize, Ordering},
};

#[cfg(unix)]
static CANCELLED: OnceLock<Arc<AtomicUsize>> = OnceLock::new();

/// Install after the baseline and before starting any isolated process groups.
/// Handlers only set a flag; the runner cleans up and restores source normally.
pub fn install_signal_handlers() -> io::Result<()> {
    #[cfg(unix)]
    if CANCELLED.get().is_none() {
        let signal = Arc::new(AtomicUsize::new(0));
        let mut handlers = Vec::new();
        for number in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
            match signal_hook::flag::register_usize(number, signal.clone(), number as usize) {
                Ok(handler) => handlers.push(handler),
                Err(error) => {
                    for handler in handlers {
                        signal_hook::low_level::unregister(handler);
                    }
                    return Err(error);
                }
            }
        }
        let _ = CANCELLED.set(signal);
    }
    Ok(())
}

pub fn cancellation_signal() -> i32 {
    #[cfg(unix)]
    return CANCELLED
        .get()
        .map_or(0, |signal| signal.load(Ordering::Relaxed) as i32);
    #[cfg(not(unix))]
    0
}

pub(crate) struct TestProcess {
    child: Child,
    terminated: bool,
}

impl TestProcess {
    pub(crate) fn spawn(command: &mut Command) -> io::Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        command.spawn().map(|child| Self {
            child,
            terminated: false,
        })
    }

    pub(crate) fn terminate(&mut self) {
        if self.terminated {
            return;
        }
        #[cfg(unix)]
        {
            // The child was made group leader at spawn. A negative PID signals
            // its entire group, including npx's shell and Vitest workers.
            unsafe {
                libc::kill(-(self.child.id() as libc::pid_t), libc::SIGKILL);
            }
        }
        // Also reap the direct child, including on non-Unix platforms.
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.terminated = true;
    }
}

impl Deref for TestProcess {
    type Target = Child;
    fn deref(&self) -> &Child {
        &self.child
    }
}

impl DerefMut for TestProcess {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.child
    }
}

impl Drop for TestProcess {
    fn drop(&mut self) {
        self.terminate();
    }
}
