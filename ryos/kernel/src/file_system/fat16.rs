use crate::file_system::disk_driver::{DiskManager, SECTOR_SIZE};
use crate::file_system::errors::FileSystemError;
use crate::file_system::errors::FileSystemError::{BadSector, IndexOutOfBounds, OutOfSpace, UnusedSector};
use crate::println;
const FIRST_USABLE_SECTOR: u16 = 100;

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct FATEntry(u16);

impl FATEntry {
    // Constants for the bit fields
    const TYPE_MASK: u16 = 0b1111_0000_0000_0000; // First 4 bits for type
    const SECTOR_MASK: u16 = 0b0000_1111_1111_1111; // Last 12 bits for sector number

    // Type values (stored in first 4 bits)
    const TYPE_FREE: u16 = 0b0000_0000_0000_0000;
    const TYPE_EOF: u16 = 0b0001_0000_0000_0000;
    const TYPE_BAD: u16 = 0b0010_0000_0000_0000;
    const TYPE_USED: u16 = 0b0011_0000_0000_0000;
    pub fn new_free() -> Self {
        FATEntry(Self::TYPE_FREE)
    }

    pub fn new_eof() -> Self {
        FATEntry(Self::TYPE_EOF)
    }

    pub fn new_used(sector: u16) -> Result<Self, FileSystemError> {
        // Ensure next_sector fits in 12 bits
        if sector > Self::SECTOR_MASK {
            return Err(BadSector)
        }
        let next = sector & Self::SECTOR_MASK;
        Ok(FATEntry(Self::TYPE_USED | next))
    }

    pub fn get_type(&self) -> u16 {
        self.0 & Self::TYPE_MASK
    }

    pub fn get_sector(&self) -> Result<u16, FileSystemError> {
        if self.is_used() {
            Ok(self.0 & Self::SECTOR_MASK)
        } else {
            Err(UnusedSector)
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
    const MAGIC_NUMBER: u16 = 0xF1A7; // Magic number for FAT table(if the first entry is this the fat is initialized)
    pub fn new() -> Self {
        let mut table = FAT {
            entries: [FATEntry::new_free(); SECTOR_SIZE / 2], // each entry is 2 bytes and the whole table is 512 bytes
        };
        table.entries[0] = FATEntry(Self::MAGIC_NUMBER);
        table
    }

    pub fn is_valid(&self) -> bool {
        self.entries[0].as_bin() == Self::MAGIC_NUMBER
    }
    pub fn load_or_create(disk_manager: &DiskManager) -> FAT {
        match FAT::load(disk_manager) {
            Ok(fat) if fat.is_valid() => {
                println!("FAT loaded successfully and is valid.");
                fat // Return the valid FAT
            }
            Ok(_) => {
                println!("FAT loaded but is invalid, creating a new one.");
                FAT::new() // If FAT is invalid, create a new one
            }
            Err(_) => {
                println!("Error: Disk failed to load FAT.");
                panic!("Error, Disk doesn't work");
            }
        }
    }

    pub fn save(&self, disk_manager: &DiskManager) -> Result<(), FileSystemError> {
        // Save the FAT table to disk
        disk_manager.write(self as *const FAT as *const u8, FIRST_USABLE_SECTOR as u64 - 1, 1)
    }
    pub fn load(disk_manager: &DiskManager) -> Result<FAT, FileSystemError> {
        // Load the FAT table from disk
        let mut buffer: [u8; SECTOR_SIZE] = [0; SECTOR_SIZE];
        match disk_manager.read(buffer.as_mut_ptr(), FIRST_USABLE_SECTOR  as u64 - 1, 1) {
            Ok(()) => Ok(Self::from_buffer(buffer)),
            Err(e) => Err(e),
        }
    }

    fn from_buffer(buffer: [u8; SECTOR_SIZE]) -> Self {
        let mut fat = FAT::new();
        fat.entries = unsafe { *(buffer.as_ptr() as *const [FATEntry; 256]) };
        fat
    }
    pub fn first_free_entry(&self) -> Result<usize, FileSystemError> {
        for i in 1..self.entries.len() {
            if self.entries[i].is_free() {
                return Ok(i);
            }
        }
        Err(OutOfSpace)
    }
    pub fn add_entry(&mut self, entry: FATEntry) -> Result<(), FileSystemError> {
        match self.first_free_entry() {
            Ok(index) => {
                self.entries[index] = entry;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub fn get_entry(&self, sector: u64) -> FATEntry {
        self.entries[sector as usize]
    }
}

pub struct FAtApi {
    table: FAT,
    disk_manager: DiskManager,
}

impl FAtApi {
    pub fn new() -> Self {
        let disk_manager = DiskManager::new();
        disk_manager.check().expect("Error init disk at FATApi");
        FAtApi {
            table: FAT::load_or_create(&disk_manager),
            disk_manager,
        }
    }
    pub fn save(&self) -> Result<(), FileSystemError> {
        self.table.save(&self.disk_manager)
    }

    pub fn add_entry(&mut self, entry: FATEntry) -> Result<(), FileSystemError> {
        self.table.add_entry(entry)
    }

    pub fn get_entry(&self, entry_index: usize) -> Result<FATEntry, FileSystemError> {
        if entry_index >= self.table.entries.len() || entry_index < 1 {
            return Err(IndexOutOfBounds);
        }
        Ok(self.table.entries[entry_index ])
    }

    pub fn get_data(&self, entry_index: usize) -> Result<[u8; SECTOR_SIZE], FileSystemError> {
        let mut buffer: [u8; SECTOR_SIZE] = [0; SECTOR_SIZE];

        let sector = self.get_sector(entry_index)?;
        self.disk_manager.read(buffer.as_mut_ptr(), sector as u64, 1)?;
        Ok(buffer)

    }

    pub fn change_data(&mut self, entry_index: usize, buffer: &[u8; SECTOR_SIZE]) -> Result<(), FileSystemError> {
        let sector = self.get_sector(entry_index)?;
        self.disk_manager.write(buffer.as_ptr(), sector as u64, 1)
    }
    pub fn get_sector(&self, entry_index: usize) -> Result<u16, FileSystemError> {
        let entry = self.get_entry(entry_index)?;
        entry.get_sector()
    }

    pub fn new_entry(&mut self, buffer: &[u8; SECTOR_SIZE]) -> Result<(), FileSystemError> {
        let index = self.table.first_free_entry()?;
        let mut sector: u16 = FIRST_USABLE_SECTOR;
        if(index != 1) { // for the first entry
            sector = self.get_sector(index - 1)?;

        }

        // when adding clusters should change the logic here
        self.disk_manager.write(buffer.as_ptr(), sector as u64 + 1, 1)?;
        self.add_entry(FATEntry::new_used(sector + 1)?)
    }
}
