#include "andre_seznec_unlimited.h"
#include "cbp-experiments/predictors/wrapper/utils.h"

namespace andre_seznec_unlimited {
#include "cbp-experiments/predictors/AndreSeznec-Unlimited/predictor.cc"
#include "cbp-experiments/predictors/AndreSeznec-Unlimited/predictor.h"
}; // namespace andre_seznec_unlimited

AndreSeznecUnlimited::AndreSeznecUnlimited() {
  impl = new andre_seznec_unlimited::PREDICTOR;
}

bool AndreSeznecUnlimited::get_conditional_branch_prediction(uint64_t pc, bool groundtruth) {
  return impl->GetPrediction(pc);
}

void AndreSeznecUnlimited::update_conditional_branch_predictor(
    uint64_t pc, branch_type type, bool resolve_direction,
    bool predict_direction, uint64_t branch_target) {
  impl->UpdatePredictor(pc, convert_type(type), resolve_direction,
                        predict_direction, branch_target);
}

void AndreSeznecUnlimited::update_conditional_branch_predictor_other_inst(
    uint64_t pc, branch_type type, bool branch_taken, uint64_t branch_target) {
  impl->TrackOtherInst(pc, convert_type(type), branch_taken, branch_target);
}
