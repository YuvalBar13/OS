use std::{
    env,
    process::{self, Command},
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
};

const DISK_IMAGE: &str = "disk.img";
const DISK_SIZE: u64 = 32 * 1024 * 1024; // 32MB virtual disk

fn create_disk_if_not_exists() {
    if !Path::new(DISK_IMAGE).exists() {
        println!("Creating virtual disk image...");
        let mut file = File::create(DISK_IMAGE).unwrap();

        // Create an empty file of DISK_SIZE bytes
        file.set_len(DISK_SIZE).unwrap();

        // Optional: Initialize with zeros
        let zeros = vec![0u8; 512];
        for _ in 0..(DISK_SIZE / 512) {
            file.write_all(&zeros).unwrap();
        }
    }
}

fn main() {
    create_disk_if_not_exists();

    let mut qemu = Command::new("qemu-system-x86_64");

    // BIOS drive with index=0
    qemu.arg("-drive");
    qemu.arg(format!("format=raw,file={},index=0", env!("BIOS_IMAGE")));

    // Virtual disk with index=1
    qemu.arg("-drive");
    qemu.arg(format!("format=raw,file={},if=ide,index=1", DISK_IMAGE));

    let exit_status = qemu.status().unwrap();
    process::exit(exit_status.code().unwrap_or(-1));
}