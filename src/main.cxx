#include "types.hpp"
#include "utils.hpp"

#include <ctype.h>
#include <algorithm>
#include <functional>
#include <fstream>
#include <iostream>
#include <sstream>
#include <set>
#include <stdexcept>

using namespace std;

static bool handle_option(lc1cenv &env, const char *opt, const char *optarg, bool &used_optarg) {
  const char *offending_arg = opt - 1;
  used_optarg = false;
  switch(*opt) {
    case 'o':
      // output file given
      if(opt[1] || !optarg) goto error;
      used_optarg = true;
      env.compout = new ofstream(optarg);
      if(!env.compout || !(*env.compout)) {
        offending_arg = optarg;
        if(env.compout) {
          delete env.compout;
          env.compout = 0;
        }
        goto error;
      }
      break;
    case 'U':
      if(opt[1]) goto error;
      env.flag_u2d = true;
      break;
    case 'v':
      if(opt[1]) goto error;
      env.flag_verbose = true;
      break;
    case 'O':
      if(strncmp(opt + 1, "0", 2)) goto error;
      env.flag_noopt = true;
      break;
    default:
      goto error;
  }

  return true;
 error:
  cerr << "lc1c: INVOCATION ERROR: invalid argument " << (offending_arg ? offending_arg : opt - 1) << '\n';
  return false;
}

static void read_file(lc1cenv &env, const char *file) {
  ifstream in(file);
  if(!in) {
    cerr << "lc1c: " << file << ": file not found\n";
    return;
  }

  size_t lineno = 0;
  string tok;
  lc1stmt stmt;
  while(getline(in, tok)) {
    // erase comments and spaces
    ++lineno;

    {
      const size_t tmp = tok.find(';');
      if(tmp != string::npos) tok.erase(tmp);
    }
    str_trim(tok);
    if(tok.empty()) continue;

    // parse line
    istringstream ss(tok);
    ss >> tok;
    if(tok.empty()) continue;
    if(tok.back() == ':') {
      // got label
      strcpy(stmt.cmd, "-L-");
      stmt.atyp = lc1atyp::LABEL;
      stmt.a_s  = move(tok);
      stmt.a_s.pop_back();
      stmt.a_i  = 0;
      env.stmts.emplace_back(move(stmt));
      ss >> tok;
      if(tok.empty()) continue;
    }

    string cmd = move(tok), errmsgtxt;
    stmt.do_ignore = (cmd.size() == 4 && cmd[3] != '*');
    if(!stmt.do_ignore && cmd.size() > 3) {
      errmsgtxt = "got invalid command '" + cmd + "'";
      goto on_cmd_error;
    }

    str_lower(cmd);
    getline(ss, tok);
    str_trim(tok);
    if(tok.empty() == cmd2has_arg(cmd)) {
      errmsgtxt = "invalid invocation of command '" + cmd + "'";
      goto on_cmd_error;
    }

    strncpy(stmt.cmd, cmd.c_str(), 4);

    if(!tok.empty()) {
      // parse arg addr type
      bool defmode = (cmd == "def");
      switch((stmt.atyp = arg2atyp(tok, defmode))) {
        case lc1atyp::INVALID:
          goto on_invalid_arg;
        case lc1atyp::LABEL:
          stmt.a_s = move(tok);
          stmt.a_i = 0;
          break;
        default:
          if(!defmode && tok.size() == 1)
            goto on_invalid_arg;
          stmt.a_s.clear();
          try {
            stmt.a_i = stoi(tok.substr(!defmode));
          } catch(...) {
            goto on_invalid_arg;
          }
          if(stmt.a_i < 0 && (!defmode && stmt.atyp != lc1atyp::RELATIVE))
            goto on_invalid_arg;
      }
      env.stmts.emplace_back(move(stmt));
      continue;
      on_invalid_arg:
        errmsgtxt = "invalid argument '" + tok + "'";
    } else {
      stmt.atyp = lc1atyp::NONE;
      stmt.a_s.clear();
      stmt.a_i = 0;
      env.stmts.emplace_back(move(stmt));
      continue;
    }
    on_cmd_error:
      file_parse_error(file, lineno, errmsgtxt);
  }
}

static void optimize(lc1cenv &env) {
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
  static const unordered_map<string, function<void (optdat_t &o)>> jt = {
    { "addsub", fn_erase_both },
    { "subadd", fn_erase_both },
    { "notnot", fn_erase_both },
    { "ldalda", fn_erase_first },
    { "ldbldb", fn_erase_first },
    { "andand", fn_erase_secnd },
    { "mabmab", fn_erase_secnd },
    { "jmpjmp", fn_erase_secnd },
    { "jmpjps", fn_erase_secnd },
    { "jmpjpo", fn_erase_secnd },
    { "jpsjps", fn_erase_secnd },
    { "jpojpo", fn_erase_secnd },
    { "calret", [](optdat_t &o) {
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

typedef unordered_map<std::string, size_t> labels_t;

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

static void optimize_idconsts(const lc1cenv &env, labels_t &labels) {
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

int main(int argc, char *argv[]) {
  if(argc == 1) {
    cerr << "lc1c [-o OUTPUT_FILE] SOURCE_FILE\n"
            "\noptions:\n"
            " -o  specfify an compilation output filename\n"
            " -U  unix2dos mode -- insert carriage returns after each compiled line\n"
            " -O0 disable optimizations\n"
            " -v  be more verbose\n"
            "\nreturn codes:\n"
            "  0  success\n"
            "  1  invalid input data or arguments\n"
            "  2  internal error\n";
    return 1;
  }

  lc1cenv env;
  env.compout = &cout;
  env.flag_u2d = false;
  env.flag_verbose = false;
  env.flag_noopt = false;
  for(int i = 1; i < argc; ++i) {
    const char *arg = argv[i];
    switch(*arg) {
      case '\0': continue;
      case '-': {
          bool used_optarg;
          if(!handle_option(env, arg + 1, argv[i + 1], used_optarg))
            return 1;
          if(used_optarg)
            ++i;
          break;
      }
      default:
        read_file(env, arg);
    }
  }

  // allows us to test the arg-parse engine
  if(env.stmts.empty())
    return 0;

  if(!env.flag_noopt)
    optimize(env);

  unordered_map<std::string, size_t> labels;
  size_t stcnt = 0;
  // generate map of labels
  for(auto &i : env.stmts) {
    i.do_ignore = !strncmp(i.cmd, "-L-", 3);
    if(i.do_ignore) {
      labels[i.a_s] = stcnt;
      continue;
    }
    transform(i.cmd, i.cmd + 3, i.cmd, ::toupper);

    switch(i.atyp) {
      case lc1atyp::RELATIVE:
        i.atyp = lc1atyp::ABSOLUTE;
        i.a_i += stcnt;
        break;
      case lc1atyp::LABEL:
        try {
          i.a_i = labels.at(i.a_s);
          string().swap(i.a_s);
          i.atyp = lc1atyp::ABSOLUTE;
        } catch(...) {
          // do nothing
        }
        break;
      default: break;
    }

    ++stcnt;
  }

  // resolve labels
  for(auto &i : env.stmts) {
    if(i.do_ignore || i.atyp != lc1atyp::LABEL) continue;
    try {
      i.a_i = labels.at(i.a_s);
      string().swap(i.a_s);
      i.atyp = lc1atyp::ABSOLUTE;
    } catch(...) {
      cerr << "lc1c: ERROR: undefined label '" << i.a_s << "'\n";
      return 1;
    }
  }

  if(!env.flag_noopt)
    optimize_idconsts(env, labels);

  // resolve idconsts
  const size_t stmtcnt = env.stmts.size();
  vector<int> idc_vals;
  for(auto &i : env.stmts) {
    if(i.do_ignore || i.atyp != lc1atyp::IDCONST) continue;
    const string lblnam = "$" + to_string(i.a_i);
    const auto it = labels.find(lblnam);
    if(it == labels.end()) {
      idc_vals.emplace_back(i.a_i);
      i.a_i = stcnt + idc_vals.size() - 1;
      labels[lblnam] = i.a_i;
    } else {
      i.a_i = it->second;
    }
    i.atyp = lc1atyp::ABSOLUTE;
  }
  env.stmts.reserve(stmtcnt + idc_vals.size());
  {
    lc1stmt stmt;
    strcpy(stmt.cmd, "DEF");
    stmt.atyp = lc1atyp::ABSOLUTE;
    for(const auto &i : idc_vals) {
      stmt.a_i = i;
      env.stmts.emplace_back(stmt);
    }
  }

  // print code
  stcnt = 0;
  if(env.flag_verbose && env.compout == &cout) cerr << "==== compiled code: ====\n";
  for(const auto &i : env.stmts) {
    if(i.do_ignore) continue;
    (*env.compout) << stcnt << ' ' << i.cmd;
    switch(i.atyp) {
      case lc1atyp::ABSOLUTE:
        (*env.compout) << ' ' << i.a_i;
        [[fallthrough]];
      case lc1atyp::NONE:
        break;
      default:
        cerr << "\nlc1c: INTERNAL ERROR: impossible state i.atyp (" << lc1atyp2str(i.atyp) << ") != (ABSOLUTE|NONE) \n";
        return 2;
    }
    if(env.flag_u2d) (*env.compout) << '\r';
    (*env.compout) << '\n';
    ++stcnt;
  }

  if(env.compout != &cout) delete env.compout;

  return 0;
}
