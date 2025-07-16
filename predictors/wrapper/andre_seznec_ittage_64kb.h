#include "cbp-experiments/predictors/wrapper/interface.h"

namespace andre_seznec_ittage_64kb {
class my_predictor;
};

class AndreSeznecITTAGE64KB : public IndirectBranchPredictor {
public:
  AndreSeznecITTAGE64KB();
  uint64_t get_indirect_branch_prediction(uint64_t pc, branch_type type,
                                          uint64_t groundtruth);
  void update_indirect_branch_predictor(uint64_t pc, branch_type type,
                                        bool taken, uint64_t branch_target);

private:
  andre_seznec_ittage_64kb::my_predictor *impl;
};
