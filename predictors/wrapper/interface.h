#pragma once
#include "rust/cxx.h"
#include <memory>

enum class BranchType : uint32_t;
typedef BranchType branch_type;

class Predictor {
public:
  Predictor() {}
  virtual bool get_prediction(uint64_t pc) = 0;
  virtual void update_predictor(uint64_t pc, branch_type type,
                                bool resolve_direction, bool predict_direction,
                                uint64_t branch_target) = 0;
  virtual void track_other_inst(uint64_t pc, branch_type type,
                                bool branch_taken, uint64_t branch_target) = 0;
};

std::unique_ptr<Predictor> new_predictor(rust::Str name);