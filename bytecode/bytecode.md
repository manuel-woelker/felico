# felico bytecode format

## instruction format

The instruction format is a 32 bit unsigned integer with the following layout:

### instruction layout
```
| MSB                                     LSB |
| 8 bits  | 8 bits    | 8 bits    | 8 bits    |
|---------|-----------|-----------|-----------|
| op_code | operand_a | operand_b | operand_c |
```
The op_code is an 8-bit unsigned integer that specifies the operation to be performed.
operand_a, operand_b and operand_c are 8-bit unsigned integers that specify the operands of the operation.

### operand addressing modes

The operand addressing modes are as follows:

### Constant pool
The constant pool is a per module list of constants that are used by these instructions.

Each pool entry is 64 bit wide and can be one of the following:

The first 8 bits specify the type of the constant.


| constant type | description                                                                                                        |
|---------------|--------------------------------------------------------------------------------------------------------------------|
| 0             | byte array (bytes 1-3 denote the length, bytes 4-7 denote the offset in the data pool                              |
| 1             | UTF-8 string (bytes 1-3 denote the length in bytes, bytes (4-7) denote the offset in the data pool                 |
| 2             | Function import (bytes 1-3 denote the length of the function name, bytes 4-7 denote the offset into the data pool) |




