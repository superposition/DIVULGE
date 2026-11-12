/* callx forces the six-byte encoding; objdump prints it as call. */
    .text
    .p2align 1
    .globl _start, forward_callx, backward_callx, after_callx, far_forward, far_backward
_start:
forward_callx:
    callx far_forward
backward_callx:
    callx far_backward
after_callx:
    rts
