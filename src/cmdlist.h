#ifndef CMDJTE
# error "CMDJTE undefined at inclusion of cmdlist.h"
#else
// virtual commands
CMDJTE(DEF)
// the following list is sorted based of cmd id
CMDJTE(LDA) CMDJTE(LDB) CMDJTE(MOV) CMDJTE(MAB)
CMDJTE(ADD) CMDJTE(SUB) CMDJTE(AND) CMDJTE(NOT)
CMDJTE(JMP) CMDJTE(JPS) CMDJTE(JPO) CMDJTE(CAL)
CMDJTE(RET) CMDJTE(RRA) CMDJTE(RLA) CMDJTE(HLT)
#endif
