#pragma once
#include "types.hpp"
#include <stddef.h>

void str_trim(std::string &s);
void str_lower(std::string &s) noexcept;

void file_parse_error(const char *file, size_t lineno, const std::string &msg);
bool cmd2has_arg(const std::string &command) noexcept;
lc1atyp arg2atyp(const std::string &command, bool &defmode) noexcept;

auto lc1atyp2str(const lc1atyp lat) noexcept -> const char*;
