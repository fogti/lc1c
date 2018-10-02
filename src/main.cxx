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

    string cmd = move(tok);
    if(cmd.size() > 3) {
      file_parse_error(file, lineno, "got invalid command " + cmd);
      continue;
    }

    str_lower(cmd);
    getline(ss, tok);
    str_trim(tok);
    if(tok.empty() == cmd2has_arg(cmd)) {
      file_parse_error(file, lineno, "invalid invocation of command " + cmd);
      continue;
    }

    strncpy(stmt.cmd, cmd.c_str(), 4);

    if(!tok.empty()) {
      // parse arg addr type
      stmt.atyp = arg2atyp(tok);
      switch(stmt.atyp) {
        case lc1atyp::INVALID:
          goto on_invalid_arg;
        case lc1atyp::LABEL:
          stmt.a_s = move(tok);
          stmt.a_i = 0;
          break;
        default:
          if(tok.size() == 1)
            goto on_invalid_arg;
          stmt.a_s.clear();
          try {
            stmt.a_i = stoi(tok.substr(1));
          } catch(...) {
            goto on_invalid_arg;
          }
          if(stmt.a_i < 0 && stmt.atyp != lc1atyp::RELATIVE)
            goto on_invalid_arg;
      }
      env.stmts.emplace_back(move(stmt));
      continue;
      on_invalid_arg:
        file_parse_error(file, lineno, "invalid argument '" + tok + "'");
    } else {
      stmt.atyp = lc1atyp::NONE;
      stmt.a_s.clear();
      stmt.a_i = 0;
      env.stmts.emplace_back(move(stmt));
    }
  }
}

static void optimize(lc1cenv &env) {
  typedef vector<lc1stmt>::iterator it_t;
  struct optdat_t {
    it_t it;
    bool erase_prev, erase_cur;
  };
  static const unordered_map<string, function<void (optdat_t &o)>> jt = {
    { "addsub", [](optdat_t &o) {
      o.erase_prev = true;
      o.erase_cur = true;
    }},
    { "subadd", [](optdat_t &o) {
      o.erase_prev = true;
      o.erase_cur = true;
    }},
  };
  auto &stmts = env.stmts;
  if(stmts.size() < 2) return;
  optdat_t od;
  auto &oit = od.it;
  oit = stmts.begin() + 1;
  while(oit != stmts.end()) {
    decltype(jt)::const_iterator fnit;
    {
      string fname = (oit - 1)->cmd;
      fname += oit->cmd;
      fnit = jt.find(fname);
      if(fnit == jt.end()) goto do_cont;
      cerr << "optimize " << fname << " @ " << (oit - stmts.begin()) << '\n';
    }
    od.erase_prev = false;
    od.erase_cur = false;
    fnit->second(od);
    if(od.erase_prev || od.erase_cur) {
      oit = stmts.erase(oit - od.erase_prev, oit + od.erase_cur);
      continue;
    }

    do_cont:
      ++oit;
  }
}

int main(int argc, char *argv[]) {
  if(argc == 1) {
    cerr << "lc1c SOURCE_FILE\n"
            "\nreturn codes:\n"
            "  0  success\n"
            "  1  invalid input data or arguments\n"
            "  2  internal error\n";
    return 1;
  }

  lc1cenv env;
  for(int i = 1; i < argc; ++i)
    read_file(env, argv[i]);

  optimize(env);

  unordered_map<std::string, size_t> labels;
  const size_t stmtcnt = env.stmts.size();
  vector<int> idc_vals;
  size_t stcnt = 0;
  // generate map of labels
  for(auto &i : env.stmts) {
    if(!strncmp(i.cmd, "-L-", 3)) {
      i.do_ignore = true;
      labels[i.a_s] = stcnt;
      continue;
    }
    transform(i.cmd, i.cmd + 3, i.cmd, ::toupper);

    if(i.atyp == lc1atyp::RELATIVE) {
      i.atyp = lc1atyp::ABSOLUTE;
      i.a_i += stcnt;
    }

    ++stcnt;
  }

  // resolve labels
  for(auto &i : env.stmts) {
    if(i.do_ignore) continue;
    switch(i.atyp) {
      case lc1atyp::LABEL:
        try {
          i.a_i = labels.at(i.a_s);
        } catch(...) {
          cerr << "lc1c: ERROR: undefined label '" << i.a_s << "'\n";
          return 1;
        }
        string().swap(i.a_s);
        break;
      case lc1atyp::IDCONST: {
        const string lblnam = "$" + to_string(i.a_i);
        const auto it = labels.find(lblnam);
        if(it == labels.end()) {
          idc_vals.emplace_back(i.a_i);
          i.a_i = stcnt + idc_vals.size() - 1;
          labels[lblnam] = i.a_i;
        } else {
          i.a_i = it->second;
        }
        break;
      }
      default:
        continue;
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
  cerr << "==== compiled code: ====\n";
  for(const auto &i : env.stmts) {
    if(i.do_ignore) continue;
    cout << stcnt << ' ' << i.cmd;
    switch(i.atyp) {
      case lc1atyp::ABSOLUTE:
        cout << ' ' << i.a_i;
        [[fallthrough]];
      case lc1atyp::NONE:
        break;
      default:
        cerr << "\nlc1c: INTERNAL ERROR: impossible state i.atyp (" << lc1atyp2str(i.atyp) << ") != (ABSOLUTE|NONE) \n";
        return 2;
    }
    cout << '\n';
    ++stcnt;
  }

  return 0;
}
