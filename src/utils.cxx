#include "utils.hpp"
#include <ctype.h>
#include <algorithm>
#include <iostream>
#include <locale>
#include <unordered_set>

using namespace std;

void str_trim(string &s) {
  static const string whitespace = "\t\n\v\f\r ";
  size_t tmp = s.find_last_not_of(whitespace);
  if(tmp != string::npos) s.erase(tmp + 1);
  tmp = s.find_first_not_of(whitespace);
  if(tmp != string::npos) s.erase(0, tmp);
}

void str_lower(string &s) noexcept {
  std::transform(s.begin(), s.end(), s.begin(), ::tolower);
}

void file_parse_error(const char *file, size_t lineno, const string &msg) {
  cerr << "lc1c: " << file;
  if(lineno) cerr << ": line " << (lineno - 1);
  cerr << ": " << msg << '\n';
}

bool cmd2has_arg(const string &command) noexcept {
  static const unordered_set<string> lut = {
    "cal", "def", "jmp", "jpo", "jps", "lda", "ldb", "mov", "rla", "rra"
  };
  return lut.find(command) != lut.end();
}

lc1atyp arg2atyp(const string &command) noexcept {
  switch(command.front()) {
    case '@': return lc1atyp::ABSOLUTE;
    case '.': return lc1atyp::RELATIVE;
    case '$': return lc1atyp::IDCONST;
    default:  return isalpha(command.front()) ? lc1atyp::LABEL : lc1atyp::INVALID;
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
