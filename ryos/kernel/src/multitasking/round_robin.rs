use alloc::boxed::Box;
use alloc::vec::Vec;
use core::arch::{asm, naked_asm};
use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::interrupts;
use crate::println;

const STACK_SIZE: usize = 512;
#[repr(align(16))]
pub struct Task {
    stack: Box<[u64; STACK_SIZE]>,
    id: usize,
    pub rsp: u64
}
impl Task {
    pub fn new(func: extern "C" fn(), id: usize) -> Self {
        let mut stack = Box::new([0; STACK_SIZE]);
        stack[STACK_SIZE - 1] = remove_task as u64;
        stack[STACK_SIZE - 2] = func as u64;
        for i in 0..16 {
            stack[STACK_SIZE - 3 - i] = 0
        }
        stack[STACK_SIZE - 18] = 0x202;
        Task {
            rsp: stack.as_ptr() as u64 + (((STACK_SIZE - 18) as u64) * 8),
            stack,
            id,
        }

    }

}

pub struct TaskManager {
    tasks: Vec<Task>,
    current_task: u32,
    switching: AtomicBool,
    delete: Option<u32>,
    next_id : u32,
    main: Task,
}

impl TaskManager {
    pub fn new() -> Self {
        TaskManager {
            main: Task::new(crate::main_kernel_mode, 0),
            tasks: Vec::new(),
            current_task: 0,
            switching: AtomicBool::new(false),
            delete: None,
            next_id: 1
        }
    }

    fn delete_current(&mut self) {
        self.delete = Some(self.current_task);
    }
    pub fn add_task(&mut self, function: extern "C" fn()){
        self.tasks.push(Task::new(function, self.next_id as usize));
        self.next_id += 1;

    }

    pub fn schedule(&mut self) {
        if self.tasks.len() <= 1 {
            println!("test0");
            unsafe {switch_context(self.main.rsp, core::ptr::null_mut());}
            return;  // Need at least two tasks to switch
        }
        println!("test1");
        // Use a more robust synchronization mechanism
        if self.switching.load(Ordering::Acquire) {
            return;
        }
        println!("test2");
        self.switching.store(true, Ordering::Release);
        println!("test3");

        let old_task_rsp: *mut u64 = &mut self.tasks[self.current_task as usize].rsp;
        self.current_task = (self.current_task + 1) % self.tasks.len() as u32;
        let new_rsp = self.tasks[self.current_task as usize].rsp;

        unsafe { switch_context(new_rsp, old_task_rsp); }


        println!("test4");

        if let Some(delete_index) = self.delete.take() {
            if delete_index < self.tasks.len() as u32 {
                self.tasks.remove(delete_index as usize);

                // Adjust current task index if necessary
                if self.current_task >= self.tasks.len() as u32 {
                    self.current_task = 0;
                }
            }
        }
        println!("test5");
        self.switching.store(false, Ordering::Release);
    }

}
#[naked]
pub unsafe extern "C" fn switch_context(new_rsp: u64, old_rsp: *mut u64) {
    naked_asm!(
        // Save all general-purpose registers on the current stack
        "push rax",
        "push rcx",
        "push rdx",
        "push rbx",
        "push rbp",
        "push rsi",
        "push rdi",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "pushfq",           // Push flags onto stack
        "mov [rsi], rsp",  // old_rsp is passed in rsi
        // Switch stack pointer
        "mov rsp, rdi",  // new_rsp is passed in rdi
        "popfq",            // Pop rflags from stack
        // Restore registers from the new stack
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rdi",
        "pop rsi",
        "pop rbp",
        "pop rbx",
        "pop rdx",
        "pop rcx",
        "pop rax",
        // Return to the new context
        "ret",
        );
}


lazy_static! {
    pub static ref TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());
}


pub fn schedule() {
    TASK_MANAGER.lock().schedule();
}
fn remove_task()
{
    println!("removing task {}", TASK_MANAGER.lock().current_task);
    TASK_MANAGER.lock().delete_current();
}
pub fn add_task(func: extern "C" fn()) {
    TASK_MANAGER.lock().add_task(func);
}

extern "C" fn print_no()
{
    for _ in 0..50
    {
        crate::print!("a");
    }
}


