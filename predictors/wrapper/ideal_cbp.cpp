#include "ideal_cbp.h"
#include "cbp-experiments/predictors/wrapper/utils.h"

IdealCBP::IdealCBP() {}

bool IdealCBP::get_conditonal_branch_prediction(uint64_t pc, bool groundtruth) {
  return groundtruth;
}

void IdealCBP::update_conditional_branch_predictor(uint64_t pc,
                                                   branch_type type,
                                                   bool resolve_direction,
                                                   bool predict_direction,
                                                   uint64_t branch_target) {}

void IdealCBP::update_conditional_branch_predictor_other_inst(
    uint64_t pc, branch_type type, bool branch_taken, uint64_t branch_target) {}
