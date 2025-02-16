use crate::{print, println};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::arch::{asm, naked_asm};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::interrupts;

const STACK_SIZE: usize = 512;
#[repr(align(16))]
pub struct Task {
    stack: Option<Box<[u64; STACK_SIZE]>>,
    id: usize,
    pub rsp: u64,
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
            stack: Some(stack),
            id,
        }
    }
    fn new_main() -> Self {
        Task {
            rsp: 0,
            stack: None,
            id: 0,
        }
    }
}

pub struct TaskManager {
    tasks: Vec<Task>,
    current_task: u32,
    delete: Option<u32>,
    next_id: u32,
    running: u32,
}

impl TaskManager {
    pub fn new() -> Self {
        let mut tasks = Vec::new();
        tasks.push(Task::new_main());
        TaskManager {
            tasks,
            current_task: 0,
            delete: None,
            next_id: 1,
            running: 0,
        }
    }

    fn delete_current(&mut self) {
        self.delete = Some(self.running);
    }
    pub fn add_task(&mut self, function: extern "C" fn()) {
        self.tasks.push(Task::new(function, self.next_id as usize));
        self.next_id += 1;
    }

    pub fn schedule(&mut self) {
        if self.tasks.len() == 1 {
            return;
        }

        let old_task_rsp: *mut u64 = &mut self.tasks[self.running as usize].rsp;
        self.current_task = (self.current_task + 1) % self.tasks.len() as u32;
        let new_rsp = self.tasks[self.current_task as usize].rsp;

        // in case that one index has been deleted last schedule
        if let Some(delete_index) = self.delete.take() {
            if delete_index < self.tasks.len() as u32 {
                self.tasks.remove(delete_index as usize);
                //Adjust current task index if necessary(remove shift left by one all the indexes that greater than the removed index
                if self.current_task > delete_index {
                    self.current_task -= 1;
                }
                // in case the main is the only task that remain run the main
                if self.tasks.len() == 1 {
                    unsafe {
                        unsafe { TASK_MANAGER.force_unlock() };
                        switch_context(self.tasks[0].rsp, null_mut());
                    }
                }
                self.running = self.current_task;
                // in case that there is only main and one more task run the task
                if (self.running == 1 ) && self.tasks.len() == 2 {
                    self.running = 1;
                    self.current_task = 1;
                    unsafe {
                        unsafe { TASK_MANAGER.force_unlock() };
                        switch_context(self.tasks[1].rsp, null_mut());
                    }
                }
            }
        }

        if self.tasks.len() == 2 && self.running == 1 {
            self.current_task = 1;
            return;
        }

        if self.current_task == 0{
            schedule();
            return;
        }

        interrupts::without_interrupts(|| {
            unsafe { TASK_MANAGER.force_unlock() };
            self.running = self.current_task;
            unsafe {
                switch_context(new_rsp, old_task_rsp);
            }
        });
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
        "pushfq",         // Push flags onto stack
        "cmp rsi, 0",
        "jz 2f",
        "mov [rsi], rsp", // old_rsp is passed in rsi
        "2:",
        // Switch stack pointer
        "mov rsp, rdi", // new_rsp is passed in rdi
        "popfq",        // Pop rflags from stack
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
    unsafe {
        TASK_MANAGER.force_unlock();
    }
    TASK_MANAGER.lock().schedule();
}
fn remove_task() {
    unsafe {
        TASK_MANAGER.force_unlock();
    }
    TASK_MANAGER.lock().delete_current();
    schedule();
}
pub fn add_task(func: extern "C" fn()) {
    TASK_MANAGER.lock().add_task(func);
}


