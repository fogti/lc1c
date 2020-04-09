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
