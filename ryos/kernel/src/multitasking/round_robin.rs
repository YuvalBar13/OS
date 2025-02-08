use alloc::boxed::Box;
use alloc::vec::Vec;
use core::arch::asm;
use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::interrupts;

#[derive(Debug, Clone)]
pub struct CpuState {
    // General Purpose Registers
    pub rax: u64, // Accumulator
    pub rbx: u64, // Base
    pub rcx: u64, // Counter
    pub rdx: u64, // Data
    pub rsi: u64, // Source Index
    pub rdi: u64, // Destination Index
    pub rbp: u64, // Base Pointer
    pub rsp: u64, // Stack Pointer

    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,

    // Flags Register
    pub rflags: u64, // Program Status and Control

    // Segment Registers
    pub cs: u16, // Code Segment
    pub ss: u16, // Stack Segment
    pub ds: u16, // Data Segment
    pub es: u16, // Extra Segment
    pub fs: u16, // General Purpose Segment
    pub gs: u16, // General Purpose Segment
}

impl CpuState {
    pub fn new() -> Self {
        CpuState {
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            rbp: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rsp: 0,
            rflags: 0,
            cs: 0,
            ss: 0,
            ds: 0,
            es: 0,
            fs: 0,
            gs: 0,
        }
    }

    pub fn load(&self) {
        unsafe {
            asm!("mov rax, {}", in(reg) self.rax);
            asm!("mov rbx, {}", in(reg) self.rbx);
            asm!("mov rcx, {}", in(reg) self.rcx);
            asm!("mov rdx, {}", in(reg) self.rdx);
            asm!("mov rsi, {}", in(reg) self.rsi);
            asm!("mov rdi, {}", in(reg) self.rdi);
            asm!("mov rbp, {}", in(reg) self.rbp);
            asm!("mov rsp, {}", in(reg) self.rsp);
            asm!("mov r8, {}", in(reg) self.r8);
            asm!("mov r9, {}", in(reg) self.r9);
            asm!("mov r10, {}", in(reg) self.r10);
            asm!("mov r11, {}", in(reg) self.r11);
            asm!("mov r12, {}", in(reg) self.r12);
            asm!("mov r13, {}", in(reg) self.r13);
            asm!("mov r14, {}", in(reg) self.r14);
            asm!("mov r15, {}", in(reg) self.r15);
            asm!("push {}", in(reg) self.rflags);
            asm!("popfq");
            asm!("mov cs, {}", in(reg) self.cs);
            asm!("mov ss, {}", in(reg) self.ss);
            asm!("mov ds, {}", in(reg) self.ds);
            asm!("mov es, {}", in(reg) self.es);
            asm!("mov fs, {}", in(reg) self.fs);
            asm!("mov gs, {}", in(reg) self.gs);
        }
    }

    pub fn save(&mut self) {
        unsafe {
            // Save general-purpose registers
            asm!("mov {}, rax", out(reg) self.rax);
            asm!("mov {}, rbx", out(reg) self.rbx);
            asm!("mov {}, rcx", out(reg) self.rcx);
            asm!("mov {}, rdx", out(reg) self.rdx);
            asm!("mov {}, rsi", out(reg) self.rsi);
            asm!("mov {}, rdi", out(reg) self.rdi);
            asm!("mov {}, rbp", out(reg) self.rbp);
            asm!("mov {}, rsp", out(reg) self.rsp);
            asm!("mov {}, r8", out(reg) self.r8);
            asm!("mov {}, r9", out(reg) self.r9);
            asm!("mov {}, r10", out(reg) self.r10);
            asm!("mov {}, r11", out(reg) self.r11);
            asm!("mov {}, r12", out(reg) self.r12);
            asm!("mov {}, r13", out(reg) self.r13);
            asm!("mov {}, r14", out(reg) self.r14);
            asm!("mov {}, r15", out(reg) self.r15);

            // Save RFLAGS
            asm!("pushfq");
            asm!("pop {}", out(reg) self.rflags);

            // Save segment registers
            asm!("mov {}, cs", out(reg) self.cs);
            asm!("mov {}, ss", out(reg) self.ss);
            asm!("mov {}, ds", out(reg) self.ds);
            asm!("mov {}, es", out(reg) self.es);
            asm!("mov {}, fs", out(reg) self.fs);
            asm!("mov {}, gs", out(reg) self.gs);
        }
    }
}

const STACK_SIZE: usize = 4096;
#[repr(align(16))] // Ensure 16-byte alignment
pub struct Task {
    cpu_state: CpuState,
    stack: Box<[u64; STACK_SIZE]>,
}

impl Task {
    pub fn new(func: extern "C" fn()) -> Self {
        let mut task = Task {
            cpu_state: CpuState::new(),
            stack: Box::new([0; STACK_SIZE])
        };

        // Setup initial stack for task entry
        let stack_top = task.stack.len() - 1;
        task.stack[stack_top] = func as u64;  // Task entry point
        task.stack[stack_top - 1] = remove_task as u64;  // Return address

        // Set initial stack pointer to point to this location
        task.cpu_state.rsp = (task.stack[stack_top - 1]);

        task
    }
    pub fn switch_stack(&mut self) {
        self.cpu_state.rsp = self.stack.as_ptr() as u64;
    }
}

pub struct TaskManager {
    tasks: Vec<Task>,
    current_task: u32,
    switching: AtomicBool,
    delete: Option<u32>,
}

impl TaskManager {
    pub fn new() -> Self {
        TaskManager {
            tasks: Vec::new(),
            current_task: 0,
            switching: AtomicBool::new(false),
            delete: None,
        }
    }

    fn delete_current(&mut self) {
        self.delete = Some(self.current_task);
    }
    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    pub fn schedule(&mut self) {
        if self.tasks.len() <= 0 {
            return;  // Need at least two tasks to switch
        }

        // Use a more robust synchronization mechanism
        if self.switching.load(Ordering::Acquire) {
            return;
        }

        self.switching.store(true, Ordering::Release);

        interrupts::without_interrupts(|| {
            self.switch_context();
        });

        if let Some(delete_index) = self.delete.take() {
            if delete_index < self.tasks.len() as u32 {
                self.tasks.remove(delete_index as usize);

                // Adjust current task index if necessary
                if self.current_task >= self.tasks.len() as u32 {
                    self.current_task = 0;
                }
            }
        }

        self.switching.store(false, Ordering::Release);
    }


    extern "C" fn switch_context(&mut self) {
        if self.tasks.is_empty() {
            return;
        }

        let current = &mut self.tasks[self.current_task as usize];
        current.cpu_state.save();

        // Rotate to next task
        self.current_task = (self.current_task + 1) % self.tasks.len() as u32;

        let next = &mut self.tasks[self.current_task as usize];
        next.switch_stack();

        // Ensure stack pointer is 16-byte aligned and points to the top of the stack
        next.cpu_state.rsp = (next.cpu_state.rsp & !0xF) - 8;

        // Load next task's state
        next.cpu_state.load();
    }

}



lazy_static! {
    pub static ref TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager {
        tasks: Vec::new(),
        current_task: 0,
        switching: AtomicBool::new(false),
        delete: None
    });
}

pub fn schedule() {
    TASK_MANAGER.lock().schedule();
}
fn remove_task()
{
    TASK_MANAGER.lock().delete_current();
}
pub fn add_task(task: Task) {
    TASK_MANAGER.lock().add_task(task);
}

