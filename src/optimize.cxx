#include "optimize.hpp"
#include "utils.hpp"

#include <algorithm>
#include <functional>
#include <iostream>

using namespace std;

void optimize(lc1cenv &env) {
  typedef vector<lc1stmt>::iterator it_t;
  struct optdat_t {
    it_t it;
    // erase[0] = prev; erase[1] = cur
    bool erase[2];
  };
  static const auto fn_erase_both = [](optdat_t &o) {
    o.erase[0] = o.erase[1] = true;
  };
  static const auto fn_erase_first = [](optdat_t &o)
    { o.erase[0] = true; };
  static const auto fn_erase_secnd = [](optdat_t &o)
    { o.erase[1] = true; };
  static const auto fn_erase_rr2 = [](optdat_t &o) {
    if((o.it - 1)->a_i == o.it->a_i)
      fn_erase_both(o);
  };
  static const auto fn_swap = [](optdat_t &o) {
    swap(*(o.it), *(o.it - 1));
  };
  static const unordered_map<string, function<void (optdat_t &o)>> jt = {
    // opposite ops
    { "addsub", fn_erase_both  }, { "subadd", fn_erase_both  },
    { "notnot", fn_erase_both  },
    // direct overwrite reg a
    { "ldalda", fn_erase_first }, { "notlda", fn_erase_first },
    { "addlda", fn_erase_first }, { "sublda", fn_erase_first },
    // direct overwrite reg b
    { "ldbldb", fn_erase_first },
    { "mabldb", fn_erase_first }, { "ldbmab", fn_erase_first },
    // no-ops
    { "andand", fn_erase_secnd }, { "mabmab", fn_erase_secnd },
    { "jmpjmp", fn_erase_secnd },
    { "jmpjps", fn_erase_secnd }, { "jpsjps", fn_erase_secnd },
    { "jmpjpo", fn_erase_secnd }, { "jpojpo", fn_erase_secnd },
    { "retret", fn_erase_secnd },
    { "retcal", fn_erase_secnd }, { "retjmp", fn_erase_secnd },
    { "hlthlt", fn_erase_secnd }, { "hltjmp", fn_erase_secnd },
    // possible opposite ops
    { "rrarla", fn_erase_rr2   }, { "rlarra", fn_erase_rr2   },
    // swaps for easier optimizations (independent ops)
    { "ldbnot", fn_swap },
    // translations
    { "calret", [](optdat_t &o) { // tail-call
      strncpy((o.it - 1)->cmd, "jmp", 4);
      o.erase[1] = true;
    }},
  };
  auto &stmts = env.stmts;
  if(stmts.size() < 2) return;
  optdat_t od;
  auto &oit = od.it;

  // mark relative addresses as non-optimizable (might break relative addresses)
  {
    const auto itb = stmts.begin(), ite = stmts.end();
    for(oit = itb; oit != ite; ++oit) {
      if(oit->atyp != lc1atyp::RELATIVE)
        continue;
      const auto offset = oit->a_i; // signed offset from current oit
      if(!offset) {
        oit->do_ignore = true;
        continue;
      }

      auto trg = oit + offset;
      trg = (offset > 0) ? std::min(ite, trg + 1) : std::max(itb, trg);
      auto itstart = oit, itstop = trg;
      if(itstart > itstop) swap(itstart, itstop);
      for(; itstart != itstop; ++itstart)
        itstart->do_ignore = true;
    }
  }

  while(stmts.size() > 1) {
    oit = stmts.begin() + 1;
    // copy stmts into temp buffer
    const size_t stcnt = stmts.size();
    if(stmts.front().do_ignore) ++oit;

    while(oit != stmts.end()) {
      if(oit->do_ignore) {
        ++oit;
        while(oit != stmts.end() && oit->do_ignore)
          ++oit;
        if(oit == stmts.end())
          break;
        // skip the element past the ignored element, so we don't mangle it in optimizations
        ++oit;
        continue;
      }
      decltype(jt)::const_iterator fnit;
      {
        string fname = (oit - 1)->cmd;
        fname += oit->cmd;
        fnit = jt.find(fname);
        if(fnit == jt.end()) goto do_cont;
        if(env.flag_verbose) cerr << "optimize " << fname << " @ " << (oit - stmts.begin()) << '\n';
      }
      od.erase[0] = od.erase[1] = false;
      fnit->second(od);
      if(od.erase[0] || od.erase[1]) {
        oit = stmts.erase(oit - od.erase[0], oit + od.erase[1]);
        continue;
      }

      do_cont:
        ++oit;
    }

    if(stcnt == stmts.size()) break;
  }
}

static void mark_idconst(const lc1cenv &env, labels_t &labels, const int value) {
  static const vector<std::string> cmd2hi = {
    "LDA", "LDB", "MOV", "MAB", "ADD", "SUB", "AND", "NOT",
    "JMP", "JPS", "JPO", "CAL", "RET", "RRA", "RLA", "HLT"
  };

  const uint8_t val_lo = value & 63, val_hi = value >> 6;
  string needed_cmd;
  try { needed_cmd = cmd2hi.at(val_hi); }
  catch(...) { return; }
  const bool cmd_has_arg = ([&needed_cmd] {
    string cmd = needed_cmd;
    str_lower(cmd);
    return cmd2has_arg(cmd);
  })();
  if(!cmd_has_arg && val_lo) return;

  auto &stmts = env.stmts;
  auto it = stmts.begin();
  size_t stcnt = 0;
  for(; it != stmts.end(); ++it) {
    if(it->do_ignore)
      continue;
    if(!it->cmd || needed_cmd != it->cmd || it->atyp != lc1atyp::ABSOLUTE)
      goto inc_cont;
    if(!cmd_has_arg || it->a_i == val_lo)
      break;
    inc_cont:
      ++stcnt;
  }
  if(it != stmts.end()) {
    if(env.flag_verbose) cerr << "optimize: re-use existing const " << value << " @ " << stcnt << '\n';
    labels["$" + to_string(value)] = stcnt;
  }
}

void optimize_idconsts(const lc1cenv &env, labels_t &labels) {
  vector<int> idc_vals;
  for(const auto &i : env.stmts)
    if(i.atyp == lc1atyp::IDCONST)
      idc_vals.emplace_back(i.a_i);
  // uniquify idc_vals
  sort(idc_vals.begin(), idc_vals.end());
  idc_vals.erase(unique(idc_vals.begin(), idc_vals.end()), idc_vals.end());
  for(const auto i : idc_vals)
    mark_idconst(env, labels, i);
}
