#include "cbp-experiments/predictors/wrapper/interface.h"

class IdealIBP : public IndirectBranchPredictor {
public:
  IdealIBP();
  uint64_t get_indirect_branch_prediction(uint64_t pc, uint64_t groundtruth);
  void update_indirect_branch_predictor(uint64_t pc, branch_type type,
                                        bool taken, uint64_t branch_target);
};
