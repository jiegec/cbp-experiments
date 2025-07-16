#include "andre_seznec_tage_sc_192kb.h"
#include "cbp-experiments/predictors/wrapper/utils.h"

namespace andre_seznec_tage_sc_192kb {
#include "cbp-experiments/predictors/AndreSeznec-TAGE-SC-192KB/my_cond_branch_predictor.h"
};

AndreSeznecTAGESC192KB::AndreSeznecTAGESC192KB() {
  impl = new andre_seznec_tage_sc_192kb::CBP2025;
  seq_no = 0;
}

bool AndreSeznecTAGESC192KB::get_conditonal_branch_prediction(uint64_t pc) {
  return impl->predict(seq_no++, 0, pc);
}

static inline int convert_type_tage_sc_192kb(branch_type type) {
  switch (type) {
  case BranchType::DirectJump:
    return 0;
  case BranchType::IndirectJump:
    return 2;
  case BranchType::DirectCall:
    return 0;
  case BranchType::IndirectCall:
    return 2;
  case BranchType::Return:
    return 2;
  case BranchType::ConditionalDirectJump:
    return 1;
  default:
    assert(false);
  }
}

void AndreSeznecTAGESC192KB::update_conditional_branch_predictor(
    uint64_t pc, branch_type type, bool resolve_direction,
    bool predict_direction, uint64_t branch_target) {
  impl->HistoryUpdate(pc, convert_type_tage_sc_192kb(type), resolve_direction,
                      branch_target);
  impl->update(seq_no - 1, 0, pc, resolve_direction, predict_direction,
               branch_target);
}

void AndreSeznecTAGESC192KB::update_conditional_branch_predictor_other_inst(
    uint64_t pc, branch_type type, bool branch_taken, uint64_t branch_target) {
  impl->TrackOtherInst(pc, convert_type_tage_sc_192kb(type), branch_taken,
                       branch_target);
}
