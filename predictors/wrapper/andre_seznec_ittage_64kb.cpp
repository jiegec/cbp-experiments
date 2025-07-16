#include "andre_seznec_ittage_64kb.h"
#include "cbp-experiments/predictors/wrapper/utils.h"

// from cbp3 framework
#define IS_BR_CONDITIONAL (1 << 0) // conditional branch
#define IS_BR_INDIRECT (1 << 1)    // indirect branch
#define IS_BR_CALL (1 << 2)        // call
#define IS_BR_RETURN (1 << 3)      // return
#define IS_BR_OTHER (1 << 4)       // other branches

namespace andre_seznec_ittage_64kb {
#define INCLUDEPRED
#include "cbp-experiments/predictors/AndreSeznec-ITTAGE-64KB/predictor.h"
}; // namespace andre_seznec_ittage_64kb

AndreSeznecITTAGE64KB::AndreSeznecITTAGE64KB() {
  impl = new andre_seznec_ittage_64kb::my_predictor();
}

static inline int convert_type_ittage_64kb(branch_type type) {
  switch (type) {
  case BranchType::DirectJump:
    return IS_BR_OTHER;
  case BranchType::IndirectJump:
    return IS_BR_INDIRECT;
  case BranchType::DirectCall:
    return IS_BR_CALL;
  case BranchType::IndirectCall:
    return IS_BR_INDIRECT | IS_BR_CALL;
  case BranchType::Return:
    return IS_BR_RETURN;
  case BranchType::ConditionalDirectJump:
    return IS_BR_CONDITIONAL;
  default:
    assert(false);
  }
}

uint64_t AndreSeznecITTAGE64KB::get_indirect_branch_prediction(
    uint64_t pc, branch_type type, uint64_t groundtruth) {
  return impl->predict_brindirect(pc, convert_type_ittage_64kb(type));
}

void AndreSeznecITTAGE64KB::update_indirect_branch_predictor(
    uint64_t pc, branch_type type, bool taken, uint64_t branch_target) {
  // fetch
  impl->FetchHistoryUpdate(pc, convert_type_ittage_64kb(type), taken,
                           branch_target);
  // retire
  impl->update_brindirect(pc, convert_type_ittage_64kb(type), taken,
                          branch_target);
}
