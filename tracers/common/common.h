#include <stdint.h>

enum branch_type {
  op_direct_jump,             // jmp imm
  op_indirect_jump,           // jmp reg/jmp mem
  op_direct_call,             // call imm
  op_indirect_call,           // call reg/call mem
  op_return,                  // ret
  op_conditional_direct_jump, // jnz imm
  op_invalid
};

// branch instance, unique by (inst_addr, targ_addr) pair
struct __attribute__((packed)) branch {
  uint64_t inst_addr;
  uint64_t targ_addr;
  uint32_t inst_length;
  enum branch_type type;
};

// dynamic branch invocation
struct __attribute__((packed)) entry {
  int br_index : 31;
  int taken : 1;
};

// loaded image, mimic perf_record_mmap2
struct __attribute__((packed)) image {
  // memory [start, start+len) mapped to file [0, len)
  uint64_t start;
  uint64_t len;

  // path
  char filename[256];
};

// trace file format:
// struct __attribute__((packed)) file {
//   struct entry entries[num_entries]; // this array is zstd-compressed
//   struct branch branches[num_brs];
//   struct image images[num_images];
//
//   uint64_t num_entries;
//   uint64_t num_brs;
//   uint64_t num_images;
// }

#define MAX_BRS (1 << 20)
#define MAX_IMAGES 128
