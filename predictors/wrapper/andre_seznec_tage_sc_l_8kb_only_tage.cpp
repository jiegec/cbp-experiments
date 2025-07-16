#include "andre_seznec_tage_sc_l_8kb_only_tage.h"
#include "cbp-experiments/predictors/wrapper/utils.h"

namespace andre_seznec_tage_sc_l_8kb_only_tage {
#include "cbp-experiments/predictors/AndreSeznec-TAGE-SC-L-8KB-Only-TAGE/predictor.h"
};

AndreSeznecTAGESCL8KBOnlyTAGE::AndreSeznecTAGESCL8KBOnlyTAGE() {
  impl = new andre_seznec_tage_sc_l_8kb_only_tage::PREDICTOR;
}

bool AndreSeznecTAGESCL8KBOnlyTAGE::get_conditional_branch_prediction(
    uint64_t pc, bool groundtruth) {
  return impl->GetPrediction(pc);
}

void AndreSeznecTAGESCL8KBOnlyTAGE::update_conditional_branch_predictor(
    uint64_t pc, branch_type type, bool resolve_direction,
    bool predict_direction, uint64_t branch_target) {
  impl->UpdatePredictor(pc, convert_type(type), resolve_direction,
                        predict_direction, branch_target);
}

void AndreSeznecTAGESCL8KBOnlyTAGE::
    update_conditional_branch_predictor_other_inst(uint64_t pc,
                                                   branch_type type,
                                                   bool branch_taken,
                                                   uint64_t branch_target) {
  impl->TrackOtherInst(pc, convert_type(type), branch_taken, branch_target);
}
