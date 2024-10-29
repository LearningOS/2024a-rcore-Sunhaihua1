//! Types related to task management & Functions for completely changing TCB

use super::id::TaskUserRes;
use super::{kstack_alloc, KernelStack, ProcessControlBlock, TaskContext};
use crate::trap::TrapContext;
use crate::{mm::PhysPageNum, sync::UPSafeCell};
use alloc::collections::BTreeMap;
use alloc::sync::{Arc, Weak};
use core::cell::RefMut;

/// Task control block structure
pub struct TaskControlBlock {
    /// immutable
    pub process: Weak<ProcessControlBlock>,
    /// Kernel stack corresponding to PID
    pub kstack: KernelStack,
    /// mutable
    inner: UPSafeCell<TaskControlBlockInner>,
}

impl TaskControlBlock {
    /// Get the mutable reference of the inner TCB
    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }
    /// Get the address of app's page table
    pub fn get_user_token(&self) -> usize {
        let process = self.process.upgrade().unwrap();
        let inner = process.inner_exclusive_access();
        inner.memory_set.token()
    }
}

pub struct TaskControlBlockInner {
    pub res: Option<TaskUserRes>,
    /// The physical page number of the frame where the trap context is placed
    pub trap_cx_ppn: PhysPageNum,
    /// Save task context
    pub task_cx: TaskContext,

    /// Maintain the execution status of the current process
    pub task_status: TaskStatus,
    /// It is set when active exit or execution error occurs
    pub exit_code: Option<i32>,
    pub mutex_need: BTreeMap<usize, usize>,
    pub mutex_allocation: BTreeMap<usize, usize>,
    pub sem_need: BTreeMap<usize, usize>,
    pub sem_allocation: BTreeMap<usize, usize>,
}

impl TaskControlBlockInner {
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }

    #[allow(unused)]
    fn get_status(&self) -> TaskStatus {
        self.task_status
    }
    pub fn set_mutex_allocated(&mut self, mutex_id: usize, value: usize) {
        self.mutex_allocation.insert(mutex_id, value);
    }
    pub fn set_mutex_need(&mut self, mutex_id: usize, value: usize) {
        self.mutex_need.insert(mutex_id, value);
    }
    pub fn get_mutex_allocated(&self, mutex_id: usize) -> usize {
        if !self.mutex_allocation.contains_key(&mutex_id) {
            return 0;
        }
        *self.mutex_allocation.get(&mutex_id).unwrap()
    }
    pub fn get_mutex_need(&self, sem_id: usize) -> usize {
        if !self.mutex_need.contains_key(&sem_id) {
            return 0;
        }
        *self.mutex_need.get(&sem_id).unwrap()
    }
    pub fn set_sem_allocated(&mut self, sem_id: usize, value: usize) {
        self.sem_allocation.insert(sem_id, value);
    }
    pub fn set_sem_need(&mut self, sem_id: usize, value: usize) {
        self.sem_need.insert(sem_id, value);
    }
    pub fn get_sem_allocated(&self, sem_id: usize) -> usize {
        if !self.sem_allocation.contains_key(&sem_id) {
            return 0;
        }

        *self.sem_allocation.get(&sem_id).unwrap()
    }
    pub fn get_sem_need(&self, sem_id: usize) -> usize {
        if !self.sem_need.contains_key(&sem_id) {
            return 0;
        }
        *self.sem_need.get(&sem_id).unwrap()
    }
}

impl TaskControlBlock {
    /// Create a new task
    pub fn new(
        process: Arc<ProcessControlBlock>,
        ustack_base: usize,
        alloc_user_res: bool,
    ) -> Self {
        let res = TaskUserRes::new(Arc::clone(&process), ustack_base, alloc_user_res);
        let trap_cx_ppn = res.trap_cx_ppn();
        let kstack = kstack_alloc();
        let kstack_top = kstack.get_top();
        Self {
            process: Arc::downgrade(&process),
            kstack,
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    res: Some(res),
                    trap_cx_ppn,
                    task_cx: TaskContext::goto_trap_return(kstack_top),
                    task_status: TaskStatus::Ready,
                    exit_code: None,
                    mutex_need: BTreeMap::new(),
                    mutex_allocation: BTreeMap::new(),
                    sem_need: BTreeMap::new(),
                    sem_allocation: BTreeMap::new(),
                })
            },
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
/// The execution status of the current process
pub enum TaskStatus {
    /// ready to run
    Ready,
    /// running
    Running,
    /// blocked
    Blocked,
}
