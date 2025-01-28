#[derive(Debug)]
pub enum FileSystemError {
    FileNotFound,
    DirectoryNotFound,
    AccessDenied,
    DiskNotAvailable,
}