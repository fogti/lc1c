#include "utils.hpp"
#include <ctype.h>
#include <algorithm>
#include <iostream>
#include <locale>
#include <unordered_set>

using namespace std;

template <typename I>
static string n2hexstr(I w, size_t hex_len = sizeof(I)<<1) {
  static const char* digits = "0123456789ABCDEF";
  string rc(hex_len,'0');
  for(size_t i=0, j=(hex_len-1)*4 ; i<hex_len; ++i,j-=4)
    rc[i] = digits[(w>>j) & 0x0f];
  return rc;
}

void str_trim(string &s) {
  static const string whitespace = "\t\n\v\f\r ";
  const size_t tmp = s.find_last_not_of(whitespace);
  if(tmp != string::npos) s.erase(tmp + 1);
  s.erase(0, s.find_first_not_of(whitespace));
}

void file_parse_error(const char *file, size_t lineno, const string &msg) {
  cerr << "lc1c: " << file;
  if(lineno) cerr << ": line " << (lineno - 1);
  cerr << ": " << msg << '\n';
}

bool cmd2has_arg(const lc1cmd cmd) noexcept {
  static const unordered_set<lc1cmd> lut = {
#define JTE(X) LC1CMD_##X,
    JTE(CAL) JTE(DEF) JTE(JMP) JTE(JPO) JTE(JPS)
    JTE(LDA) JTE(LDB) JTE(MOV) JTE(RLA) JTE(RRA)
#undef JTE
  };
  return lut.find(cmd) != lut.end();
}

lc1atyp arg2atyp(const char *arg, bool &defmode) noexcept {
  const bool defmode_cached = defmode;
  defmode = false;
  switch(*arg) {
    case '@': return lc1atyp::ABSOLUTE;
    case '.': return lc1atyp::RELATIVE;
    case '$': return lc1atyp::IDCONST;
    case '0': case '1': case '2': case '3': case '4':
    case '5': case '6': case '7': case '8': case '9':
    case '-':
      if(defmode_cached) {
        defmode = true;
        return lc1atyp::ABSOLUTE;
      }
      [[fallthrough]];
    default:  return isalpha(*arg) ? lc1atyp::LABEL : lc1atyp::INVALID;
  }
}

auto lc1atyp2str(const lc1atyp atyp) noexcept -> const char* {
  switch(atyp) {
    case lc1atyp::INVALID:  return "invalid";
    case lc1atyp::NONE:     return "none";
    case lc1atyp::ABSOLUTE: return "absolute";
    case lc1atyp::RELATIVE: return "relative";
    case lc1atyp::IDCONST:  return "ind.const";
    case lc1atyp::LABEL:    return "label";
  }
  return "unknown";
}

lc1cmd str2cmd(string cmd) noexcept {
  static const unordered_map<string, lc1cmd> lut = {
# define CMDJTE(X) { #X, LC1CMD_##X },
# include "cmdlist.h"
# undef CMDJTE
  };
  std::transform(cmd.begin(), cmd.end(), cmd.begin(), ::toupper);
  const auto it = lut.find(cmd);
  return (it == lut.end()) ? LC1CMD_NONE : it->second;
}

auto lc1cmd2str(const lc1cmd cmd) noexcept -> const char * {
  switch(cmd) {
# define CMDJTE(X) case LC1CMD_##X: return #X;
# include "cmdlist.h"
    CMDJTE(LABEL)
# undef JTE
    default: return "-UKN-";
  }
}

auto lc1stmt::to_string() const -> string {
  string ret = lc1cmd2str(cmd);
  char prefix = 0;
  switch(atyp) {
    case lc1atyp::INVALID:
      ret += " (invalid)";
      [[fallthrough]];

    case lc1atyp::NONE    : return ret;
    case lc1atyp::ABSOLUTE: break;
    case lc1atyp::RELATIVE: prefix = '.'; break;
    case lc1atyp::IDCONST : prefix = '$'; break;

    case lc1atyp::LABEL:
      (ret += ' ') += a_s;
      return ret;

    default:
      return ret + " (unexpected arg_type)";
  }

  ret += ' ';
  if(prefix) ret += prefix;
  ret += std::to_string(a_i);
  return ret;
}
