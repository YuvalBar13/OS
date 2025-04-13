use std::fs::File;
use std::io::Write;
use std::path::Path;

pub const DISK_IMAGE: &str = "disk.img";
const DISK_SIZE: u64 = 32 * 1024 * 1024; // 32MB virtual disk

pub fn create_disk_if_not_exists() {
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