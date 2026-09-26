#![allow(dead_code)]

use core::fmt;

use crate::drivers::serial;

pub const LOG_BOOT: bool = false;
pub const LOG_TEST: bool = false;
pub const LOG_USER: bool = false;

pub const LOG_KBD: bool = false;

pub const LOG_SCHED: bool = false;
pub const LOG_CR3: bool = false;
pub const LOG_TSS: bool = false;
pub const LOG_SLEEP: bool = false;
pub const LOG_SYSCALL: bool = false;
pub const LOG_VFS: bool = true;
pub const LOG_EXEC: bool = false;
pub const LOG_RING3: bool = false;
pub const LOG_SMP: bool = false;

#[inline]
pub fn write(args: fmt::Arguments<'_>) {
    serial::print_fmt(args);
}

#[macro_export]
macro_rules! log_if {
    ($flag:expr, $($arg:tt)*) => {
        if $flag {
            $crate::log::write(core::format_args!($($arg)*));
            $crate::drivers::serial::println("");
        }
    };
}

#[macro_export]
macro_rules! log_if_raw {
    ($flag:expr, $($arg:tt)*) => {
        if $flag {
            $crate::log::write(core::format_args!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! log_boot {
    ($($arg:tt)*) => {
        $crate::log_if!($crate::log::LOG_BOOT, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_test {
    ($($arg:tt)*) => {
        $crate::log_if!($crate::log::LOG_TEST, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_user {
    ($($arg:tt)*) => {
        $crate::log_if!($crate::log::LOG_USER, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_kbd {
    ($($arg:tt)*) => {
        $crate::log_if!($crate::log::LOG_KBD, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_sched {
    ($($arg:tt)*) => {
        $crate::log_if!($crate::log::LOG_SCHED, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_cr3 {
    ($($arg:tt)*) => {
        $crate::log_if!($crate::log::LOG_CR3, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_tss {
    ($($arg:tt)*) => {
        $crate::log_if!($crate::log::LOG_TSS, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_sleep {
    ($($arg:tt)*) => {
        $crate::log_if!($crate::log::LOG_SLEEP, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_syscall {
    ($($arg:tt)*) => {
        $crate::log_if!($crate::log::LOG_SYSCALL, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_vfs {
    ($($arg:tt)*) => {
        $crate::log_if!($crate::log::LOG_VFS, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_exec {
    ($($arg:tt)*) => {
        $crate::log_if!($crate::log::LOG_EXEC, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_ring3 {
    ($($arg:tt)*) => {
        $crate::log_if!($crate::log::LOG_RING3, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_smp {
    ($($arg:tt)*) => {
        $crate::log_if!($crate::log::LOG_SMP, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_boot_raw {
    ($($arg:tt)*) => {
        $crate::log_if_raw!($crate::log::LOG_BOOT, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_test_raw {
    ($($arg:tt)*) => {
        $crate::log_if_raw!($crate::log::LOG_TEST, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_user_raw {
    ($($arg:tt)*) => {
        $crate::log_if_raw!($crate::log::LOG_USER, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_kbd_raw {
    ($($arg:tt)*) => {
        $crate::log_if_raw!($crate::log::LOG_KBD, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_sched_raw {
    ($($arg:tt)*) => {
        $crate::log_if_raw!($crate::log::LOG_SCHED, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_cr3_raw {
    ($($arg:tt)*) => {
        $crate::log_if_raw!($crate::log::LOG_CR3, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_tss_raw {
    ($($arg:tt)*) => {
        $crate::log_if_raw!($crate::log::LOG_TSS, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_sleep_raw {
    ($($arg:tt)*) => {
        $crate::log_if_raw!($crate::log::LOG_SLEEP, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_syscall_raw {
    ($($arg:tt)*) => {
        $crate::log_if_raw!($crate::log::LOG_SYSCALL, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_vfs_raw {
    ($($arg:tt)*) => {
        $crate::log_if_raw!($crate::log::LOG_VFS, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_exec_raw {
    ($($arg:tt)*) => {
        $crate::log_if_raw!($crate::log::LOG_EXEC, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_ring3_raw {
    ($($arg:tt)*) => {
        $crate::log_if_raw!($crate::log::LOG_RING3, $($arg)*)
    };
}

#[macro_export]
macro_rules! log_smp_raw {
    ($($arg:tt)*) => {
        $crate::log_if_raw!($crate::log::LOG_SMP, $($arg)*)
    };
}