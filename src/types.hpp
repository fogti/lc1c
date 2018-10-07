#pragma once
#include <string.h>
#include <string>
#include <ostream>
#include <unordered_map>
#include <vector>
#include <utility>

enum class lc1atyp {
  INVALID, NONE, ABSOLUTE, RELATIVE, IDCONST, LABEL
};

#define LABEL_CMD "-L-"
// strlen(cmd) = LC1CMD_LEN
#define LC1CMD_LEN 3

struct lc1stmt {
  char cmd[4];

  // argument
  lc1atyp atyp;
  std::string a_s;
  int a_i;

  // flags
  bool do_ignore;

  lc1stmt(): atyp(lc1atyp::INVALID), a_i(0), do_ignore(false)
    { memset(cmd, 0, 4); }
  lc1stmt(const lc1stmt &o) = default;
  lc1stmt(lc1stmt &&o) noexcept
    : atyp(o.atyp), a_s(move(o.a_s)), a_i(o.a_i), do_ignore(o.do_ignore)
    { memcpy(cmd, o.cmd, 4); }

  lc1stmt& operator=(const lc1stmt &o) = default;
  lc1stmt& operator=(lc1stmt &&o) noexcept {
    memcpy(cmd, o.cmd, 4);
    atyp = o.atyp;
    a_s  = move(o.a_s);
    a_i  = o.a_i;
    do_ignore = o.do_ignore;
    return *this;
  }

  auto to_string() const -> std::string;
};

struct lc1cenv {
  std::vector<lc1stmt> stmts;
  std::ostream *compout;
  bool flag_u2d, flag_verbose, flag_noopt, flag_deepopt;
};

typedef std::unordered_map<std::string, size_t> labels_t;
