#include "cbp-experiments/predictors/wrapper/interface.h"
#include "cbp-experiments/predictors/wrapper/utils.h"
#include "cbp-experiments/src/lib.rs.h"
#include "interface.h"

namespace andre_seznec_tage_sc_l_8kb {
#include "cbp-experiments/predictors/AndreSeznec-TAGE-SC-L-8KB/predictor.h"
};

OpType convert_type(branch_type type) {
  switch (type) {
  case BranchType::DirectJump:
    return OPTYPE_JMP_DIRECT_UNCOND;
  case BranchType::IndirectJump:
    return OPTYPE_JMP_INDIRECT_UNCOND;
  case BranchType::DirectCall:
    return OPTYPE_CALL_DIRECT_UNCOND;
  case BranchType::IndirectCall:
    return OPTYPE_CALL_INDIRECT_UNCOND;
  case BranchType::Return:
    return OPTYPE_RET_UNCOND;
  case BranchType::ConditionalDirectJump:
    return OPTYPE_JMP_DIRECT_COND;
  default:
    assert(false);
  }
}

class AndreSeznecTAGESCL8KB : public Predictor {
public:
  AndreSeznecTAGESCL8KB() {}
  bool get_prediction(uint64_t pc) { return impl.GetPrediction(pc); }
  void update_predictor(uint64_t pc, branch_type type, bool resolve_direction,
                        bool predict_direction, uint64_t branch_target) {
    impl.UpdatePredictor(pc, convert_type(type), resolve_direction,
                         predict_direction, branch_target);
  }
  void track_other_inst(uint64_t pc, branch_type type, bool branch_taken,
                        uint64_t branch_target) {
    impl.TrackOtherInst(pc, convert_type(type), branch_taken, branch_target);
  }

private:
  andre_seznec_tage_sc_l_8kb::PREDICTOR impl;
};

std::unique_ptr<Predictor> new_predictor(rust::Str name) {
  if (name == "AndreSeznec-TAGE-SC-L-8KB") {
    return std::unique_ptr<Predictor>(new AndreSeznecTAGESCL8KB);
  }
  return nullptr;
}