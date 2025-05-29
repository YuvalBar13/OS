# RustOS – A Minimal Operating System in Rust

RustOS is a small, custom operating system built in Rust. It is designed as an educational project and includes core components commonly found in operating systems.

## Features

- Interrupt handling  
- File system  
- Terminal interface  
- Task scheduling  
- Input/Output (I/O) operations  
- Memory management  
- Heap allocation  

---

## Usage Instructions

The system runs in a single terminal screen with no graphical interface.  
Upon boot, the system displays the OS logo and opens a command-line terminal where users can interact using a keyboard.

### Available Commands

- `shutdown`: Shut down the machine  
- `reboot`: Restart the machine  
- `echo`: Print the entered text  
- `clear`: Clear the terminal screen  
- `help`: Display a list of available commands  
- `logo`: Clear the screen and show the system logo  
- `cat`: Display file contents  
- `write`: Overwrite content of an existing file  
- `append`: Add content to the end of a file  
- `ls`: List contents of the current directory  
- `touch`: Create a new file  
- `mkdir`: Create a new directory  
- `rm`: Delete a file or directory  
- `cd`: Change the current directory  

After each command, background operations like disk access or output are performed.  
If there is no red error message, the operation succeeded.  
Note: File and directory operations will fail with an error message if the target does not exist—no automatic creation is performed.

---

## Installation Guide

### Requirements

- Linux system (Ubuntu recommended)  
- Rust (nightly toolchain)  
- QEMU

### Installation Steps (Ubuntu)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh

# Add Rust to the environment
source $HOME/.cargo/env

# Install Rust nightly
rustup install nightly

# Set nightly as the default
rustup default nightly

# Install QEMU and related tools
sudo apt update
sudo apt install qemu
sudo apt install libvirt-bin qemu-utils
