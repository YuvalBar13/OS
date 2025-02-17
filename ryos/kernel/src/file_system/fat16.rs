use crate::file_system::disk_driver::{Disk, SECTOR_SIZE};
use crate::file_system::errors::FileSystemError;
use crate::file_system::errors::FileSystemError::{
    BadSector, FileAlreadyExists, FileNotFound, IndexOutOfBounds, OutOfSpace, UnusedSector,
};
use crate::{change_writer_color, println};
use alloc::string::String;
use alloc::vec::Vec;
use core::ops::ControlFlow::Break;
use spin::Mutex;
use crate::terminal::interface::OUTPUT_COLOR;
use crate::terminal::output::framebuffer::{Color, DEFAULT_COLOR};

const FIRST_USABLE_SECTOR: u16 = 21;

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
    fn new_free() -> Self {
        FATEntry(Self::TYPE_FREE)
    }

    fn new_eof() -> Self {
        FATEntry(Self::TYPE_EOF)
    }

    fn new_used(sector: u16) -> Result<Self, FileSystemError> {
        // Ensure next_sector fits in 12 bits
        if sector + FIRST_USABLE_SECTOR > Self::SECTOR_MASK {
            return Err(BadSector);
        }
        let next = (sector + FIRST_USABLE_SECTOR) & Self::SECTOR_MASK;
        Ok(FATEntry(Self::TYPE_USED | next))
    }

    fn get_type(&self) -> u16 {
        self.0 & Self::TYPE_MASK
    }

    fn get_sector(&self) -> Result<u16, FileSystemError> {
        if self.is_used() {
            Ok(self.0 & Self::SECTOR_MASK)
        } else {
            Err(UnusedSector)
        }
    }

    fn is_free(&self) -> bool {
        self.get_type() == Self::TYPE_FREE
    }

    fn is_eof(&self) -> bool {
        self.get_type() == Self::TYPE_EOF
    }

    fn is_used(&self) -> bool {
        self.get_type() == Self::TYPE_USED
    }

    fn is_bad(&self) -> bool {
        self.get_type() == Self::TYPE_BAD
    }
    fn as_bin(&self) -> u16 {
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
    fn new() -> Self {
        let mut table = FAT {
            entries: [FATEntry::new_free(); SECTOR_SIZE / 2], // each entry is 2 bytes and the whole table is 512 bytes
        };
        table.entries[0] = FATEntry(Self::MAGIC_NUMBER);
        table
    }

    fn is_valid(&self) -> bool {
        self.entries[0].as_bin() == Self::MAGIC_NUMBER
    }
    fn load_or_create(disk_manager: &Disk) -> FAT {
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

    fn save(&self, disk_manager: &Disk) -> Result<(), FileSystemError> {
        // Save the FAT table to disk
        disk_manager.write(
            self as *const FAT as *const u8,
            FIRST_USABLE_SECTOR as u64 - 1,
            1,
        )
    }
    fn load(disk_manager: &Disk) -> Result<FAT, FileSystemError> {
        // Load the FAT table from disk
        let mut buffer: [u8; SECTOR_SIZE] = [0; SECTOR_SIZE];
        match disk_manager.read(buffer.as_mut_ptr(), FIRST_USABLE_SECTOR as u64 - 1, 1) {
            Ok(()) => Ok(Self::from_buffer(buffer)),
            Err(e) => Err(e),
        }
    }

    fn from_buffer(buffer: [u8; SECTOR_SIZE]) -> Self {
        let mut fat = FAT::new();
        fat.entries = unsafe { *(buffer.as_ptr() as *const [FATEntry; 256]) };
        fat
    }
    fn first_free_entry(&self) -> Result<usize, FileSystemError> {
        for i in 1..self.entries.len() {
            if self.entries[i].is_free() {
                return Ok(i);
            }
        }
        Err(OutOfSpace)
    }
    fn add_entry(&mut self, entry: FATEntry) -> Result<(), FileSystemError> {
        match self.first_free_entry() {
            Ok(index) => {
                self.entries[index] = entry;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
    fn remove_entry(&mut self, index: u16) -> Result<(), FileSystemError> {
        self.entries[index as usize] = FATEntry::new_free();
        Ok(())
    }
}

pub struct FAtApi {
    table: FAT,
    disk_manager: Disk,
    directory: Directory,
    allocator: SectorAllocator
}

impl FAtApi {
    pub fn new() -> Self {
        let disk = Disk::new();
        FAtApi {
            table: FAT::load_or_create(&disk),
            directory: Directory::load_or_create_dir(&disk),
            allocator: SectorAllocator::load_or_create(&disk),
            disk_manager: disk,

        }
    }

    pub fn save(&self) -> Result<(), FileSystemError> {
        self.table.save(&self.disk_manager)?;
        self.directory.save(&self.disk_manager)?;
        self.allocator.save(&self.disk_manager)
    }

    pub fn add_entry(&mut self, entry: FATEntry) -> Result<(), FileSystemError> {
        self.table.add_entry(entry)
    }

    pub fn get_entry(&self, entry_index: usize) -> Result<FATEntry, FileSystemError> {
        if entry_index >= self.table.entries.len() || entry_index < 1 {
            return Err(IndexOutOfBounds);
        }
        Ok(self.table.entries[entry_index])
    }

    pub fn get_data(&self, entry_index: usize) -> Result<[u8; SECTOR_SIZE], FileSystemError> {
        let mut buffer: [u8; SECTOR_SIZE] = [0; SECTOR_SIZE];

        let sector = self.get_sector(entry_index)?;
        self.disk_manager
            .read(buffer.as_mut_ptr(), sector as u64, 1)?;
        Ok(buffer)
    }

    pub fn change_data(
        &mut self,
        entry_index: usize,
        buffer: &[u8; SECTOR_SIZE],
    ) -> Result<(), FileSystemError> {
        let sector = self.get_sector(entry_index)?;
        self.disk_manager.write(buffer.as_ptr(), sector as u64, 1)
    }
    pub fn get_sector(&self, entry_index: usize) -> Result<u16, FileSystemError> {
        let entry = self.get_entry(entry_index)?;
        entry.get_sector()
    }

    pub fn new_entry(&mut self, name: &str) -> Result<(), FileSystemError> {
        match self.directory.get_entry(name) {
            Err(_) => {
                let index = self.table.first_free_entry()?;


                let zero = [0; SECTOR_SIZE];
                let sector = self.allocator.get_free_sector();
                self.disk_manager.write(
                    zero.as_ptr(),
                    sector as u64,
                    1,
                )?;

                self.add_entry(FATEntry::new_used(sector)?)?;
                Ok(self
                    .directory
                    .add_entry(DirEntry::new(name, index as u16, FILE_ENTRY_TYPE))?)
            }
            Ok(_) => Err(FileAlreadyExists),
        }
    }


    pub fn list_dir(&self) {
        self.directory.print()
    }

    pub fn index_by_name(&self, name: &str) -> Result<u16, FileSystemError> {
        Ok(self.directory.get_entry(name)?.first_cluster)
    }

    pub fn remove_entry(&mut self, name: &str) -> Result<(), FileSystemError> {
        let entry_index = self.index_by_name(name)?;
        self.allocator.free(self.table.entries[entry_index as usize].get_sector()?);
        self.table.remove_entry(entry_index)?;
        Ok(self.directory.remove_entry(name)) // cant failed cause teh index by name found that there is entry with the name
    }


}

const DIR_ENTRY_TYPE: u8 = 0x10;
const FILE_ENTRY_TYPE: u8 = 0x05;
#[derive(Debug, Clone, Copy)]
#[repr(C)] // Ensures the struct layout is C-compatible (for binary data)
pub struct DirEntry {
    pub filename: [u8; 13], // 8 characters for the filename + 3 for the extension
    pub first_cluster: u16, // 2 bytes for the first cluster
    pub entry_type: u8,
}

impl DirEntry {
    // Create a new directory entry with a filename and first cluster
    fn new(filename: &str, first_cluster: u16, entry_type: u8) -> Self {
        let mut filename_bytes = [0u8; 13];
        let len = filename.len().min(13);
        filename_bytes[..len].copy_from_slice(&filename.as_bytes()[..len]);
        DirEntry {
            filename: filename_bytes,
            first_cluster,
            entry_type

        }
    }
    fn empty() -> Self {
        DirEntry {
            filename: [0u8; 13],
            first_cluster: 0,
            entry_type: FILE_ENTRY_TYPE,
        }
    }
    fn to_string(&self) -> String {
        self.filename
            .iter()
            .take_while(|&&x| x != 0)
            .map(|&x| x as char)
            .collect()
    }
    fn is_empty(&self) -> bool {
        self.filename.iter().all(|&x| x == 0)
    }
}

const FIRST_DIRECTORY: u16 = 0;
const ENTRY_COUNT: usize = 32;
const DIRECTORY_MAGIC: u32 = 0xdead;
#[derive(Debug, Clone, Copy)]

pub struct Directory {
    magic: u32,
    entries: [DirEntry; ENTRY_COUNT * 8 - 1],
}

impl Directory {
    fn new() -> Self {
        Directory {
            magic: DIRECTORY_MAGIC,
            entries: [DirEntry::empty(); ENTRY_COUNT * 8 - 1],
        }
    }
    pub fn load_or_create_dir(disk_manager: &Disk) -> Directory {
        match Directory::load(&disk_manager) {
            Ok(dir) => {
                println!("Directory loaded successfully and is valid.");
                dir
            }
            Err(FileSystemError::InvalidDirectory) => {
                println!("Directory loaded but is invalid, creating a new one");
                let new_dir = Directory::new();
                new_dir
                    .save(disk_manager)
                    .expect("Failed to save new directory");
                new_dir
            }
            Err(_) => panic!("Failed to read directory"),
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
    const DIR_COLOR: Color = Color::new(40, 110, 190);
    fn print(&self) {
        for i in 0..self.entries.len() {
            if !self.entries[i].is_empty() {
                if self.entries[i].entry_type == DIR_ENTRY_TYPE {
                    change_writer_color(Self::DIR_COLOR);
                }
                println!(
                    "{}: {}",
                    self.entries[i].to_string(),
                    self.entries[i].first_cluster,

                );
                change_writer_color(OUTPUT_COLOR);
            }
        }
    }

    fn save(&self, disk_manager: &Disk) -> Result<(), FileSystemError> {
        // Transmute the entire directory structure into a byte slice
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const Directory as *const u8,
                core::mem::size_of::<Directory>(),
            )
        };

        disk_manager.write(bytes.as_ptr(), FIRST_DIRECTORY as u64, 8)
    }

    fn load(disk_manager: &Disk) -> Result<Directory, FileSystemError> {
        let mut buffer = [0u8; core::mem::size_of::<Directory>()];

        disk_manager.read(buffer.as_mut_ptr(), FIRST_DIRECTORY as u64, 8)?;

        let directory = unsafe { core::ptr::read(buffer.as_ptr() as *const Directory) };

        // Validate magic number
        if directory.magic != DIRECTORY_MAGIC {
            return Err(FileSystemError::InvalidDirectory);
        }

        Ok(directory)
    }

    fn get_entry(&self, name: &str) -> Result<(DirEntry), FileSystemError> {
        for i in 0..self.entries.len() {
            if self.entries[i].is_empty() {
                continue;
            }

            if self.entries[i].to_string() == name {
                return Ok(self.entries[i]);
            }
        }
        Err(FileNotFound)
    }

    fn remove_entry(&mut self, name: &str) {
        for i in 0..self.entries.len() {
            if self.entries[i].to_string() == name {
                self.entries[i] = DirEntry::empty();
            }
        }
    }
}

struct SectorAllocator
{
    next_free: u16,
    freed_sectors: Vec<u16>,
}
impl SectorAllocator {
    const MAGIC_SECTOR_NUMBER: u16 = 0x22;
    pub const fn new() -> Self
    {
        SectorAllocator { next_free: FIRST_USABLE_SECTOR, freed_sectors: Vec::new() }
    }
    pub fn get_free_sector(&mut self) -> u16
    {
        if self.freed_sectors.len() > 0
        {
            return self.freed_sectors.pop().unwrap()
        }
        self.next_free += 1;
        self.next_free -1
    }
    pub fn free(&mut self, sector: u16)
    {
        self.freed_sectors.push(sector);
    }

    fn save(&self, disk: &Disk) -> Result<(), FileSystemError>
    {
        let buff = self.to_bitmap();
        disk.write(buff.as_ptr(), FIRST_USABLE_SECTOR as u64 - 2, 1)
    }
    fn to_bitmap(&self) -> [u8; SECTOR_SIZE] {
        let mut buffer = [0u8; SECTOR_SIZE];

        // Store self.next_free (a u16) in the first two bytes
        buffer[0] =  (Self::MAGIC_SECTOR_NUMBER & 0xFF) as u8;
        buffer[1] = ((Self::MAGIC_SECTOR_NUMBER >> 8) & 0xFF) as u8;
        buffer[2] = (self.next_free & 0xFF) as u8;       // Lower byte
        buffer[3] = ((self.next_free >> 8) & 0xFF) as u8; // Upper byte

        // Store the freed_sectors data, treating each u16 as two bytes
        for (i, sector) in self.freed_sectors.iter().enumerate() {
            let offset = 4 + i * 2; // Each u16 takes 2 bytes

            if offset + 1 >= SECTOR_SIZE {
                break; // Prevent out-of-bounds writes
            }

            buffer[offset] = (sector & 0xFF) as u8;       // Lower byte
            buffer[offset + 1] = ((sector >> 8) & 0xFFu16) as u8; // Upper byte
        }
        buffer
    }
    fn from_bitmap(buffer: [u8; SECTOR_SIZE]) -> Result<Self, FileSystemError>
    {
        let mut allocator = SectorAllocator::new();

        if (buffer[1] as u16) << 8 | (buffer[0] as u16) != Self::MAGIC_SECTOR_NUMBER
        {
            return Err(FileSystemError::InvalidSectorAllocator)
        }
        // Restore self.next_free (stored in little-endian)
        allocator.next_free = (buffer[3] as u16) << 8 | (buffer[2] as u16);

        // Restore freed_sectors
        for i in (4..SECTOR_SIZE).step_by(2) {
            if i + 1 >= SECTOR_SIZE {
                break; // Prevent out-of-bounds read
            }

            let sector = (buffer[i + 1] as u16) << 8 | (buffer[i] as u16); // Little-endian
            if sector != 0 {
                allocator.freed_sectors.push(sector);
            }
        }
        Ok(allocator)
    }
    fn load(disk: &Disk) -> Result<Self, FileSystemError>
    {
        let mut tmp: [u8;512] = [0u8; SECTOR_SIZE];
        disk.read(tmp.as_mut_ptr(), FIRST_USABLE_SECTOR as u64 - 2, 1)?;
        Self::from_bitmap(tmp)
    }

    fn load_or_create(disk: &Disk) -> Self
    {
        match Self::load(disk)
        {
            Ok(allocator) => {
                println!("sector allocator found and is valid!");
                return allocator},
            Err(FileSystemError::InvalidSectorAllocator) => {
                println!("sector allocator found but is invalid!");
                return SectorAllocator::new();
            },
            Err(e) => {
                panic!("Error: {:?}", e);
            }
        }
    }
}

