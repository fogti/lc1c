#include "optimize.hpp"
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
      env.flag_noopt = !strncmp(opt + 1, "0", 2);
      env.flag_deepopt = !strncmp(opt + 1, "D", 2);
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
      strcpy(stmt.cmd, LABEL_CMD);
      stmt.atyp = lc1atyp::LABEL;
      stmt.a_s  = move(tok);
      stmt.a_s.pop_back();
      stmt.a_i  = 0;
      env.stmts.emplace_back(move(stmt));
      ss >> tok;
      if(tok.empty()) continue;
    }

    string cmd = move(tok), errmsgtxt;
    stmt.do_ignore = (cmd.size() == (LC1CMD_LEN + 1) && cmd[LC1CMD_LEN] != '*');
    if(!stmt.do_ignore && cmd.size() > LC1CMD_LEN) {
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

    strncpy(stmt.cmd, cmd.c_str(), LC1CMD_LEN);
    // make sure that '*' is reset
    stmt.cmd[LC1CMD_LEN] = 0;

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
          absolute_clear(stmt.a_s);
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
      absolute_clear(stmt.a_s);
      stmt.a_i = 0;
      env.stmts.emplace_back(move(stmt));
      continue;
    }
    on_cmd_error:
      file_parse_error(file, lineno, errmsgtxt);
  }
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
  env.flag_deepopt = false;
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

  if(!env.flag_noopt) {
    optimize(env);
    if(env.flag_deepopt) {
      optimize_deep(env);
      optimize(env);
    }
  }

  unordered_map<std::string, size_t> labels;
  size_t stcnt = 0;
  auto &stmts = env.stmts;
  // generate map of labels
  for(auto it = stmts.begin(); it != stmts.end();) {
    auto &i = *it;
    if(!strncmp(i.cmd, LABEL_CMD, LC1CMD_LEN)) {
      labels[i.a_s] = stcnt;
      it = stmts.erase(it);
      continue;
    }
    transform(i.cmd, i.cmd + LC1CMD_LEN, i.cmd, ::toupper);

    switch(i.atyp) {
      case lc1atyp::RELATIVE:
        i.atyp = lc1atyp::ABSOLUTE;
        i.a_i += stcnt;
        break;
      case lc1atyp::LABEL:
        try {
          i.a_i  = labels.at(i.a_s);
          // NOTE: labels.at might throw an exception, only update atyp
          // if no exception is thrown
          i.atyp = lc1atyp::ABSOLUTE;
          absolute_clear(i.a_s);
        } catch(...) {
          // do nothing
        }
        break;
      default: break;
    }

    ++stcnt;
    ++it;
  }

  // resolve labels
  for(auto &i : stmts) {
    if(i.atyp != lc1atyp::LABEL) continue;
    try {
      i.a_i = labels.at(i.a_s);
      absolute_clear(i.a_s);
      i.atyp = lc1atyp::ABSOLUTE;
    } catch(...) {
      cerr << "lc1c: ERROR: undefined label '" << i.a_s << "' @ cmd " << i.cmd << "\n";
      return 1;
    }
  }

  if(!env.flag_noopt)
    optimize_idconsts(env, labels);

  // resolve idconsts
  const size_t stmtcnt = stmts.size();
  vector<int> idc_vals;
  for(auto &i : stmts) {
    if(i.atyp != lc1atyp::IDCONST) continue;
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
  absolute_clear(labels);
  stmts.reserve(stmtcnt + idc_vals.size());
  {
    lc1stmt stmt;
    strcpy(stmt.cmd, "DEF");
    stmt.atyp = lc1atyp::ABSOLUTE;
    for(const auto &i : idc_vals) {
      stmt.a_i = i;
      stmts.emplace_back(stmt);
    }
  }

  // print code
  stcnt = 0;
  if(env.flag_verbose && env.compout == &cout) cerr << "==== compiled code: ====\n";
  for(const auto &i : stmts) {
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
