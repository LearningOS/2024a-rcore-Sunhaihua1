# 总结

在本次的实验中，实现了sys_task_info这个系统调用，从而实现当前任务的信息，对于获取当前任务，可以通过公共接口从TASK_MANAGER获取current_task;对于需要获取的信息,可以通过在TCB结构体中加入相关定义实现。status恒为Running,syscall_times可以在syscall函数中对相应的syscall_id进行加1操作，之后再进行处理实现。time可以在TCB中记录第一次运行时间，在任务状态为UnInit且第一次被调用的时候通过get_time_ms获取。

# 简答



1.rustsbi版本：0.2.2

出错行为

```
[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.

```

2.

2.1 a0是传入__restore的第一个参数，用于app运行结束或者出错之后的切换；或者用于app在初始化阶段的加载。

2.2 csrw只能操作寄存器，因此本文是将栈上的的内容存在t0-t1寄存器，然后再转移。



4. sp指向用户栈，sscratch指向内核栈。
5. 发生状态切换是在sret,会更改相应的寄存器。
6. sp指向内核站，sscratch指向用户栈。
7. ecall指令

# 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > 无

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > 无

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。