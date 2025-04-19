#include "cbp-experiments/predictors/wrapper/interface.h"

namespace andre_seznec_unlimited {
class PREDICTOR;
};

class AndreSeznecUnlimited : public Predictor {
public:
  AndreSeznecUnlimited();
  bool get_prediction(uint64_t pc);
  void update_predictor(uint64_t pc, branch_type type, bool resolve_direction,
                        bool predict_direction, uint64_t branch_target);
  void track_other_inst(uint64_t pc, branch_type type, bool branch_taken,
                        uint64_t branch_target);

private:
  andre_seznec_unlimited::PREDICTOR *impl;
};