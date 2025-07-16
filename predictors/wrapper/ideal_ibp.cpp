#include "ideal_ibp.h"
#include "cbp-experiments/predictors/wrapper/utils.h"

IdealIBP::IdealIBP() {}

uint64_t IdealIBP::get_indirect_branch_prediction(uint64_t pc, branch_type type,
                                                  uint64_t groundtruth) {
  return groundtruth;
}

void IdealIBP::update_indirect_branch_predictor(uint64_t pc, branch_type type,
                                                bool taken,
                                                uint64_t branch_target) {}
