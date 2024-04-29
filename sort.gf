// We'll start reading data from a known memory location
// The first word will be the length, and then every 
// word after that (up to the length) will be treated
// as a u32...
// Bubble Sort:
//
//  for i = 0..n-1
//      for j = 0..n-i-1
//          if arr[j] > arr[j+1]
//              /* Swap arr[j] and arr[j+1] */

// Arbitrarily start at address 1152...

// Get the length
LD32 R1, 1152 // R1 <- n

// Load start address into R0
ADDIM R0, 1184 // 1152 + 32

// Load the stop address into R1
ADDIM R3, 32
MULU R1, R1, R3 // R1 <- n * 32 (length of array in bits)
ADDU R1, R0, R1
SUBU R1, R1, R3
SUBU R1, R1, R3 // R1 now holds the address of the 2nd to last entry

XORI R3, R3, R3

// outer loop init
XORI R2, R2, R2 
ADDU R2, R2, R0 // R2 <- &data[0], analogous to i
OUTER_LOOP:
    // inner loop init, R5 analogous to n - i - 1
    XORI R5, R5, R5
    ADDU R5, R5, R1 // R5 <- &data[n-1]
    SUBU R5, R5, R2 // R5 <- &data[n-i-1]

    XORI R3, R3, R3 // R3 analogous to j
    ADDU R3, R3, R0 // R3 <- &data[0]
    INNER_LOOP:

        // comparison and swap here
        XORI R4, R4, R4
        ADDU R4, R4, R3
        ADDIM R4, 32 // R4 <- &data[j+1]
        // load values pointed to by R3 and R4 
        // into R6 and R7, compare, then branch accordingly
        LDIN32 R6, R3 // R6 <- data[j]
        LDIN32 R7, R4 // R7 <- data[j+1]

        CMP32 R6, R7
        JLTE INNER_LOOP_END // if they're already in order, don't swap
        STIN32 R7, R3 // swap
        STIN32 R6, R4 // ^

        INNER_LOOP_END:
            ADDIM R3, 32 // j++
            //CMP32 R3, R5 // check termination condition
            CMP32 R3, R1
            //JLT INNER_LOOP
            JLTE INNER_LOOP


    ADDIM R2, 32 // i++
    CMP32 R2, R1
    JLTE OUTER_LOOP
    HALT
