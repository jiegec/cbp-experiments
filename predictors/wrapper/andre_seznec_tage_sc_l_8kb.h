#include "cbp-experiments/predictors/wrapper/interface.h"

namespace andre_seznec_tage_sc_l_8kb {
class PREDICTOR;
};

class AndreSeznecTAGESCL8KB : public Predictor {
public:
  AndreSeznecTAGESCL8KB();
  bool get_prediction(uint64_t pc);
  void update_predictor(uint64_t pc, branch_type type, bool resolve_direction,
                        bool predict_direction, uint64_t branch_target);
  void track_other_inst(uint64_t pc, branch_type type, bool branch_taken,
                        uint64_t branch_target);

private:
  andre_seznec_tage_sc_l_8kb::PREDICTOR *impl;
};