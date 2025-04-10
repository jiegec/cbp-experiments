#include <stdint.h>

enum branch_type {
  op_direct_jump,             // jmp imm
  op_indirect_jump,           // jmp reg
  op_direct_call,             // call imm
  op_indirect_call,           // call reg
  op_return,                  // ret
  op_conditional_direct_jump, // jnz imm
  op_invalid
};

struct __attribute__((packed)) branch {
  uint64_t inst_addr;
  uint64_t targ_addr;
  uint32_t inst_length;
  enum branch_type type;
};

struct __attribute__((packed)) entry {
  int br_index : 15;
  int taken : 1;
};

// trace file format:
// struct __attribute__((packed)) file {
//   struct entry entries[num_entries]; // this array is zstd-compressed
//   struct branch branches[num_br];
//
//   uint64_t num_brs;
//   uint64_t num_entries;
// }

#define MAX_BRS (1 << 15)
