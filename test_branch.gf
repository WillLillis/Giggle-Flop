ADDIM R0, 1
CMP32 R0, R1
JNE DONEZO
ADDIM R5, 420
HALT
DONEZO:
    ADDIM R5, 69
    HALT

HALT
ADDIM R0, 5
CALL R6_LABEL
R4_LABEL:
    ADDIM R4, 4
    HALT
R6_LABEL:
    ADDIM R6, 6
    RET
