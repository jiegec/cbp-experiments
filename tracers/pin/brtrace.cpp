#include "../common/common.h"
#include "pin.H"
#include <cassert>
#include <cmath>
#include <map>
#include <stdio.h>
#include <zstd.h>

// based on pin manual examples

FILE *trace;
struct branch brs[MAX_BRS];
std::map<struct branch, int> br_map;
uint64_t num_brs;
uint64_t num_entries;

bool operator<(const struct branch &a, const struct branch &b) {
  if (a.inst_addr != b.inst_addr) {
    return a.inst_addr < b.inst_addr;
  }
  if (a.targ_addr != b.targ_addr) {
    return a.targ_addr < b.targ_addr;
  }
  if (a.inst_length != b.inst_length) {
    return a.inst_length < b.inst_length;
  }
  return a.type < b.type;
}

#define BUFFER_SIZE 16384

struct entry write_buffer[BUFFER_SIZE];
int buffer_size;

// zstd
ZSTD_CCtx *zstd_cctx;
void *zstd_output_buffer;
size_t zstd_output_buffer_size;

VOID RecordBranch(VOID *inst_addr, VOID *targ_addr, UINT32 inst_length,
                  UINT32 type, BOOL taken) {
  struct branch br;
  br.inst_addr = (uint64_t)inst_addr;
  br.targ_addr = (uint64_t)targ_addr;
  br.inst_length = inst_length;
  br.type = (branch_type)type;

  struct entry e;
  e.taken = taken;

  // insert branch if not exists
  auto it = br_map.find(br);
  if (it == br_map.end()) {
    assert(num_brs < MAX_BRS);
    br_map[br] = num_brs;
    e.br_index = num_brs;
    brs[num_brs++] = br;
  } else {
    e.br_index = it->second;
  }

  if (buffer_size == BUFFER_SIZE) {
    // send write_buffer to zstd

    // https://github.com/facebook/zstd/blob/dev/examples/streaming_compression.c
    ZSTD_EndDirective mode = ZSTD_e_continue;
    ZSTD_inBuffer input = {write_buffer, sizeof(write_buffer), 0};
    int finished;
    do {
      ZSTD_outBuffer output = {zstd_output_buffer, zstd_output_buffer_size, 0};
      size_t remaining = ZSTD_compressStream2(zstd_cctx, &output, &input, mode);
      assert(!ZSTD_isError(remaining));
      assert(fwrite(zstd_output_buffer, 1, output.pos, trace) == output.pos);
      finished = input.pos == input.size;
    } while (!finished);

    buffer_size = 0;
  }
  write_buffer[buffer_size++] = e;
  num_entries++;
}

VOID Instruction(INS ins, VOID *v) {
  if (INS_IsControlFlow(ins)) {
    UINT32 size = INS_Size(ins);
    enum branch_type type = op_invalid;
    if (INS_Category(ins) == XED_CATEGORY_COND_BR) {
      type = op_conditional_direct_jump;
    } else if (INS_IsRet(ins)) {
      type = op_return;
    } else if (INS_IsDirectCall(ins)) {
      type = op_direct_call;
    } else if (INS_IsCall(ins)) {
      type = op_indirect_call;
    } else if (INS_IsDirectBranch(ins)) {
      type = op_direct_jump;
    } else if (INS_IsBranch(ins)) {
      type = op_indirect_jump;
    } else {
      assert(false);
    }
    INS_InsertCall(ins, IPOINT_BEFORE, (AFUNPTR)RecordBranch, IARG_INST_PTR,
                   IARG_BRANCH_TARGET_ADDR, IARG_UINT32, size, IARG_UINT32,
                   type, IARG_BRANCH_TAKEN, IARG_END);
  }
}

KNOB<std::string> KnobOutputFile(KNOB_MODE_WRITEONCE, "pintool", "o",
                                 "brtrace.log", "Specify output file name");

VOID Fini(INT32 code, VOID *v) {
  // finish entries
  // https://github.com/facebook/zstd/blob/dev/examples/streaming_compression.c
  ZSTD_EndDirective mode = ZSTD_e_end;
  ZSTD_inBuffer input = {write_buffer, sizeof(struct entry) * buffer_size, 0};
  int finished;
  do {
    ZSTD_outBuffer output = {zstd_output_buffer, zstd_output_buffer_size, 0};
    size_t remaining = ZSTD_compressStream2(zstd_cctx, &output, &input, mode);
    assert(!ZSTD_isError(remaining));
    assert(fwrite(zstd_output_buffer, 1, output.pos, trace) == output.pos);
    finished = remaining == 0;
  } while (!finished);
  buffer_size = 0;

  // write branches
  assert(fwrite(brs, sizeof(struct branch), num_brs, trace) == num_brs);

  // write number of branches & number of events
  assert(fwrite(&num_brs, sizeof(num_brs), 1, trace) == 1);
  assert(fwrite(&num_entries, sizeof(num_entries), 1, trace) == 1);
  fclose(trace);
  fprintf(stderr, "Finished writing log\n");
}

INT32 Usage() {
  fprintf(stderr, "This tool generates a branch trace\n\n%s\n",
          KNOB_BASE::StringKnobSummary().c_str());
  return -1;
}

int main(int argc, char *argv[]) {
  // Initialize pin
  if (PIN_Init(argc, argv))
    return Usage();

  trace = fopen(KnobOutputFile.Value().c_str(), "w");

  zstd_cctx = ZSTD_createCCtx();
  assert(zstd_cctx);
  zstd_output_buffer_size = ZSTD_CStreamOutSize();
  zstd_output_buffer = malloc(zstd_output_buffer_size);
  assert(zstd_output_buffer);

  // Register Instruction to be called to instrument instructions
  INS_AddInstrumentFunction(Instruction, 0);

  // Register Fini to be called when the application exits
  PIN_AddFiniFunction(Fini, 0);

  // Start the program, never returns
  PIN_StartProgram();

  return 0;
}
