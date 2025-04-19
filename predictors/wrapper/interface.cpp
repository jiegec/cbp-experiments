#include "interface.h"
#include "andre_seznec_tage_sc_l_8kb.h"
#include "andre_seznec_tage_sc_l_64kb.h"
#include "cbp-experiments/src/lib.rs.h"

std::unique_ptr<Predictor> new_predictor(rust::Str name) {
  if (name == "AndreSeznec-TAGE-SC-L-8KB") {
    return std::unique_ptr<Predictor>(new AndreSeznecTAGESCL8KB);
  } else if (name == "AndreSeznec-TAGE-SC-L-64KB") {
    return std::unique_ptr<Predictor>(new AndreSeznecTAGESCL64KB);
  }
  return nullptr;
}