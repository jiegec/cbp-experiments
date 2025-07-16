#include "interface.h"
#include "cbp-experiments/src/lib.rs.h"
#include <memory>

#include "andre_seznec_tage_cookbook.h"
#include "andre_seznec_tage_sc_192kb.h"
#include "andre_seznec_tage_sc_l_64kb.h"
#include "andre_seznec_tage_sc_l_64kb_only_tage.h"
#include "andre_seznec_tage_sc_l_8kb.h"
#include "andre_seznec_tage_sc_l_8kb_only_tage.h"
#include "andre_seznec_ittage_64kb.h"
#include "andre_seznec_unlimited.h"
#include "ideal_cbp.h"
#include "ideal_ibp.h"

std::unique_ptr<ConditionalBranchPredictor>
new_conditional_branch_predictor(rust::Str name) {
  if (false) {
  }
#define PREDICTOR(predictor, class)                                            \
  else if (name == #predictor) {                                               \
    return std::unique_ptr<ConditionalBranchPredictor>(new class);             \
  }
#include "conditional_branch_predictors.h"
#undef PREDICTOR
  return nullptr;
}

std::unique_ptr<std::vector<std::string>> list_conditional_branch_predictors() {
  std::vector<std::string> result = {
#define PREDICTOR(predictor, class) #predictor,
#include "conditional_branch_predictors.h"
#undef PREDICTOR
  };
  return std::unique_ptr<std::vector<std::string>>(
      new std::vector<std::string>(result));
}

std::unique_ptr<IndirectBranchPredictor>
new_indirect_branch_predictor(rust::Str name) {
  if (false) {
  }
#define PREDICTOR(predictor, class)                                            \
  else if (name == #predictor) {                                               \
    return std::unique_ptr<IndirectBranchPredictor>(new class);                \
  }
#include "indirect_branch_predictors.h"
#undef PREDICTOR
  return nullptr;
}

std::unique_ptr<std::vector<std::string>> list_indirect_branch_predictors() {
  std::vector<std::string> result = {

#define PREDICTOR(predictor, class) #predictor,
#include "indirect_branch_predictors.h"
#undef PREDICTOR
  };
  return std::unique_ptr<std::vector<std::string>>(
      new std::vector<std::string>(result));
}