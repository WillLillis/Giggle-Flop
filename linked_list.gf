// Arbitrarily start at address 2304...
XORI R11, R11, R11
XORI R13, R13, R13
ADDIM R13, 1
// Set the head of the list to NULL
XORI R0, R0, R0
ADDIM R0, 2034

// Insert elements into the linked list
LDI32 R1, 10   // Insert value 10
CALL insert_node

LDI32 R1, 20   // Insert value 20
CALL insert_node

LDI32 R1, 30   // Insert value 30
CALL insert_node

// Remove the node at index 1
LDI32 R2, 1    // Index to remove
CALL remove_at_index

// Remove the node at index 0
LDI32 R2, 0    // Index to remove
CALL remove_at_index

// Remove the node at index 1
LDI32 R2, 1    // Index to remove
CALL remove_at_index

// Exit the program
HALT

// Function to insert a node into the linked list
// Inputs: R1 - value to insert
// Outputs: none
insert_node:
    // Allocate memory for a new node
    CALL allocate_node

    // Store the value in the new node
    STIN32 R1, R0

    // Find the last node in the list
    LDIN32 R2, R0     // R2 <- Address of the current node (start from head)
    ADDIM R6, 32
    ADDU R3, R2, R6 // R3 <- Address of the next node

find_last_node:
    CMP32 R3, R11     // Check if the next node is NULL
    IJE found_last_node

    XORI R12, R12, R12
    ADDIM R12, 32
    // Move to the next node
    ADDU R2, R3, R12    // R2 <- Address of the current node
    LDIN32 R3, R3 // R3 <- Address of the next node
    CALL find_last_node

found_last_node:
    // Insert the new node at the end of the list
    STIN32 R2, R0  // Update the next pointer of the last node
    RET

// Function to remove a node from the linked list at the specified index
// Inputs: R2 - index of the node to remove
// Outputs: none
remove_at_index:
    // Check if the list is empty
    LD32 R1, 0  // R1 <- Address of the head of the list
    CMP32 R1, R11   // Check if the head pointer is NULL
    IJE end_remove

    // Special case: remove the head node
    CMP32 R2, R11   // Check if the index is 0
    IJE remove_head_node

    // General case: remove a node from the middle or end of the list
    LD32 R3, 0   // R3 <- Address of the previous node (start from NULL)
    ADDU R4, R1, R11  // R4 <- Address of the current node (start from head)

traverse_list:
    // Decrement the index
    SUBU R2, R2, R13
    // Check if we've reached the desired index
    CMP32 R2, R11
    IJE found_node_to_remove

    // Move to the next node
    ADDU R3, R4, R11       // R3 <- Address of the previous node
    LDIN32 R4, R4    // Move to the next node
    CALL traverse_list

found_node_to_remove:
    // Remove the node
    LDIN32 R5, R4    // R5 <- Address of the next node
    STIN32 R3, R5    // Update the next pointer of the previous node
    RET

remove_head_node:
    // Remove the head node
    LDIN32 R1, R1    // R1 <- Address of the next node (new head)
    ST32 R1, 2034       // Update the head pointer
    RET

end_remove:
    RET

// Function to allocate memory for a new node
// Outputs: R0 - address of the allocated memory block
allocate_node:
    // Allocate memory for the node (32 bits)
    // For demonstration, let's just simulate the allocation by moving R0 to a predefined memory address
    XORI R12, R12, R12
    ADDIM R12, 32
    ADDU R0, R0, R12 // Increment to the next available memory block for the next allocation
    ST32 R0, 0
    RET
