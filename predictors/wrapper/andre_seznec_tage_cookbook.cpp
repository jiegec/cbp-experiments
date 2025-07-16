#include "andre_seznec_tage_cookbook.h"
#include "cbp-experiments/predictors/wrapper/utils.h"

namespace andre_seznec_tage_cookbook {
#include "cbp-experiments/predictors/AndreSeznec-TAGE-Cookbook/predictor.h"
};

AndreSeznecTAGECookbook::AndreSeznecTAGECookbook() {
  impl = new andre_seznec_tage_cookbook::PREDICTOR;
}

bool AndreSeznecTAGECookbook::get_conditonal_branch_prediction(
    uint64_t pc, bool groundtruth) {
  return impl->GetPrediction(pc);
}

void AndreSeznecTAGECookbook::update_conditional_branch_predictor(
    uint64_t pc, branch_type type, bool resolve_direction,
    bool predict_direction, uint64_t branch_target) {
  impl->UpdatePredictor(pc, convert_type(type), resolve_direction,
                        predict_direction, branch_target);
}

void AndreSeznecTAGECookbook::update_conditional_branch_predictor_other_inst(
    uint64_t pc, branch_type type, bool branch_taken, uint64_t branch_target) {
  impl->TrackOtherInst(pc, convert_type(type), branch_taken, branch_target);
}
