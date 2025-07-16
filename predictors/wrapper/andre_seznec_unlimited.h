#include "cbp-experiments/predictors/wrapper/interface.h"

namespace andre_seznec_unlimited {
class PREDICTOR;
};

class AndreSeznecUnlimited : public ConditionalBranchPredictor {
public:
  AndreSeznecUnlimited();
  bool get_conditonal_branch_prediction(uint64_t pc);
  void update_conditional_branch_predictor(uint64_t pc, branch_type type,
                                           bool resolve_direction,
                                           bool predict_direction,
                                           uint64_t branch_target);
  void update_conditional_branch_predictor_other_inst(uint64_t pc,
                                                      branch_type type,
                                                      bool branch_taken,
                                                      uint64_t branch_target);

private:
  andre_seznec_unlimited::PREDICTOR *impl;
};