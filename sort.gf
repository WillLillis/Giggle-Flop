// We'll start reading from a known memory location
// The first word will be the length, and then every 
// word after that (up to the length) will be treated
// as a u32...
//  for i = 0..n-1
//      for j = 0..n-i-1
//          if arr[j] > arr[j+1]
//              swap
// R0 = i, R1 = j, R2 = n use other registers for scratch
// Start at address 640...

// Get the length
LD32 R2, 640 // R2 <- n

// Load start address into R0
ADDIM R0, 672 // 640 + 32

// Load the stop address into R2
ADDIM R3, 32
MULU R2, R2, R3 // R2 <- n * 32 (length of array)
ADDU R2, R0, R2 // First address past the end of the array

// for (i = 0; i < n - 1; i++)
OUTER_LOOP:
    
    INNER_LOOP:
        // compute stop address
        // j = 0



    ADDIM R0, 32
    CMP32 R0, R2
    JLT OUTER_LOOP
// how do we want to evaluate the loop ending?
