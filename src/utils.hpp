#pragma once
#include "types.hpp"
#include <stddef.h>

void str_trim(std::string &s);

void file_parse_error(const char *file, size_t lineno, const std::string &msg);
lc1atyp arg2atyp(const char *arg, bool &defmode) noexcept;
lc1cmd  str2cmd(std::string command) noexcept;
bool cmd2has_arg(const lc1cmd cmd) noexcept;

auto lc1cmd2str(const lc1cmd cmd) noexcept -> const char *;
auto lc1atyp2str(const lc1atyp lat) noexcept -> const char*;

template<class Cont>
void absolute_clear(Cont &cont) noexcept {
  Cont().swap(cont);
}
