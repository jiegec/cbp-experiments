#include "cbp-experiments/predictors/wrapper/interface.h"

namespace andre_seznec_tage_sc_l_8kb_only_tage {
class PREDICTOR;
};

class AndreSeznecTAGESCL8KBOnlyTAGE : public ConditionalBranchPredictor {
public:
  AndreSeznecTAGESCL8KBOnlyTAGE();
  bool get_conditonal_branch_prediction(uint64_t pc, bool groundtruth);
  void update_conditional_branch_predictor(uint64_t pc, branch_type type,
                                           bool resolve_direction,
                                           bool predict_direction,
                                           uint64_t branch_target);
  void update_conditional_branch_predictor_other_inst(uint64_t pc,
                                                      branch_type type,
                                                      bool branch_taken,
                                                      uint64_t branch_target);

private:
  andre_seznec_tage_sc_l_8kb_only_tage::PREDICTOR *impl;
};