#include "interface.h"
#include "andre_seznec_tage_cookbook.h"
#include "andre_seznec_tage_sc_l_64kb.h"
#include "andre_seznec_tage_sc_l_64kb_only_tage.h"
#include "andre_seznec_tage_sc_l_8kb.h"
#include "andre_seznec_tage_sc_l_8kb_only_tage.h"
#include "andre_seznec_unlimited.h"
#include "cbp-experiments/src/lib.rs.h"
#include <memory>

std::unique_ptr<Predictor> new_predictor(rust::Str name) {
  if (false) {
  }
#define PREDICTOR(predictor, class)                                            \
  else if (name == #predictor) {                                               \
    return std::unique_ptr<Predictor>(new class);                              \
  }
#include "predictors.h"
#undef PREDICTOR
  return nullptr;
}

std::unique_ptr<std::vector<std::string>> list_predictors() {
  std::vector<std::string> result = {

#define PREDICTOR(predictor, class) #predictor,
#include "predictors.h"
#undef PREDICTOR
  };
  return std::unique_ptr<std::vector<std::string>>(
      new std::vector<std::string>(result));
}