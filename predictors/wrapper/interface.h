#pragma once
#include "rust/cxx.h"
#include <memory>
#include <vector>

enum class BranchType : uint32_t;
typedef BranchType branch_type;

class ConditionalBranchPredictor {
public:
  ConditionalBranchPredictor() {}
  virtual bool get_conditional_branch_prediction(uint64_t pc,
                                                 bool groundtruth) = 0;
  virtual void update_conditional_branch_predictor(uint64_t pc,
                                                   branch_type type,
                                                   bool resolve_direction,
                                                   bool predict_direction,
                                                   uint64_t branch_target) = 0;
  virtual void
  update_conditional_branch_predictor_other_inst(uint64_t pc, branch_type type,
                                                 bool branch_taken,
                                                 uint64_t branch_target) = 0;
};

std::unique_ptr<ConditionalBranchPredictor>
new_conditional_branch_predictor(rust::Str name);
std::unique_ptr<std::vector<std::string>> list_conditional_branch_predictors();

class IndirectBranchPredictor {
public:
  IndirectBranchPredictor() {}
  virtual uint64_t get_indirect_branch_prediction(uint64_t pc, branch_type type,
                                                  uint64_t groundtruth) = 0;
  virtual void update_indirect_branch_predictor(uint64_t pc, branch_type type,
                                                bool branch_taken,
                                                uint64_t branch_target) = 0;
};

std::unique_ptr<IndirectBranchPredictor>
new_indirect_branch_predictor(rust::Str name);
std::unique_ptr<std::vector<std::string>> list_indirect_branch_predictors();
