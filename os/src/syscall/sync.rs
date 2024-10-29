use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    let thread_count = process_inner.thread_count();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        process_inner.set_mutex_available(id, 1);
        for task_id in 0..thread_count {
            let task = process_inner.get_task(task_id);
            let mut task_inner = task.inner_exclusive_access();
            task_inner.set_mutex_allocated(id, 0);
            task_inner.set_mutex_need(id, 0);
        }

        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        let len = process_inner.mutex_list.len() as isize - 1;
        process_inner.set_mutex_available(len as usize, 1);
        for task_id in 0..thread_count {
            let task = process_inner.get_task(task_id);
            let mut task_inner = task.inner_exclusive_access();
            task_inner.set_mutex_allocated(len as usize, 0);
            task_inner.set_mutex_need(len as usize, 0);
        }
        len
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    task_inner.set_mutex_need(mutex_id, 1);
    drop(task_inner);
    drop(task);
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let detect = process_inner.dead_lock_detect;
    let thread_count = process_inner.thread_count();
    if detect == 1 {
        let mut work = process_inner.mutex_available.clone();
        let mut finish = vec![false; thread_count];
        loop {
            let mut found = false;
            for task_id in 0..thread_count {
                if finish[task_id] == true {
                    continue;
                }
                let task = process_inner.get_task(task_id);
                let task_inner = task.inner_exclusive_access();
                let adjust = work
                    .iter()
                    .any(|(&id, &available)| task_inner.get_mutex_need(id) > available);
                if !adjust {
                    finish[task_id] = true;
                    work.iter_mut().for_each(|(&id, available)| {
                        *available += task_inner.get_mutex_allocated(id);
                    });
                    found = true;
                }
            }
            if !found {
                break;
            }
        }
        let task = current_task().unwrap();
        let mut task_inner = task.inner_exclusive_access();
        if finish.iter().any(|x| *x == false) {
            task_inner.set_mutex_need(mutex_id, 0);
            return -0xDEAD;
        }
        task_inner.set_mutex_allocated(mutex_id, 1);
        task_inner.set_mutex_need(mutex_id, 0);
        process_inner.set_mutex_available(mutex_id, 0);
        drop(task_inner);
        drop(task);
    }
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    let detect = process_inner.dead_lock_detect;
    if detect == 1 {
        process_inner.set_mutex_available(mutex_id, 1);
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .set_mutex_allocated(mutex_id, 0);
    }

    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    process_inner.set_sem_available(id, res_count);
    error!("semaphore create id:{} {}", id, res_count);

    for task_id in 0..process_inner.thread_count() {
        let task = process_inner.get_task(task_id);
        let mut task_inner = task.inner_exclusive_access();
        task_inner.set_sem_allocated(id, 0);
        task_inner.set_sem_need(id, 0);
    }
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let detect = process_inner.dead_lock_detect;
    if detect == 1 {
        let available = process_inner.get_sem_available(sem_id);
        process_inner.set_sem_available(sem_id, available + 1);
        let need = current_task()
            .unwrap()
            .inner_exclusive_access()
            .get_sem_need(sem_id);
        if need != 0 {
            current_task()
                .unwrap()
                .inner_exclusive_access()
                .set_sem_need(sem_id, need - 1);
        }

        let allocated = current_task()
            .unwrap()
            .inner_exclusive_access()
            .get_sem_allocated(sem_id);

        if allocated != 0 {
            current_task()
                .unwrap()
                .inner_exclusive_access()
                .set_sem_allocated(sem_id, allocated - 1);
        }
    }
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let task = current_task().unwrap();
    let mut task_inner = task.inner_exclusive_access();
    let need = task_inner.get_sem_need(sem_id);
    task_inner.set_sem_need(sem_id, need + 1);

    drop(task_inner);
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    let thread_count = process_inner.thread_count();
    let detect = process_inner.dead_lock_detect;
    if detect == 1 {
        let mut work = process_inner.sem_available.clone();
        let mut finish = vec![false; thread_count];
        loop {
            let mut found = false;
            for task_id in 0..thread_count {
                if finish[task_id] {
                    continue;
                }
                let task = process_inner.get_task(task_id);
                let task_inner = task.inner_exclusive_access();
                let adjust = work
                    .iter()
                    .any(|(&id, &available)| task_inner.get_sem_need(id) > available);
                if !adjust {
                    finish[task_id] = true;
                    work.iter_mut().for_each(|(&id, available)| {
                        *available += task_inner.get_sem_allocated(id);
                    });
                    found = true;
                }
            }
            if !found {
                break;
            }
        }
        let task = current_task().unwrap();
        let mut task_inner = task.inner_exclusive_access();
        if finish.iter().any(|x| *x == false) {
            let need = task_inner.get_sem_need(sem_id);
            task_inner.set_sem_need(sem_id, need - 1);
            return -0xDEAD;
        }
        let available = process_inner.get_sem_available(sem_id);

        if available > 0 {
            let allocated = task_inner.get_sem_allocated(sem_id);
            error!("allocated:{}", allocated);
            task_inner.set_sem_allocated(sem_id, allocated + 1);
            let need = task_inner.get_sem_need(sem_id);
            task_inner.set_sem_need(sem_id, need - 1);
            error!("need:{}", need);
            process_inner.set_sem_available(sem_id, available - 1);
            error!("available:{}", available);
        }
        drop(task_inner);
        drop(task);
    }
    drop(task);
    drop(process_inner);
    sem.down();
    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.enable_dead_lock_detect(_enabled);
    0
}
