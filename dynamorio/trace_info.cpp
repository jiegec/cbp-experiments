#include "common.h"
#include <algorithm>
#include <assert.h>
#include <cstdint>
#include <fcntl.h>
#include <numeric>
#include <stdint.h>
#include <stdio.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <vector>

struct result {
  uint64_t count;
  uint64_t mispred_count;
  double taken;
  double not_taken;
  double mispred;

  bool operator<(const result &other) const {
    return mispred_count > other.mispred_count;
  }
};

int main(int argc, char *argv[]) {
  if (argc != 2) {
    fprintf(stderr, "Usage: %s log_name\n", argv[0]);
    return 1;
  }
  struct stat st;
  assert(stat(argv[1], &st) == 0);
  int fd = open(argv[1], O_RDONLY);
  assert(fd != -1);

  // mmap
  uint8_t *base = (uint8_t *)mmap(0, st.st_size, PROT_READ, MAP_SHARED, fd, 0);
  assert(base != MAP_FAILED);

  // read num_brs and num_entries
  uint64_t num_brs = *(uint64_t *)&base[st.st_size - 16];
  uint64_t num_entries = *(uint64_t *)&base[st.st_size - 8];
  printf("Got %ld branches and %ld entries\n", num_brs, num_entries);

  // read entries and branches
  entry *entries = (entry *)base;
  branch *brs = (branch *)&base[sizeof(entry) * num_entries];

  uint64_t branch_type_counts[op_invalid] = {0};
  for (int i = 0; i < num_brs; i++) {
    assert(brs[i].type < op_invalid);
    branch_type_counts[brs[i].type]++;
  }

  printf("Branch counts:\n");
  printf("\tdirect jump: %ld\n", branch_type_counts[op_direct_jump]);
  printf("\tindirect jump: %ld\n", branch_type_counts[op_indirect_jump]);
  printf("\tdirect call: %ld\n", branch_type_counts[op_direct_call]);
  printf("\tindirect call: %ld\n", branch_type_counts[op_indirect_call]);
  printf("\treturn: %ld\n", branch_type_counts[op_return]);
  printf("\tconditional direct jump: %ld\n",
         branch_type_counts[op_conditional_direct_jump]);

  std::vector<uint64_t> branch_execution_counts;
  branch_execution_counts.resize(num_brs);
  std::vector<uint64_t> branch_taken_counts;
  branch_taken_counts.resize(num_brs);

  for (int i = 0; i < num_entries; i++) {
    assert(entries[i].br_index < num_brs);
    branch_execution_counts[entries[i].br_index]++;
    if (entries[i].taken) {
      branch_taken_counts[entries[i].br_index]++;
    }
  }

  // sort by execution counts, desc
  // https://stackoverflow.com/questions/1577475/c-sorting-and-keeping-track-of-indexes
  // initialize original index locations
  std::vector<size_t> idx(num_brs);
  std::iota(idx.begin(), idx.end(), 0);

  stable_sort(
      idx.begin(), idx.end(), [&branch_execution_counts](size_t i1, size_t i2) {
        return branch_execution_counts[i1] > branch_execution_counts[i2];
      });

  const char *branch_names[op_invalid] = {
      "direct jump",   "indirect jump", "direct call",
      "indirect call", "return",        "cond jump",
  };

  printf("Top branches by execution count:\n");
  printf(
      "| Branch PC  | Branch Type   | Execution Count | Taken Rate (%%) |\n");
  for (int i = 0; i < 10; i++) {
    size_t br_index = idx[i];
    printf("| 0x%08lx | %13s | %15ld | %14.2lf |\n", brs[br_index].inst_addr,
           branch_names[brs[br_index].type], branch_execution_counts[br_index],
           (double)branch_taken_counts[br_index] * 100.0 /
               branch_execution_counts[br_index]);
  }

  return 0;
}