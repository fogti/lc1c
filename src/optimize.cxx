#include "optimize.hpp"
#include "utils.hpp"

#include <algorithm>
#include <functional>
#include <iostream>
#include <list>
#include <unordered_map>
#include <utility>

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
  static const unordered_map<uint16_t, function<void (optdat_t &o)>> jt = {
    // opposite ops
    { 0x1415, fn_erase_both  }, { 0x1514, fn_erase_both  },
    { 0x1717, fn_erase_both  },
    // direct overwrite reg a
    { 0x1010, fn_erase_first }, { 0x1710, fn_erase_first },
    { 0x1410, fn_erase_first }, { 0x1510, fn_erase_first },
    // direct overwrite reg b
    { 0x1111, fn_erase_first },
    { 0x1311, fn_erase_first }, { 0x1113, fn_erase_first },
    // no-ops
    { 0x1616, fn_erase_secnd }, { 0x1313, fn_erase_secnd },
    { 0x1818, fn_erase_secnd },
    { 0x1819, fn_erase_secnd }, { 0x1919, fn_erase_secnd },
    { 0x181a, fn_erase_secnd }, { 0x1a1a, fn_erase_secnd },
    { 0x1c1c, fn_erase_secnd },
    { 0x1c1b, fn_erase_secnd }, { 0x1c18, fn_erase_secnd },
    { 0x1f1f, fn_erase_secnd }, { 0x1f18, fn_erase_secnd },
    // possible opposite ops
    { 0x1d1e, fn_erase_rr2   }, { 0x1e1d, fn_erase_rr2   },
    // swaps for easier optimizations (independent ops)
    { 0x1117, fn_swap },
    // translations
    { 0x1b1c, [](optdat_t &o) { // tail-call
      (o.it - 1)->cmd = LC1CMD_JMP;
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
      auto fnit = jt.find(((oit - 1)->cmd << 8) + oit->cmd);
      if(fnit == jt.end()) goto do_cont;
      if(env.flag_verbose) cerr << "optimize " << lc1cmd2str((oit - 1)->cmd) << lc1cmd2str(oit->cmd) << " @ " << (oit - stmts.begin()) << '\n';
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

// NOTE: optimize_deep splits the code into basic blocks and tries to reorder the asm statements
// optimize_deep part prefix: zdo_
struct zdo_basic_block {
  // entry points
  vector<string> entp_labels;
  size_t entp_cnt;
  bool is_jmptrg;

  // exit points, exip_norm == nullptr means 'HLT' after block
  zdo_basic_block *exip_norm, *exip_ovfl, *exip_sign;

  // block body
  vector<lc1stmt> body;

  // methods
  zdo_basic_block()
    : entp_cnt(0), is_jmptrg(false), exip_norm(nullptr), exip_ovfl(nullptr), exip_sign(nullptr) { }

  ~zdo_basic_block() {
    unref_all_exips();
  }

  bool unused() const noexcept
    { return !entp_cnt; }
  bool empty()  const noexcept
    { return !exip_norm && !exip_ovfl && !exip_sign && body.empty(); }

  void shrink_to_fit() noexcept {
    entp_labels.shrink_to_fit();
    body.shrink_to_fit();
  }

  static void unref_exip(zdo_basic_block **exipptr) {
    if(!exipptr || !(*exipptr)) return;
    auto &epcnt = (*exipptr)->entp_cnt;
    if(!epcnt) {
      cerr << "optimize_deep:zdo_basic_block::unref_exip: got illegal state '!exipptr->entp_cnt'\n";
    } else {
      --epcnt;
    }
    *exipptr = nullptr;
  }

  void unref_all_exips() {
    unref_exip(&exip_norm);
    unref_exip(&exip_ovfl);
    unref_exip(&exip_sign);
  }
};

/* The optimize_deep phase is composed out of three sub-phases
   1. (init)    split the source code into basic blocks, compute use counts
   2. (run)     in a loop: optimize basic blocks and delete unused basic blocks
   3. (fini)    recompose the program from left-over basic blocks, overwrites env
 */

class zdo_data {
  // entry point = blocks.front()
  list<zdo_basic_block> blocks;
  bool flag_verbose;
  size_t anon_lblid;

 public:
  zdo_data(): anon_lblid(0) { }

  void do_init(const lc1cenv &env) {
    // exit points cache
    typedef unordered_map<string, vector<zdo_basic_block*>> exc_t;
    exc_t exc_jmp, exc_jpo, exc_jps, exc_dests;

    const auto fn_jump_regexc = [this, &exc_jmp](const lc1stmt &i, exc_t &exc) {
      auto *olbbptr = &blocks.back();
      exc[i.a_s].emplace_back(olbbptr);
      blocks.emplace_back();
      if(&exc != &exc_jmp && !olbbptr->exip_norm) {
        olbbptr->exip_norm = &blocks.back();
        blocks.back().entp_cnt++;
      }
    };

    const auto fn_jump_resolve = [](const string &trglbl, zdo_basic_block *jmpdest, exc_t &exc, zdo_basic_block* zdo_basic_block::*exipptr) {
      const auto it = exc.find(trglbl);
      if(it == exc.end()) return;
      bool any_jmptrg = false;
      for(zdo_basic_block *i : it->second) {
        if(i->is_jmptrg) any_jmptrg = true;
        i->*exipptr = jmpdest;
      }
      if(!it->second.empty()) {
        jmpdest->entp_cnt += it->second.size();
        jmpdest->is_jmptrg = any_jmptrg;
      }
      exc.erase(it);
    };

    const unordered_map<uint8_t, function<void (const lc1stmt &)>> jt = {
#define ZDO_JTFN [&, this](const lc1stmt &i)
      { LC1CMD_JMP, ZDO_JTFN { fn_jump_regexc(i, exc_jmp); }},
      { LC1CMD_JPS, ZDO_JTFN { fn_jump_regexc(i, exc_jps); }},
      { LC1CMD_JPO, ZDO_JTFN { fn_jump_regexc(i, exc_jpo); }},
      { LC1CMD_LABEL, ZDO_JTFN {
        if(!blocks.back().empty()) {
          auto &olbb = blocks.back();
          blocks.emplace_back();
          if(!olbb.exip_norm) olbb.exip_norm = &blocks.back();
        }
        exc_dests[i.a_s].emplace_back(&blocks.back());
        blocks.back().entp_labels.emplace_back(i.a_s);
      }},
      { LC1CMD_HLT, ZDO_JTFN {
        blocks.back().exip_norm = 0;
        blocks.emplace_back();
      }},
#undef ZDO_JTFN
    };

    // we don't care about env.compout
    flag_verbose = env.flag_verbose;

    // create the entry point
    blocks.emplace_back();
    blocks.front().entp_cnt++;
    blocks.front().is_jmptrg = true;

    // insert data
    for(const auto &i : env.stmts) {
      const auto jtit = jt.find(i.cmd);
      if(jtit != jt.end())
        jtit->second(i);
      else
        blocks.back().body.emplace_back(i);
    }

    // cleanup data
    for(auto &i : blocks) {
      if(i.entp_labels.empty())
        i.entp_labels.emplace_back("%" + to_string(anon_lblid++));
      i.shrink_to_fit();
      for(auto &j : i.body) {
        if(j.atyp != lc1atyp::LABEL) continue;
        auto &dstv = exc_dests[j.a_s];
        if(!dstv.empty() && dstv.front())
          dstv.front()->entp_cnt++;
      }
    }

    // resolve jumps
    for(const auto &i : exc_dests) {
      if(i.second.size() != 1 && flag_verbose)
        cerr << "optimize_deep: ERROR: got redefinition of label '" << i.first << "' (" << i.second.size() << " times)\n";
      if(i.second.empty()) continue;
      zdo_basic_block *jmpdest = i.second.back();
      fn_jump_resolve(i.first, jmpdest, exc_jmp, &zdo_basic_block::exip_norm);
      fn_jump_resolve(i.first, jmpdest, exc_jpo, &zdo_basic_block::exip_ovfl);
      fn_jump_resolve(i.first, jmpdest, exc_jps, &zdo_basic_block::exip_sign);
    }
  }

  void do_run() {
    static const auto fn_print_exip = [](const char *exipname, zdo_basic_block * exipptr) {
      if(!exipptr) return;
      cerr << " X" << exipname << ": ";
      if(!exipptr->entp_labels.empty())
        cerr << exipptr->entp_labels.front();
      cerr << '\n';
    };

    // search for blocks with use_count == 1, and "used_from block" direct jump to (1 <-), simplify
    for(auto &i : blocks) {
      if(!(i.exip_norm && !i.exip_ovfl && !i.exip_sign && i.exip_norm->entp_cnt == 1))
        continue;
      auto &othblk = *i.exip_norm;
      auto &othv = othblk.body;
      i.body.reserve(i.body.size() + othv.size());
      i.body.insert(i.body.end(), othv.begin(), othv.end());
      othv.clear();
      // update jumps
      i.exip_ovfl = othblk.exip_ovfl;
      i.exip_sign = othblk.exip_sign;
      i.unref_exip(&i.exip_norm);
      i.exip_norm = othblk.exip_norm;
    }

    if(flag_verbose) {
      // debug print
      cerr << "optimize_deep: DEBUG: current blocks...\n";

      size_t n = 0;
      for(const auto &i : blocks) {
        cerr << "BB " << n++ << " used by " << i.entp_cnt << '\n';
        for(const auto &lbl : i.entp_labels)
          cerr << " LBL " << lbl << '\n';
        for(const auto &cmd : i.body)
          cerr << "     " << cmd.to_string() << '\n';
        fn_print_exip("o", i.exip_ovfl);
        fn_print_exip("s", i.exip_sign);
        fn_print_exip("-", i.exip_norm);
      }
    }
  }

  void do_cleanup() {
    const auto ie = blocks.end();
    blocks.erase(remove_if(blocks.begin(), ie,
      [](const zdo_basic_block &i) { return i.unused(); }), ie);
  }

  void do_fini(lc1cenv &env) {
    // resolve is_jmptrg
    static const auto mark_jmptrg = [](zdo_basic_block *exipptr) {
      if(exipptr) exipptr->is_jmptrg = true;
    };
    for(auto it = blocks.begin(); it != blocks.end(); ++it) {
      auto &i = *it;
      if(i.is_jmptrg) {
        mark_jmptrg(i.exip_ovfl);
        mark_jmptrg(i.exip_sign);
        mark_jmptrg(i.exip_norm);
      }
    }

    auto &stmts = env.stmts;
    stmts.clear();
    for(auto it = blocks.begin(); it != blocks.end(); ++it) {
      auto &i = *it;
      stmts.reserve(stmts.size() + i.entp_labels.size() + i.body.size() + 1);

      // create labels
      lc1stmt stmt;
      stmt.cmd = LC1CMD_LABEL;
      stmt.atyp = lc1atyp::LABEL;
      for(const auto &lbl : i.entp_labels) {
        // NOTE: don't move from lbl, as we need it to resolve jumps later
        stmt.a_s = lbl;
        // WARNING: here we use interna of the lc1stmt move constructor
        stmts.emplace_back(move(stmt));
      }

      // append commands
      stmts.insert(stmts.end(), i.body.begin(), i.body.end());
      if(i.exip_ovfl) {
        stmt.cmd = LC1CMD_JPO;
        stmt.a_s = i.exip_ovfl->entp_labels.front();
        stmts.emplace_back(move(stmt));
      }
      if(i.exip_sign) {
        stmt.cmd = LC1CMD_JPS;
        stmt.a_s = i.exip_sign->entp_labels.front();
        stmts.emplace_back(move(stmt));
      }
      const auto next_it = ([it] { auto it2 = it; return ++it2; })();
      if(!i.exip_norm) {
        // if HLT is the last command, it's implicit
        if(i.is_jmptrg) {
          stmt.cmd = LC1CMD_HLT;
          stmt.atyp = lc1atyp::NONE;
          stmt.a_s = {};
          stmts.emplace_back(move(stmt));
        }
      } else if(next_it == blocks.end() || i.exip_norm != &(*next_it)) {
        // jump after this block (non-linear flow)
        stmt.cmd = LC1CMD_JMP;
        stmt.a_s = i.exip_norm->entp_labels.front();
        stmts.emplace_back(move(stmt));
      } else {
        // no jump needed
      }

      // unref all exit points to prevent use-after-free
      i.unref_all_exips();
    }
  }

  size_t get_block_count() const noexcept { return blocks.size(); }
};

void optimize_deep(lc1cenv &env) {
  zdo_data dat;
  dat.do_init(env);
  dat.do_cleanup();
  while(true) {
    const size_t bcnt = dat.get_block_count();
    dat.do_run();
    dat.do_cleanup();
    if(bcnt == dat.get_block_count()) break;
  }
  dat.do_fini(env);
}

// NOTE: optimize_idconsts is the latest optimize phase, which works
//       on mostly resolved addresses to find raw byte constants in the asm-code
//       for re-usage
static void mark_idconst(const lc1cenv &env, labels_t &labels, const int value) {
  const uint8_t val_lo = value & 63, val_hi = value >> 6;
  const uint8_t needed_cmd = 0x10 + val_hi;
  if(!cmd2has_arg(needed_cmd) && val_lo) return;

  auto &stmts = env.stmts;
  auto it = stmts.begin();
  size_t stcnt = 0;
  for(; it != stmts.end(); ++it) {
    if(it->cmd != needed_cmd || it->atyp != lc1atyp::ABSOLUTE)
      goto inc_cont;
    if(it->a_i == val_lo)
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
