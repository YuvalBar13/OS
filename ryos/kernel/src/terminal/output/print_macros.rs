use x86_64::instructions::interrupts::without_interrupts;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({x86_64::instructions::interrupts::without_interrupts(|| {
        use core::fmt::Write;
        let _ =  x86_64::instructions::interrupts::without_interrupts(|| {write!($crate::terminal::output::framebuffer::WRITER
            .get()
            .expect("Writer not initialized")
            .lock(),
            $($arg)*
        )});

    });})
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! eprint {
    () => ($crate::print!(""));
    ($($arg:tt)*) => ({
        $crate::terminal::output::framebuffer::WRITER
            .get()
            .expect("Writer not initialized")
            .lock()
            .change_color($crate::terminal::output::framebuffer::ERROR_COLOR.clone());
        $crate::print!("{}", format_args!($($arg)*));
         $crate::terminal::output::framebuffer::WRITER
            .get()
            .expect("Writer not initialized")
            .lock()
            .change_color($crate::terminal::output::framebuffer::DEFAULT_COLOR.clone());
    });
}

#[macro_export]
macro_rules! eprintln {
    () => ($crate::eprint!("\n"));
    ($($arg:tt)*) => ($crate::eprint!("{}\n", format_args!($($arg)*)));
}
