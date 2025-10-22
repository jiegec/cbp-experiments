/* **********************************************************
 * Copyright (c) 2014-2018 Google, Inc.  All rights reserved.
 * **********************************************************/

/*
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * * Redistributions of source code must retain the above copyright notice,
 *   this list of conditions and the following disclaimer.
 *
 * * Redistributions in binary form must reproduce the above copyright notice,
 *   this list of conditions and the following disclaimer in the documentation
 *   and/or other materials provided with the distribution.
 *
 * * Neither the name of VMware, Inc. nor the names of its contributors may be
 *   used to endorse or promote products derived from this software without
 *   specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED. IN NO EVENT SHALL VMWARE, INC. OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH
 * DAMAGE.
 */

/* Code Manipulation API Sample:
 * brtrace.c
 *
 * Collects the trace of branches executed.
 * Writes that info into per-thread files named brtrace.<pid>.<tid>.log
 * in the client library directory.
 */

#include "common.h"
#include "dr_api.h"
#include "drmgr.h"
#include "hashmap.h"
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <zstd.h>

static client_id_t client_id;

static int tls_idx;

static char log_file_name[256] = "brtrace.log";

#define BUFFER_SIZE 16384

struct tls {
  file_t log;
  struct branch brs[MAX_BRS];
  struct image images[MAX_IMAGES];
  struct hashmap_s br_map;

  uint64_t num_entries;
  uint64_t num_brs;
  uint64_t num_images;

  struct entry write_buffer[BUFFER_SIZE];
  int buffer_size;

  // zstd
  ZSTD_CCtx *zstd_cctx;
  void *zstd_output_buffer;
  size_t zstd_output_buffer_size;
};

static inline void logger(struct tls *t, app_pc inst_addr, app_pc fall_addr,
                          app_pc targ_addr, enum branch_type type, int taken) {
  struct branch br;
  br.inst_addr = (uint64_t)inst_addr;
  br.targ_addr = (uint64_t)targ_addr;
  br.inst_length = (uint8_t *)fall_addr - (uint8_t *)inst_addr;
  br.type = type;

  struct entry e;
  e.taken = taken;

  // insert branch if not exists
  int it = (int)(uintptr_t)hashmap_get(&t->br_map, &br, sizeof(br));
  if (it == 0) {
    assert(t->num_brs < MAX_BRS);
    struct branch *pbr = (struct branch *)malloc(sizeof(br));
    *pbr = br;
    hashmap_put(&t->br_map, pbr, sizeof(br),
                (void *)(uintptr_t)(t->num_brs + 1));
    e.br_index = t->num_brs;
    t->brs[t->num_brs++] = br;
  } else {
    e.br_index = it - 1;
  }

  if (t->buffer_size == BUFFER_SIZE) {
    // send write_buffer to zstd

    // https://github.com/facebook/zstd/blob/dev/examples/streaming_compression.c
    ZSTD_EndDirective mode = ZSTD_e_continue;
    ZSTD_inBuffer input = {t->write_buffer, sizeof(t->write_buffer), 0};
    int finished;
    do {
      ZSTD_outBuffer output = {t->zstd_output_buffer,
                               t->zstd_output_buffer_size, 0};
      size_t remaining =
          ZSTD_compressStream2(t->zstd_cctx, &output, &input, mode);
      assert(!ZSTD_isError(remaining));
      dr_write_file(t->log, t->zstd_output_buffer, output.pos);
      finished = input.pos == input.size;
    } while (!finished);

    t->buffer_size = 0;
  }
  t->write_buffer[t->buffer_size++] = e;
  t->num_entries++;
}

/* Clean call for the cbr */
static void at_cbr(app_pc inst_addr, app_pc targ_addr, app_pc fall_addr,
                   int taken, void *bb_addr) {
  void *drcontext = dr_get_current_drcontext();
  struct tls *t = (struct tls *)drmgr_get_tls_field(drcontext, tls_idx);
  assert(t);
  logger(t, inst_addr, fall_addr, targ_addr, op_conditional_direct_jump, taken);
}

/* Clean call for the mbr/ubr */
#define GEN_HANDLER(inst_length, type)                                         \
  static void at_mbrubr_##inst_length##_##type(app_pc inst_addr,               \
                                               app_pc targ_addr) {             \
    app_pc fall_addr = inst_addr + inst_length;                                \
    void *drcontext = dr_get_current_drcontext();                              \
    struct tls *t = (struct tls *)drmgr_get_tls_field(drcontext, tls_idx);     \
    assert(t);                                                                 \
    logger(t, inst_addr, fall_addr, targ_addr, type, 1);                       \
  }

#include "handlers.h"

#undef GEN_HANDLER

static dr_emit_flags_t event_app_instruction(void *drcontext, void *tag,
                                             instrlist_t *bb, instr_t *instr,
                                             bool for_trace, bool translating,
                                             void *user_data) {
  if (instr_is_cbr(instr)) {
    dr_insert_cbr_instrumentation_ex(
        drcontext, bb, instr, (void *)at_cbr,
        OPND_CREATE_INTPTR(dr_fragment_app_pc(tag)));
  } else if (instr_is_ubr(instr) || instr_is_mbr(instr) ||
             instr_is_call(instr)) {
    enum branch_type type = op_invalid;
    void *callback = NULL;
    if (instr_is_call_direct(instr)) {
      type = op_direct_call;
    } else if (instr_is_call_indirect(instr)) {
      type = op_indirect_call;
    } else if (instr_is_return(instr)) {
      type = op_return;
    } else if (instr_is_ubr(instr)) {
      type = op_direct_jump;
    } else if (instr_is_mbr(instr)) {
      type = op_indirect_jump;
    } else {
      assert(false);
    }

#define GEN_HANDLER(inst_length, _type)                                        \
  else if (instr_length(drcontext, instr) == inst_length && type == _type) {   \
    callback = (void *)at_mbrubr_##inst_length##_##_type;                      \
  }

    if (0) {
    }
#include "handlers.h"
    else {
      printf("Unhandled branch with type %d and length %d\n", type,
             instr_length(drcontext, instr));
      assert(false);
    }
#undef GEN_HANDLER

    if (instr_is_ubr(instr)) {
      dr_insert_ubr_instrumentation(drcontext, bb, instr, callback);
    } else if (instr_is_mbr(instr)) {
      dr_insert_mbr_instrumentation(drcontext, bb, instr, callback,
                                    SPILL_SLOT_1);
    } else if (instr_is_call(instr)) {
      dr_insert_call_instrumentation(drcontext, bb, instr, callback);
    } else {
      assert(false);
    }
  }
  return DR_EMIT_DEFAULT;
}

int hashmap_comparer(const void *const a, const hashmap_uint32_t a_len,
                     const void *const b, const hashmap_uint32_t b_len) {
  assert(a_len == sizeof(struct branch) && b_len == sizeof(struct branch));
  const struct branch *br_a = (const struct branch *)a;
  const struct branch *br_b = (const struct branch *)b;
  return br_a->inst_addr == br_b->inst_addr &&
         br_a->targ_addr == br_b->targ_addr &&
         br_a->inst_length == br_b->inst_length && br_a->type == br_b->type;
}

static void event_thread_init(void *drcontext) {
  file_t log;
  log =
      dr_open_file(log_file_name, DR_FILE_CLOSE_ON_FORK | DR_FILE_ALLOW_LARGE |
                                      DR_FILE_WRITE_OVERWRITE);
  DR_ASSERT(log != INVALID_FILE);
  // leave space for file header, zstd compressed branches start at
  // sizeof(struct file_header)
  dr_file_seek(log, sizeof(struct file_header), DR_SEEK_SET);
  struct tls *t = (struct tls *)malloc(sizeof(struct tls));
  t->log = log;
  t->num_entries = 0;
  t->num_brs = 0;
  t->num_images = 0;
  t->buffer_size = 0;
  struct hashmap_create_options_s options;
  memset(&options, 0, sizeof(options));
  options.initial_capacity = 16384;
  options.comparer = &hashmap_comparer;
  hashmap_create_ex(options, &t->br_map);

  // initial zstd
  t->zstd_cctx = ZSTD_createCCtx();
  assert(t->zstd_cctx);
  t->zstd_output_buffer_size = ZSTD_CStreamOutSize();
  t->zstd_output_buffer = malloc(t->zstd_output_buffer_size);
  assert(t->zstd_output_buffer);

  drmgr_set_tls_field(drcontext, tls_idx, (void *)t);
}

static void event_thread_exit(void *drcontext) {
  struct tls *t = (struct tls *)drmgr_get_tls_field(drcontext, tls_idx);
  assert(t);

  // finish entries
  // https://github.com/facebook/zstd/blob/dev/examples/streaming_compression.c
  ZSTD_EndDirective mode = ZSTD_e_end;
  ZSTD_inBuffer input = {t->write_buffer, sizeof(struct entry) * t->buffer_size,
                         0};
  int finished;
  do {
    ZSTD_outBuffer output = {t->zstd_output_buffer, t->zstd_output_buffer_size,
                             0};
    size_t remaining =
        ZSTD_compressStream2(t->zstd_cctx, &output, &input, mode);
    assert(!ZSTD_isError(remaining));
    dr_write_file(t->log, t->zstd_output_buffer, output.pos);
    finished = remaining == 0;
  } while (!finished);
  t->buffer_size = 0;

  struct file_header header;
  header.magic = MAGIC;
  header.version = 0;
  header.num_entries = t->num_entries;
  header.entries_offset = sizeof(struct file_header);
  header.entries_size = dr_file_tell(t->log) - header.entries_offset;

  // write branches array
  header.num_branches = t->num_brs;
  header.branches_offset = dr_file_tell(t->log);
  dr_write_file(t->log, t->brs, sizeof(struct branch) * t->num_brs);

  // write image content
  for (int i = 0; i < t->num_images; i++) {
    t->images[i].data_offset = dr_file_tell(t->log);

    // if the file exists in file system, use the full image instead;
    // otherwise we may get partial file
    file_t image = dr_open_file(t->images[i].filename, DR_FILE_CLOSE_ON_FORK |
                                                           DR_FILE_ALLOW_LARGE |
                                                           DR_FILE_READ);
    if (image != INVALID_FILE) {
      char buffer[1024];
      t->images[i].data_size = 0;
      while (true) {
        ssize_t size = dr_read_file(image, buffer, sizeof(buffer));
        if (size == 0) {
          break;
        } else if (size > 0) {
          dr_write_file(t->log, buffer, size);
          t->images[i].data_size += size;
        }
      }
      dr_close_file(image);
    } else {
      // if it doesn't exist (e.g. vdso), use existing data in memory
      dr_write_file(t->log, (void *)t->images[i].start, t->images[i].len);
      t->images[i].data_size = t->images[i].len;
    }
  }

  // write images array
  header.num_images = t->num_images;
  header.images_offset = dr_file_tell(t->log);
  dr_write_file(t->log, t->images, sizeof(struct image) * t->num_images);

  // write header
  dr_file_seek(t->log, 0, DR_SEEK_SET);
  dr_write_file(t->log, &header, sizeof(struct file_header));

  dr_close_file(t->log);
  fprintf(stderr, "Finished writing log\n");
}

static void event_exit(void) {
  dr_log(NULL, DR_LOG_ALL, 1, "Client 'brtrace' exiting");
#ifdef SHOW_RESULTS
  if (dr_is_notify_on())
    dr_fprintf(STDERR, "Client 'brtrace' exiting\n");
#endif
  if (!drmgr_unregister_bb_insertion_event(event_app_instruction) ||
      !drmgr_unregister_tls_field(tls_idx))
    DR_ASSERT(false);
  drmgr_exit();
}

static void event_module_load(void *drcontext, const module_data_t *info,
                              bool loaded) {
  struct tls *t = (struct tls *)drmgr_get_tls_field(drcontext, tls_idx);
  assert(t);

  struct image new_image;
  new_image.start = (uint64_t)info->start;
  new_image.len = (uint64_t)info->end - (uint64_t)info->start;
  fprintf(stderr, "Image %s loaded at 0x%lx\n", info->full_path, info->start);
  snprintf(new_image.filename, sizeof(new_image.filename), "%s",
           info->full_path);

  assert(t->num_images < MAX_IMAGES);
  t->images[t->num_images++] = new_image;
}

DR_EXPORT
void dr_client_main(client_id_t id, int argc, const char *argv[]) {
  dr_set_client_name("brtrace", "");
  dr_log(NULL, DR_LOG_ALL, 1, "Client 'brtrace' initializing");

  if (argc == 2) {
    snprintf(log_file_name, sizeof(log_file_name), "%s", argv[1]);
  }
  dr_log(NULL, DR_LOG_ALL, 1, "Output trace is written at %s", log_file_name);

  drmgr_init();

  client_id = id;
  tls_idx = drmgr_register_tls_field();

  drmgr_register_exit_event(event_exit);
  if (!drmgr_register_module_load_event(event_module_load) ||
      !drmgr_register_thread_init_event(event_thread_init) ||
      !drmgr_register_thread_exit_event(event_thread_exit) ||
      !drmgr_register_bb_instrumentation_event(NULL, event_app_instruction,
                                               NULL))
    DR_ASSERT(false);

#ifdef SHOW_RESULTS
  if (dr_is_notify_on()) {
#ifdef WINDOWS
    dr_enable_console_printing();
#endif /* WINDOWS */
    dr_fprintf(STDERR, "Client 'brtrace' is running\n");
  }
#endif /* SHOW_RESULTS */
}
