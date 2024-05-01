// We'll start reading data from a known memory location
// The first word will be the number of rows of first matrix
// The second word will be the number of columns of the second matrix
// The third word will be the columns of first matrix/rows of second matrix
// 
// Matrix Multiply:
//
// for (int r = 0// r < M1 row count// r++) { <- Loop_1
//   for (int c = 0// c < M2 column count// c++) { <- Loop_2
//     R[r][c] = 0
//     for (int k = 0// k < M1 column count// k++) { <- Loop 3
//       R[r][c] += M1[r][k]*M2[k][c]
//     }
//   }
// }


// Arbitrarily start at address 2304...

// Get the rows of first matrix
LD32 R0, 2304 // R0 <- r

// Get the columns of the second matrix
LD32 R1, 2336 // R1 <- c

//Get the columns of first matrix and rows of second matrix
LD32 R2, 2368 // R2 <- k

XORI R15, R15, R15
ADDIM R15, 2304
// Calculate the sizes of matrices 1, 2, and 3
LDIN32 R3, R15        // R3 <- Rows of matrix 1
ADDIM R15, 32
LDIN32 R4, R15        // R4 <- Columns of matrix 2
ADDIM R15, 32
LDIN32 R5, R15        // R5 <- Columns of matrix 1 and rows of matrix 2

XORI R15, R15, R15
ADDIM R15, 32

// Calculate the sizes of matrices
MULU R3, R3, R5
MULU R3, R3, R15    // R3 <- Size of matrix 1 in bits
MULU R4, R4, R5
MULU R4, R4, R15     // R4 <- Size of matrix 2 in bits
ADDU R5, R3, R4    // R5 <- Total size of matrices 1 and 2 in bits

XORI R9, R9, R9
ADDIM R9, 2304      // Get start address again
// Load the address of the first matrix into R6
ADDIM R10, 96
ADDU R6, R9, R10   // Address of the first matrix

// Load the address of the second matrix into R7
ADDU R7, R6, R3    // Address of the second matrix

// Load the address of the third matrix into R8
ADDU R8, R7, R4    // Address of the third matrix

// Loop to populate matrices 1, 2, and 3
// Loop_1: Iterate over rows of matrix 1
XORI R9, R9, R9     // Initialize row counter for matrix 1
outer_loop:
    // Loop_2: Iterate over columns of matrix 2
    XORI R10, R10, R10        // Initialize column counter for matrix 2
inner_loop:
    // Loop_3: Iterate over columns of matrix 1 and rows of matrix 2
    XORI R11, R11, R11        // Initialize loop counter for matrix 1 columns and matrix 2 rows
mul_loop:
    // Load the current value from matrix 1
    // LDIN32 R12, [R6 + R9 * R0 * 32 + R11 * 32]
    // does this work?
    XORI R14, R14, R14
    ADDIM R14, 1
    MULU R12, R0, R15
    MULU R14, R14, R15
    SUBU R12, R12, R14
    MULU R12, R12, R9
    ADDU R12, R12, R6

    MULU R13, R11, R15
    ADDU R12, R12, R13
    LDIN32 R12, R12

    // Load the current value from matrix 2
    // LDIN32 R13, [R7 + R11 * R2 * 32 + R10 * 32]
    // does this work?
    XORI R14, R14, R14
    ADDIM R14, 1
    MULU R13, R2, R15
    MULU R14, R14, R15
    SUBU R13, R13, R14
    MULU R13, R13, R11
    ADDU R13, R13, R7

    MULI R14, R10, R15
    ADDU R13, R13, R14
    LDIN32 R13, R13

    // Multiply and accumulate
    MULU R14, R12, R13

    // Store the result to matrix 3
    // STIN32 [R8 + R9 * R0 * 32 + R10 * 32], R14
    MULU R12, R0, R15
    MULU R12, R12, R9
    ADDU R12, R12, R8

    MULU R13, R10, R15
    ADDU R13, R13, R12
    STIN32 R14, R13

    // Increment loop counter for matrix 1 columns and matrix 2 rows
    ADDIM R11, 1

    // Check loop condition for loop 3
    CMP32 R11, R2
    JLT mul_loop

    // Move to the next column of matrix 2
    ADDIM R10, 1

    // Check loop condition for loop 2
    CMP32 R10, R1
    JLT inner_loop

    // Move to the next row of matrix 1
    ADDIM R9, 1

    // Check loop condition for loop 1
    CMP32 R9, R0
    JLT outer_loop

    // If we've reached here, matrices 1, 2, and 3 are populated
    HALT
