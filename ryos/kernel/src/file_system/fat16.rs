use crate::file_system::disk_driver::{DiskManager, SECTOR_SIZE};
use crate::file_system::errors::FileSystemError;



#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct FATEntry(u16);

impl FATEntry {
    // Constants for the bit fields
    const TYPE_MASK: u16      = 0b1111_0000_0000_0000;  // First 4 bits for type
    const NEXT_SECTOR_MASK: u16 = 0b0000_1111_1111_1111;  // Last 12 bits for sector number

    // Type values (stored in first 4 bits)
    const TYPE_FREE: u16       = 0b0000_0000_0000_0000;
    const TYPE_EOF: u16        = 0b0001_0000_0000_0000;
    const TYPE_BAD: u16        = 0b0010_0000_0000_0000;
    const TYPE_USED: u16       = 0b0011_0000_0000_0000;

    pub fn new_free() -> Self {
        FATEntry(Self::TYPE_FREE)
    }

    pub fn new_eof() -> Self {
        FATEntry(Self::TYPE_EOF)
    }

    pub fn new_used(next_sector: u16) -> Self {
        // Ensure next_sector fits in 12 bits
        let next = next_sector & Self::NEXT_SECTOR_MASK;
        FATEntry(Self::TYPE_USED | next)
    }

    pub fn get_type(&self) -> u16 {
        self.0 & Self::TYPE_MASK
    }

    pub fn get_next_sector(&self) -> Option<u16> {
        if self.is_used() {
            Some(self.0 & Self::NEXT_SECTOR_MASK)
        } else {
            None
        }
    }

    pub fn is_free(&self) -> bool {
        self.get_type() == Self::TYPE_FREE
    }

    pub fn is_eof(&self) -> bool {
        self.get_type() == Self::TYPE_EOF
    }

    pub fn is_used(&self) -> bool {
        self.get_type() == Self::TYPE_USED
    }

    pub fn is_bad(&self) -> bool {
        self.get_type() == Self::TYPE_BAD
    }
    pub fn as_bin(&self) -> u16 {
        self.0
    }
}

// Example of how the FAT table would use this
#[repr(C, packed)]
pub struct FAT {
    entries: [FATEntry; 256], // Still fits in 512 bytes
}



impl FAT {
    pub fn new() -> Self {
        FAT {
            entries: [FATEntry::new_free(); 256]
        }
    }

    pub fn save(&self, disk_manager: &DiskManager) -> Result<(), FileSystemError>{
        // Save the FAT table to disk
        disk_manager.write(self as *const FAT as *const u8, 1, 1)
    }
    pub fn load(disk_manager: &DiskManager) -> Result<FAT, FileSystemError> {
        // Load the FAT table from disk
        let mut buffer: [u8; SECTOR_SIZE] = [0; SECTOR_SIZE];
        match disk_manager.read(buffer.as_mut_ptr(), 1, 1)
        {
            Ok(()) => { Ok(Self::from_buffer(buffer)) },
            Err(e) => return Err(e),
        }
    }

    fn from_buffer(buffer: [u8; SECTOR_SIZE]) -> Self {
        let mut fat = FAT::new();
        fat.entries = unsafe { *(buffer.as_ptr() as *const [FATEntry; 256]) };
        fat
    }
    pub fn add_entry(&mut self, entry: FATEntry) {
        self.entries[0] = entry;
    }
    pub fn first(&self) -> FATEntry {
        self.entries[0]
    }
}

pub struct FAtApi
{
    table: FAT,
    disk_manager: DiskManager
}

impl FAtApi
{
    pub fn new() -> Self {
        let disk_manager = DiskManager::new();
        disk_manager.check().expect("Error init disk at FATApi");
        FAtApi {
            table: FAT::new(),
            disk_manager
        }
    }
    pub fn save(&self) -> Result<(), FileSystemError> {
        self.table.save(&self.disk_manager)
    }
    pub fn load(&self) -> Result<FAT, FileSystemError> {
        FAT::load(&self.disk_manager)
    }

    pub fn add_entry(&mut self, entry: FATEntry) {
        self.table.add_entry(entry);
    }

    pub fn first(&self) -> FATEntry
    {
        self.table.first()
    }

}