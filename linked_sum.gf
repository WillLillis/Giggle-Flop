// Set up accumulator
// Get pointer to first item
// Start jumping


// Keep a reference 0 value
XORI R4, R4, R4

// Arbitrarily start at address 1152...
XORI R1, R1, R1 
ADDIM R1, 1152

// Accumulator starts at 0
XORI R0, R0, R0 

LOOP:
    LDIN32 R2, R1 // Get the value
    ADDU R0, R0, R2 // Add the value
    
    ADDIM R1, 32 // access the "next" field
    LDIN32 R1, R1

    // if it's null, we're done
    CMP32 R1, R4
    JNE LOOP


// Store result at address 1120
ST32 R0, 1120
HALT
