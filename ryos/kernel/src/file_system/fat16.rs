use alloc::string::String;
use crate::file_system::disk_driver::{DiskManager, SECTOR_SIZE};
use crate::file_system::errors::FileSystemError;
use crate::file_system::errors::FileSystemError::{BadSector, FileAlreadyExists, FileNotFound, IndexOutOfBounds, OutOfSpace, UnusedSector};
use crate::println;
const FIRST_USABLE_SECTOR: u16 = 120;

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
        if sector + FIRST_USABLE_SECTOR > Self::SECTOR_MASK {
            return Err(BadSector)
        }
        let next = (sector + FIRST_USABLE_SECTOR) & Self::SECTOR_MASK;
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
    directory: Directory
}

impl FAtApi {
    pub fn new() -> Self {
        let disk_manager = DiskManager::new();
        disk_manager.check().expect("Error init disk at FATApi");
        FAtApi {
            table: FAT::load_or_create(&disk_manager),
            disk_manager,
            directory: Directory::new()
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

    pub fn new_entry(&mut self, name: &str) -> Result<(), FileSystemError> {
        match self.directory.get_entry(name)
        {
            Err(_) => {
                let index = self.table.first_free_entry()?;
                let mut sector: u16 = 0;
                if(index != 1) { // for the first entry
                    sector = self.get_sector(index - 1)?;

                }

                let zero = [0; SECTOR_SIZE];
                self.disk_manager.write(zero.as_ptr(), sector as u64 + 1 + FIRST_USABLE_SECTOR as u64, 1)?;

                self.add_entry(FATEntry::new_used(sector + 1)?)?;
                Ok(self.directory.add_entry(DirEntry::new(name, index as u16))?)
            }
            Ok(_) => {
                Err(FileAlreadyExists)
            }

        }

    }
    pub fn save_dir(&self) -> Result<(), FileSystemError> {
        self.directory.save(&self.disk_manager)
    }
    pub fn load_dir(&self) -> Result<Directory, FileSystemError> {
        Directory::load(&self.disk_manager)
    }

    pub fn list_dir(&self)  {
        self.directory.print()
    }

    pub fn index_by_name(&self, name: &str) -> Result<u16, FileSystemError> {
        Ok(self.directory.get_entry(name)?.first_cluster)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)] // Ensures the struct layout is C-compatible (for binary data)
pub struct DirEntry {
    pub filename: [u8; 11], // 8 characters for the filename + 3 for the extension
    pub first_cluster: u16,  // 2 bytes for the first cluster
}

impl DirEntry {
    // Create a new directory entry with a filename and first cluster
    pub fn new(filename: &str, first_cluster: u16) -> Self {
        let mut filename_bytes = [0u8; 11];

        // Ensure the filename fits into 8.3 format
        let name = &filename[0..8.min(filename.len())];  // Max 8 characters for the name part
        let extension = if filename.len() > 8 {
            &filename[8..11.min(filename.len())] // Max 3 characters for the extension part
        } else {
            ""
        };

        // Copy the name part (0-7) to the filename
        filename_bytes[..name.len()].copy_from_slice(name.as_bytes());

        // Copy the extension part (8-10) to the filename
        filename_bytes[8..8 + extension.len()].copy_from_slice(extension.as_bytes());

        DirEntry {
            filename: filename_bytes,
            first_cluster,
        }
    }
    pub fn empty() -> Self {
        DirEntry {
            filename: [0u8; 11],
            first_cluster: 0,
        }
    }
    pub fn to_string(&self) -> String {
        self.filename.iter()
            .take_while(|&&x| x != 0)
            .map(|&x| x as char)
            .collect()
    }
    pub fn is_empty(&self) -> bool {
        self.filename.iter().all(|&x| x == 0)
    }
}

const FIRST_DIRECTORY: u16 = 100;
const ENTRY_COUNT: usize = 39; // each sector contains 39 entries so 5 sectors fit 196
#[derive(Debug, Clone, Copy)]

pub(crate) struct Directory {
    entries: [DirEntry; ENTRY_COUNT],
}

impl Directory {
    pub fn new() -> Self {
        Directory {
            entries: [DirEntry::empty(); ENTRY_COUNT],
        }
    }
    pub fn add_entry(&mut self, entry: DirEntry) -> Result<(), FileSystemError> {
        for i in 0..self.entries.len() {
            if self.entries[i].is_empty() {
                self.entries[i] = entry;
                return Ok(());
            }
        }
        Err(OutOfSpace)
    }

    pub fn print(&self) {
        for i in 0..self.entries.len() {
            if !self.entries[i].is_empty() {
                println!("{}: {}", self.entries[i].to_string(), self.entries[i].first_cluster);
            }
        }
    }

    pub fn save(&self, disk_manager: &DiskManager) -> Result<(), FileSystemError> {
        disk_manager.write(self as *const Directory as *const u8, FIRST_DIRECTORY as u64, 1)
    }
    pub fn load(disk_manager: &DiskManager) -> Result<Directory, FileSystemError> {
        let mut buffer = [0u8; ENTRY_COUNT];
        disk_manager.read(buffer.as_mut_ptr(), FIRST_DIRECTORY as u64, 1)?;
        Ok(Self::from_bytes(buffer))
    }

    fn from_bytes(bytes: [u8; ENTRY_COUNT]) -> Directory {
        let mut directory = Directory::new();
        directory.entries = unsafe {*(bytes.as_ptr() as *const [DirEntry; ENTRY_COUNT])};
        directory
    }

    fn get_entry(&self, name: &str) -> Result<(DirEntry), FileSystemError> {
        for i in 0..self.entries.len() {
            if self.entries[i].is_empty() {
                continue;
            }

            if self.entries[i].to_string() == name {
                return Ok(self.entries[i])
            }
        }
        Err(FileNotFound)
    }

}