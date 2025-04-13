use std::{
    env,
    process::{self, Command},
    fs::{File},
    io::Write,
    path::Path,
};

mod create_disk;

fn main() {
    create_disk::create_disk_if_not_exists();

    let mut qemu = Command::new("qemu-system-x86_64");

    // BIOS drive with index=0
    qemu.arg("-drive");
    qemu.arg(format!("format=raw,file={},index=0", env!("BIOS_IMAGE")));

    // Virtual disk with index=1
    qemu.arg("-drive");
    qemu.arg(format!("format=raw,file={},if=ide,index=1", create_disk::DISK_IMAGE));

    let exit_status = qemu.status().unwrap();
    process::exit(exit_status.code().unwrap_or(-1));
}