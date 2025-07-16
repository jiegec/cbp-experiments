#include "andre_seznec_tage_sc_l_64kb.h"
#include "cbp-experiments/predictors/wrapper/utils.h"

namespace andre_seznec_tage_sc_l_64kb {
#include "cbp-experiments/predictors/AndreSeznec-TAGE-SC-L-64KB/predictor.h"
};

AndreSeznecTAGESCL64KB::AndreSeznecTAGESCL64KB() {
  impl = new andre_seznec_tage_sc_l_64kb::PREDICTOR;
}

bool AndreSeznecTAGESCL64KB::get_conditonal_branch_prediction(uint64_t pc, bool groundtruth) {
  return impl->GetPrediction(pc);
}

void AndreSeznecTAGESCL64KB::update_conditional_branch_predictor(
    uint64_t pc, branch_type type, bool resolve_direction,
    bool predict_direction, uint64_t branch_target) {
  impl->UpdatePredictor(pc, convert_type(type), resolve_direction,
                        predict_direction, branch_target);
}

void AndreSeznecTAGESCL64KB::update_conditional_branch_predictor_other_inst(
    uint64_t pc, branch_type type, bool branch_taken, uint64_t branch_target) {
  impl->TrackOtherInst(pc, convert_type(type), branch_taken, branch_target);
}
