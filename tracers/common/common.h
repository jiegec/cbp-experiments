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
  uint64_t data_size;
  uint64_t data_offset; // offset of image data from the beginning of file

  // path
  char filename[256];
};

#define MAGIC 0x2121505845504243ULL

struct __attribute__((packed)) file_header {
  // trace file format magic
  uint64_t magic;
  // trace file version
  uint64_t version;
  uint64_t num_entries;
  // offset of entries array from the beginning of file
  uint64_t entries_offset;
  // size of compressed entries array
  uint64_t entries_size;
  uint64_t num_branches;
  // offset of branches array from the beginning of file
  uint64_t branches_offset;
  uint64_t num_images;
  // offset of images array from the beginning of file
  uint64_t images_offset;
};

// trace file format:
// struct __attribute__((packed)) file {
//   file_header header;
//
//   // the following arrays can be placed in arbitrary location
//   struct entry entries[header.num_entries]; // this array is zstd-compressed
//   struct branch branches[header.num_branches];
//   struct image images[header.num_images];
// }

#define MAX_BRS (1 << 25)
#define MAX_IMAGES 128
