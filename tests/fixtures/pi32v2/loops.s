/* Original DIVULGE fixture; MIT. Loop braces are continuation text, not bytes. */
    .text
    .p2align 1
    .globl _start, rep_register, rep_immediate, after_rep
_start:
rep_register:
    rep r0 {
        r1 += 1
        r2 += 2
    }
rep_immediate:
    rep 3 {
        r3 += 1
    }
after_rep:
    rts
