/* Original DIVULGE fixture; MIT. Linker-provided absolute symbols are the oracle. */
    .text
    .p2align 1
    .globl _start, far_call, far_goto, far_if, far_back_if, far_back_call, far_back_goto
    .globl far_forward, far_backward
_start:
far_call:
    call far_forward
far_goto:
    goto far_forward
far_if:
    if (r0 == 0) goto far_forward
far_back_if:
    ifs (r1 > 0) goto far_backward
far_back_call:
    call far_backward
far_back_goto:
    goto far_backward
    rts
