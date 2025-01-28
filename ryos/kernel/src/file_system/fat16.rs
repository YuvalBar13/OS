// use crate::file_system::disk_driver::Disk;
// use crate::println;
//
// #[repr(C, packed)]
// struct BootSector {
//     jump_instruction: [u8; 3],
//     oem_name: [u8; 8],
//     bytes_per_sector: u16,
//     sectors_per_cluster: u8,
//     reserved_sectors: u16,
//     num_fats: u8,
//     root_entries: u16,
//     total_sectors_short: u16,
//     media_descriptor: u8,
//     sectors_per_fat: u16,
//     sectors_per_track: u16,
//     num_heads: u16,
//     hidden_sectors: u32,
//     total_sectors_long: u32,
//     drive_number: u8,
//     reserved: u8,
//     boot_signature: u8,
//     volume_id: u32,
//     volume_label: [u8; 11],
//     filesystem_type: [u8; 8],
//     boot_code: [u8; 448],
//     boot_sector_signature: u16,
// }
//
// impl BootSector {
//     pub fn from_bytes(data: &[u8; 512]) -> Self {
//         unsafe { core::ptr::read_unaligned(data.as_ptr() as *const BootSector) }
//     }
// }
//
// // Example function to read and parse the boot sector
// fn read_boot_sector(disk: &Disk) {
//     let mut boot_sector_data = [0u8; 512];
//
//     // Read the first sector (sector 0) of the FAT16 partition
//     disk.read(boot_sector_data.as_mut_ptr(), 0, 1);
//
//     // Parse the boot sector
//     let boot_sector = BootSector::from_bytes(&boot_sector_data);
//
//     // Print some information
//     println!(
//         "FAT16 Volume Label: {}",
//         core::str::from_utf8(&boot_sector.volume_label).unwrap_or("Invalid")
//     );
//     println!("Bytes per sector: {}", boot_sector.bytes_per_sector);
//     println!("Sectors per cluster: {}", boot_sector.sectors_per_cluster);
//     println!("Number of FATs: {}", boot_sector.num_fats);
// }
