/* Original DIVULGE fixture; MIT. Assemble/link only, never execute. */
    .text
    .p2align 1
    .globl _start, forward_goto, forward_target, forward_cond, backward_cond
    .globl backward_goto, call_forward, call_backward, call_target, indirect_call
    .globl indirect_goto, stack_return, plain_return, immediate_six, normal_four
_start:
forward_goto:
    goto forward_target
immediate_six:
    r7 = 0x12345678
normal_four:
    r0 = r1 + -3
forward_target:
forward_cond:
    if (r0 == 0) goto call_forward
backward_cond:
    ifs (r1 > 0) goto forward_target
backward_goto:
    goto _start
call_forward:
    call call_target
call_backward:
    call _start
indirect_call:
    call r2
indirect_goto:
    goto r3
stack_return:
    pc = [sp++]
plain_return:
    rts
call_target:
    r0 += 3
    rts
