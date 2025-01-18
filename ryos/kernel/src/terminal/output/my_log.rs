#[macro_export]
macro_rules! my_info {

    (target: $target:expr, $($arg:tt)+) => {
        x86_64::instructions::interrupts::without_interrupts(|| {log::info!(target: $target, $($arg)+)})
    };

    ($($arg:tt)+) => {
        x86_64::instructions::interrupts::without_interrupts(|| {log::info!($($arg)+)})
    };
}