#pragma once
#include <inttypes.h>
#include <string.h>
#include <string>
#include <ostream>
#include <unordered_map>
#include <vector>
#include <utility>

enum class lc1atyp {
  INVALID, NONE, ABSOLUTE, RELATIVE, IDCONST, LABEL
};

typedef uint8_t lc1cmd;

// virtual commands
#define LC1CMD_NONE  0x00
#define LC1CMD_DEF   0x01
#define LC1CMD_LABEL 0x02

// real commands
#define LC1CMD_LDA 0x10
#define LC1CMD_LDB 0x11
#define LC1CMD_MOV 0x12
#define LC1CMD_MAB 0x13
#define LC1CMD_ADD 0x14
#define LC1CMD_SUB 0x15
#define LC1CMD_AND 0x16
#define LC1CMD_NOT 0x17

#define LC1CMD_JMP 0x18
#define LC1CMD_JPS 0x19
#define LC1CMD_JPO 0x1a
#define LC1CMD_CAL 0x1b
#define LC1CMD_RET 0x1c
#define LC1CMD_RRA 0x1d
#define LC1CMD_RLA 0x1e
#define LC1CMD_HLT 0x1f

struct lc1stmt {
  lc1cmd cmd;

  // argument
  lc1atyp atyp;
  std::string a_s;
  int a_i;

  // flags
  bool do_ignore;

  lc1stmt(): cmd(0), atyp(lc1atyp::INVALID), a_i(0), do_ignore(false)
    { }
  lc1stmt(const lc1stmt &o) = default;
  lc1stmt(lc1stmt &&o) noexcept
    : cmd(o.cmd), atyp(o.atyp), a_s(move(o.a_s)), a_i(o.a_i), do_ignore(o.do_ignore)
    { }

  lc1stmt& operator=(const lc1stmt &o) = default;
  lc1stmt& operator=(lc1stmt &&o) noexcept {
    cmd  = o.cmd;
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
