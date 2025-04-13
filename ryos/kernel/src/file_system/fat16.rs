use crate::file_system::disk_driver::{Disk, SECTOR_SIZE};
use crate::file_system::errors::FileSystemError;
use crate::file_system::errors::FileSystemError::{
    BadSector, DirAlreadyExists, DirectoryNotFound, FileAlreadyExists, FileNotFound,
    IndexOutOfBounds, OutOfSpace, UnusedSector,
};
use crate::terminal::interface::{OUTPUT_COLOR, WORKING_DIR};
use crate::terminal::output::framebuffer::{Color, DEFAULT_COLOR};
use crate::{change_writer_color, eprintln, print, println};
use alloc::string::String;
use alloc::vec::Vec;
use core::ops::ControlFlow::Break;
use spin::Mutex;

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
        if sector > Self::SECTOR_MASK {
            return Err(BadSector);
        }
        let next = sector & Self::SECTOR_MASK;
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
        match FAT::load(disk_manager, None) {
            Ok(fat) if fat.is_valid() => {
                println!("FAT loaded successfully and is valid.");
                fat // Return the valid FAT
            }
            Ok(_) => {
                println!("FAT loaded but is invalid, creating a new one.");
                let mut fat = FAT::new(); // If FAT is invalid, create a new one
                fat.save(disk_manager, None).expect("Error saving fat");
                fat
            }
            Err(_) => {
                println!("Error: Disk failed to load FAT.");
                panic!("Error, Disk doesn't work");
            }
        }
    }

    /*
    this function save the current state of the FAT table to the hard disk
    params: disk manager - driver for write into the disk,
     sector - when None the current Fat is the main so save it on const place on the disk,
     when some it's the sector where should the Fat saved on
     */
    fn save(&self, disk_manager: &Disk, sector: Option<u16>) -> Result<(), FileSystemError> {
        if sector.is_none() {
            return disk_manager.write(
                self as *const FAT as *const u8,
                FIRST_USABLE_SECTOR as u64 - 1,
                1,
            );
        }
        disk_manager.write(self as *const FAT as *const u8, sector.unwrap() as u64, 1)
    }

    fn load(disk_manager: &Disk, sector: Option<u16>) -> Result<FAT, FileSystemError> {
        let mut buffer: [u8; SECTOR_SIZE] = [0; SECTOR_SIZE];
        if sector.is_none() {
            return match disk_manager.read(buffer.as_mut_ptr(), FIRST_USABLE_SECTOR as u64 - 1, 1) {
                Ok(()) => Ok(Self::from_buffer(buffer)),
                Err(e) => Err(e),
            };
        }
        match disk_manager.read(buffer.as_mut_ptr(), sector.unwrap() as u64, 1) {
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
    allocator: SectorAllocator,
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

    pub fn get_data(&self, file_name: &str) -> Result<[u8; SECTOR_SIZE], FileSystemError> {
        let mut buffer: [u8; SECTOR_SIZE] = [0; SECTOR_SIZE];
        let dir = self.get_current_directory()?;
        let fat = self.get_current_fat(&dir.0)?;
        let entry = dir.0.get_entry(file_name)?;

        if entry.entry_type == DIR_ENTRY_TYPE {
            return Err(FileSystemError::NotAFile);
        }
        self.disk_manager.read(
            buffer.as_mut_ptr(),
            fat.entries[entry.first_cluster as usize].get_sector()? as u64,
            1,
        )?;
        Ok(buffer)
    }

    pub fn change_data(
        &mut self,
        file_name: &str,
        buffer: &[u8; SECTOR_SIZE],
    ) -> Result<(), FileSystemError> {
        let dir = self.get_current_directory()?;
        let fat = self.get_current_fat(&dir.0)?;
        let entry = dir.0.get_entry(file_name)?;
        if entry.entry_type == DIR_ENTRY_TYPE {
            return Err(FileSystemError::NotAFile);
        }

        self.disk_manager.write(
            buffer.as_ptr(),
            fat.entries[entry.first_cluster as usize].get_sector()? as u64,
            1,
        )?;
        Ok(())
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
                self.disk_manager.write(zero.as_ptr(), sector as u64, 1)?;

                self.add_entry(FATEntry::new_used(sector)?)?;
                Ok(self
                    .directory
                    .add_entry(DirEntry::new(name, index as u16, FILE_ENTRY_TYPE))?)
            }
            Ok(_) => Err(FileAlreadyExists),
        }
    }

    /*
    this function return the directory with the name 'name', the function will search for the directory at the 'directory' path
     */
    fn get_directory_table_by_name(
        &self,
        directory: &Directory,
        name: &str,
    ) -> Result<(Directory, u16), FileSystemError> {
        let entry = directory.get_entry(name)?;
        if entry.entry_type != DIR_ENTRY_TYPE {
            return Err(DirectoryNotFound);
        }
        Ok((
            Directory::load(&self.disk_manager, Some(entry.first_cluster))?,
            entry.first_cluster,
        ))
    }

    pub fn search_directory(&self, name: &str) -> Result<bool, FileSystemError> {
        let dir = self.get_current_directory()?.0.get_entry(name);
        match dir {
            Ok(dir) => {
                if dir.entry_type == DIR_ENTRY_TYPE {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(FileSystemError::FileNotFound) => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn get_current_directory(&self) -> Result<(Directory, u16), FileSystemError> {
        let parts: Vec<String> = WORKING_DIR
            .lock()
            .split('/')
            .map(String::from)
            .filter(|s| !s.is_empty())
            .collect();

        let mut last_dir = (Directory::load(&self.disk_manager, None)?, FIRST_DIRECTORY);
        for dir_name in parts {
            last_dir = self.get_directory_table_by_name(&last_dir.0, dir_name.as_str())?;
        }

        Ok(last_dir)
    }
    fn get_current_fat(&self, directory: &Directory) -> Result<FAT, FileSystemError> {
        FAT::load(&self.disk_manager, Some(directory.fat_sector))
    }
    fn get_parent_sector(&self) -> Result<u16, FileSystemError> {
        let mut parts: Vec<String> = WORKING_DIR
            .lock()
            .split('/')
            .map(String::from)
            .filter(|s| !s.is_empty())
            .collect();

        if parts.len() == 0 {
            return Ok(FIRST_DIRECTORY);
        }
        let current = parts.pop().unwrap(); // remove the current dir
        let mut last_dir = (Directory::load(&self.disk_manager, None)?, FIRST_DIRECTORY).0;
        for dir_name in parts {
            last_dir = self
                .get_directory_table_by_name(&last_dir, dir_name.as_str())?
                .0;
        }
        Ok(last_dir.get_entry(current.as_str())?.first_cluster)
    }
    pub fn add_file(&mut self, name: &str) -> Result<(), FileSystemError> {
        let mut dir = self.get_current_directory()?;
        match dir.0.get_entry(name) {
            Err(_) => {
                let mut fat = self.get_current_fat(&dir.0)?;

                let index = fat.first_free_entry()?;

                let zero = [0u8; SECTOR_SIZE];
                let sector = self.allocator.get_free_sector();
                self.allocator.save(&self.disk_manager)?;
                self.disk_manager.write(zero.as_ptr(), sector as u64, 1)?;

                fat.add_entry(FATEntry::new_used(sector)?)?;

                dir.0
                    .add_entry(DirEntry::new(name, index as u16, FILE_ENTRY_TYPE))?;

                fat.save(&self.disk_manager, Some(dir.0.fat_sector))?;
                dir.0.save(&self.disk_manager, Some(dir.1))?;
                Ok(())
            }
            Ok(_) => Err(FileAlreadyExists),
        }
    }

    // this function creates new dir and making a sub dirs of '.' and '..'
    pub fn new_dir(&mut self, name: &str) -> Result<(), FileSystemError> {
        if self.get_current_directory()?.0.get_entry(name).is_ok() {
            return Err(DirAlreadyExists);
        }
        let fat_sector = self.allocator.get_free_sectors(9);
        self.allocator.save(&self.disk_manager)?;
        let mut fat = FAT::new();

        let dir_sector = fat_sector + 1;
        let mut dir = Directory::new(fat_sector);
        dir.fat_sector = fat_sector;
        dir.add_entry(DirEntry::new(".", dir_sector, DIR_ENTRY_TYPE))?;
        dir.add_entry(DirEntry::new(
            "..",
            self.get_parent_sector()?,
            DIR_ENTRY_TYPE,
        ))?;

        fat.save(&self.disk_manager, Some(fat_sector))?;

        dir.save(&self.disk_manager, Some(dir_sector))?;
        let mut parent = self.get_current_directory()?;
        parent
            .0
            .add_entry(DirEntry::new(name, dir_sector, DIR_ENTRY_TYPE))?;
        parent.0.save(&self.disk_manager, Some(parent.1))
    }

    pub fn list_dir(&self) {
        self.get_current_directory().unwrap().0.print();
    }

    pub fn index_by_name(&self, name: &str) -> Result<u16, FileSystemError> {
        Ok(self.directory.get_entry(name)?.first_cluster)
    }

    fn remove_file_by_name(
        &mut self,
        name: &str,
        directory: &mut (Directory, u16),
    ) -> Result<(), FileSystemError> {
        let mut entry = directory.0.get_entry(name)?;
        if entry.entry_type == FILE_ENTRY_TYPE {
            let fat_index = entry.first_cluster;
            let mut fat = self.get_current_fat(&directory.0)?;
            let fat_entry = fat.entries[fat_index as usize];
            self.allocator.free(fat_entry.get_sector()?);
            fat.remove_entry(fat_index)?;
            fat.save(&self.disk_manager, Some(directory.0.fat_sector))?;
            directory.0.remove_entry(name);
            directory.0.save(&self.disk_manager, Some(directory.1))?;
            Ok(())
        } else {
            Err(FileSystemError::NotAFile)
        }
    }
    fn remove_dir_by_name(
        &mut self,
        name: &str,
        directory: &mut (Directory, u16),
    ) -> Result<(), FileSystemError> {
        println!("test");
        let mut entry_index: usize = 0;
        for (index, entry) in directory.0.entries.iter_mut().enumerate() {
            if entry.to_string() == name {
                entry_index = index;
                break;
            }
        }
        if !directory.0.entries[entry_index].entry_type == DIR_ENTRY_TYPE {
            return Err(FileSystemError::NotADirectory);
        }
        let mut dir = self.get_directory_table_by_name(&directory.0, name)?;
        let mut fat = self.get_current_fat(&dir.0)?;
        for fat_entry in &mut fat.entries {
            if fat_entry.is_used() {
                self.allocator.free(fat_entry.get_sector()?);
                *fat_entry = FATEntry::new_free();
            }
        }
        for entry in dir.0.entries {
            if entry.entry_type == DIR_ENTRY_TYPE && entry.to_string() != "." && entry.to_string() != ".." {
                self.remove_dir_by_name(&entry.to_string(), &mut dir)?
            }
        }
        fat.entries[0] = FATEntry::new_free();
        fat.save(&self.disk_manager, Some(dir.0.fat_sector))?;
        self.allocator.free(dir.0.fat_sector);
        dir.0.magic = 0;
        dir.0.save(&self.disk_manager, Some(dir.1))?;
        directory.0.entries[entry_index] = DirEntry::empty();
        directory.0.save(&self.disk_manager, Some(directory.1))?;
        self.allocator.free_directory(dir.1);

        Ok(())
    }
    pub fn remove_entry(&mut self, name: &str) -> Result<(), FileSystemError> {
        let mut curr_dir = self.get_current_directory()?;
        return match self.remove_file_by_name(name, &mut curr_dir) {
            Ok(_) => Ok(()),
            Err(FileSystemError::NotAFile) => self.remove_dir_by_name(name, &mut curr_dir),
            Err(e) => Err(e),
        };
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
            entry_type,
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
    fat_sector: u16,
    entries: [DirEntry; ENTRY_COUNT * 8 - 3],
}

impl Directory {
    fn new(fat_sector: u16) -> Self {
        Directory {
            magic: DIRECTORY_MAGIC,
            entries: [DirEntry::empty(); ENTRY_COUNT * 8 - 3],
            fat_sector,
        }
    }
    fn get_entries(&self) -> &[DirEntry] {
        &self.entries
    }
    pub fn load_or_create_dir(disk_manager: &Disk) -> Directory {
        match Directory::load(&disk_manager, None) {
            Ok(dir) => {
                println!("Directory loaded successfully and is valid.");
                dir
            }
            Err(FileSystemError::InvalidDirectory) => {
                println!("Directory loaded but is invalid, creating a new one");
                let mut new_dir = Directory::new(FIRST_DIRECTORY);
                new_dir
                    .save(disk_manager, None)
                    .expect("Error saving to disk");
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
                    self.entries[i].first_cluster
                );
                change_writer_color(OUTPUT_COLOR);
            }
        }
    }

    fn save(&self, disk_manager: &Disk, sector: Option<u16>) -> Result<(), FileSystemError> {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const Directory as *const u8,
                core::mem::size_of::<Directory>(),
            )
        };

        if sector.is_none() {
            return disk_manager.write(bytes.as_ptr(), FIRST_DIRECTORY as u64, 8);
        } else {
            disk_manager.write(bytes.as_ptr(), sector.unwrap() as u64, 8)
        }
    }

    fn load(disk_manager: &Disk, sector: Option<u16>) -> Result<Directory, FileSystemError> {
        let mut buffer = [0u8; core::mem::size_of::<Directory>()];
        if sector.is_none() {
            disk_manager.read(buffer.as_mut_ptr(), FIRST_DIRECTORY as u64, 8)?;
        } else {
            disk_manager.read(buffer.as_mut_ptr(), sector.unwrap() as u64, 8)?;
        }
        let mut directory = unsafe { core::ptr::read(buffer.as_ptr() as *const Directory) };

        // Validate magic number
        if directory.magic != DIRECTORY_MAGIC {
            return Err(FileSystemError::InvalidDirectory);
        }
        if sector.is_none() {
            directory.fat_sector = FIRST_USABLE_SECTOR - 1;
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

struct SectorAllocator {
    next_free: u16,
    freed_sectors: Vec<u16>,
}
impl SectorAllocator {
    const MAGIC_SECTOR_NUMBER: u16 = 0x22;
    pub const fn new() -> Self {
        SectorAllocator {
            next_free: FIRST_USABLE_SECTOR,
            freed_sectors: Vec::new(),
        }
    }
    pub fn get_free_sector(&mut self) -> u16 {
        if self.freed_sectors.len() > 0 {
            return self.freed_sectors.pop().unwrap();
        }
        self.get_free_sectors(1)
    }

    fn get_free_sectors(&mut self, count: u16) -> u16 {
        self.next_free += count;
        self.next_free - count
    }
    pub fn free(&mut self, sector: u16) {
        self.freed_sectors.push(sector);
    }
    fn free_directory(&mut self, sector: u16) {
        let last = self.freed_sectors.len();
        for offset in 0..8 {
            self.freed_sectors.push(sector + offset);
        }
    }
    fn save(&self, disk: &Disk) -> Result<(), FileSystemError> {
        let buff = self.to_bitmap();
        disk.write(buff.as_ptr(), FIRST_USABLE_SECTOR as u64 - 2, 1)
    }
    fn to_bitmap(&self) -> [u8; SECTOR_SIZE] {
        let mut buffer = [0u8; SECTOR_SIZE];

        // Store self.next_free (a u16) in the first two bytes
        buffer[0] = (Self::MAGIC_SECTOR_NUMBER & 0xFF) as u8;
        buffer[1] = ((Self::MAGIC_SECTOR_NUMBER >> 8) & 0xFF) as u8;
        buffer[2] = (self.next_free & 0xFF) as u8; // Lower byte
        buffer[3] = ((self.next_free >> 8) & 0xFF) as u8; // Upper byte

        // Store the freed_sectors data, treating each u16 as two bytes
        for (i, sector) in self.freed_sectors.iter().enumerate() {
            let offset = 4 + i * 2; // Each u16 takes 2 bytes

            if offset + 1 >= SECTOR_SIZE {
                break; // Prevent out-of-bounds writes
            }

            buffer[offset] = (sector & 0xFF) as u8; // Lower byte
            buffer[offset + 1] = ((sector >> 8) & 0xFFu16) as u8; // Upper byte
        }
        buffer
    }
    fn from_bitmap(buffer: [u8; SECTOR_SIZE]) -> Result<Self, FileSystemError> {
        let mut allocator = SectorAllocator::new();

        if (buffer[1] as u16) << 8 | (buffer[0] as u16) != Self::MAGIC_SECTOR_NUMBER {
            return Err(FileSystemError::InvalidSectorAllocator);
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
    fn load(disk: &Disk) -> Result<Self, FileSystemError> {
        let mut tmp: [u8; 512] = [0u8; SECTOR_SIZE];
        disk.read(tmp.as_mut_ptr(), FIRST_USABLE_SECTOR as u64 - 2, 1)?;
        Self::from_bitmap(tmp)
    }

    fn load_or_create(disk: &Disk) -> Self {
        match Self::load(disk) {
            Ok(allocator) => {
                println!("sector allocator found and is valid!");
                return allocator;
            }
            Err(FileSystemError::InvalidSectorAllocator) => {
                println!("sector allocator found but is invalid!");
                let allocator = SectorAllocator::new();
                allocator.save(disk).expect("Error saving to disk");
                return allocator;
            }
            Err(e) => {
                panic!("Error: {:?}", e);
            }
        }
    }
}
