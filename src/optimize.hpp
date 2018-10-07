#pragma once
#include "types.hpp"

void optimize(lc1cenv &env);
void optimize_deep(lc1cenv &env);
void optimize_idconsts(const lc1cenv &env, labels_t &labels);
