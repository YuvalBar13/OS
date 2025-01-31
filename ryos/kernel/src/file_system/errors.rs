#[derive(Debug)]
pub enum FileSystemError {
    FileNotFound,
    DirectoryNotFound,
    AccessDenied,
    OutOfSpace,
    IndexOutOfBounds,
    UnusedSector,
    DiskNotAvailable,
    BadSector,
    FileAlreadyExists,
    InvalidDirectory,
}
