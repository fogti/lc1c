mod utils;

use crate::bb::utils::{jumptrg_to_bbid, BbRef, FlexBB, JumpTarget, UcJumpTarget};
use crate::stmt::{Argument, CmdNoArg, CmdWithArg, Command, CondJumpCond, Statement};
use std::collections::{BTreeSet, HashMap};
use std::{borrow::Cow, fmt, mem::take};

#[derive(Clone, Debug)]
pub struct RawBasicBlock<BbId> {
    stmts: Vec<Statement>,
    condjmp: Option<(CondJumpCond, JumpTarget<BbId>)>,
    begin_loc: Option<crate::stmt::Location>,
    endjmp_loc: Option<crate::stmt::Location>,
    next: UcJumpTarget<BbId>,
}

impl<B> Default for RawBasicBlock<B> {
    fn default() -> Self {
        RawBasicBlock {
            stmts: Vec::new(),
            condjmp: None,
            begin_loc: None,
            endjmp_loc: None,
            next: UcJumpTarget::Halt,
        }
    }
}

impl<B> RawBasicBlock<B> {
    pub fn is_empty(&self) -> bool {
        self.stmts.is_empty() && self.condjmp.is_none()
    }
}

pub type BasicBlock = RawBasicBlock<usize>;

impl BasicBlock {
    fn linked_bb_ids(&self) -> impl Iterator<Item = BbRef<'_>> {
        self.stmts
            .iter()
            .filter_map(|i| match &i.cmd {
                Command::Def(ref a) | Command::Wa(_, ref a) => {
                    if let Argument::Label(ref l) = a {
                        Some(BbRef::Label(l.into()))
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .chain(jumptrg_to_bbid(self.condjmp.map(|i| i.1)).map(BbRef::Id))
            .chain(self.next.try_into_bbid().ok().map(BbRef::Id))
    }

    fn optimize_once(&mut self) {
        let mut trm = HashMap::new();
        let mut it = self.stmts.windows(2).enumerate();

        while let Some((n, [i, j])) = it.next() {
            use {CmdNoArg as Cna, CmdWithArg as Cwa};
            if j.do_ignore {
                it.next();
            }
            if i.do_ignore || j.do_ignore {
                continue;
            }
            let (kill_fi, kill_se) = match (&i.cmd, &j.cmd) {
                (Command::Na(a), Command::Na(b)) => match (a, b) {
                    (Cna::Add, Cna::Sub) | (Cna::Sub, Cna::Add) | (Cna::Not, Cna::Not) => {
                        (true, true)
                    }
                    (Cna::Mab, Cna::Mab) | (Cna::And, Cna::And) => (true, false),
                    _ => (false, false),
                },
                (Command::Wa(a, aarg), Command::Wa(b, barg)) => match (a, b) {
                    (Cwa::Lda, Cwa::Lda) | (Cwa::Ldb, Cwa::Ldb) => (true, false),
                    (Cwa::Rra, Cwa::Rla) | (Cwa::Rla, Cwa::Rra) => {
                        if aarg == barg {
                            (true, true)
                        } else {
                            (false, false)
                        }
                    }
                    _ => (false, false),
                },
                (Command::Na(a), Command::Wa(b, _)) => match (a, b) {
                    (a, Cwa::Lda) => match a {
                        Cna::Add | Cna::Sub | Cna::And | Cna::Not => (true, false),
                        _ => (false, false),
                    },
                    (a, Cwa::Ldb) => match a {
                        Cna::Mab => (true, false),
                        _ => (false, false),
                    },
                    _ => (false, false),
                },
                (Command::Wa(a, _), Command::Na(b)) => match (a, b) {
                    (Cwa::Ldb, Cna::Mab) => (true, false),
                    _ => (false, false),
                },
                _ => (false, false),
            };
            if kill_fi {
                trm.insert(n, None);
            }
            if kill_se {
                trm.insert(n + 1, None);
                it.next();
            }
        }

        self.stmts = take(&mut self.stmts)
            .into_iter()
            .enumerate()
            .filter_map(|(n, i)| trm.remove(&n).unwrap_or(Some(i)))
            .collect();
    }

    fn optimize_end(&mut self, labels: &HashMap<String, usize>) {
        // perform tail-call optimization
        if self.next != UcJumpTarget::Return {
            return;
        }
        let new_next = if let Some(last) = self.stmts.last() {
            if last.do_ignore {
                return;
            }
            if let Command::Wa(CmdWithArg::Cal, cal_trg) = &last.cmd {
                match cal_trg {
                    Argument::Label(ref l) => labels.get(l).copied().map(UcJumpTarget::BasicBlock),
                    Argument::Absolute(a) => Some(UcJumpTarget::Absolute(*a)),
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        };
        if let Some(next) = new_next {
            self.stmts.pop();
            self.next = next;
        }
    }
}

impl<B: fmt::Display> fmt::Display for RawBasicBlock<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        if let Some(l) = self.begin_loc.as_ref() {
            write!(f, " /* {} */", l)?;
        }
        writeln!(f)?;
        for i in self.stmts.iter() {
            writeln!(f, "{:#}", i)?;
        }
        if let Some(x) = self.condjmp.as_ref() {
            writeln!(f, "  condjmp: if {} -> {}", CmdWithArg::from(x.0), x.1)?;
        }
        write!(f, "  --> {}", self.next)?;
        if let Some(l) = self.endjmp_loc.as_ref() {
            write!(f, " /* {} */", l)?;
        }
        writeln!(f)?;
        writeln!(f, "}}")?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Module {
    bbs: Vec<BasicBlock>,
    labels: HashMap<String, usize>,
}

#[derive(Clone, Debug)]
pub struct ModuleParseError(pub Argument);

impl Module {
    pub fn new(init_stmts: Vec<Statement>) -> Result<Module, ModuleParseError> {
        #[inline(always)]
        fn wraptrg<T: Into<FlexBB>>(x: JumpTarget<T>) -> JumpTarget<FlexBB> {
            x.map(|bb| bb.into())
        }
        let mut bbs = Vec::<RawBasicBlock<FlexBB>>::new();
        let mut labels = HashMap::new();
        let mut last_bb = RawBasicBlock::<FlexBB>::default();

        macro_rules! pushbb {
            () => {
                if !last_bb.is_empty() {
                    if last_bb.next == UcJumpTarget::Halt {
                        last_bb.next =
                            wraptrg(JumpTarget::<usize>::BasicBlock(bbs.len() + 1)).into();
                    }
                    bbs.push(take(&mut last_bb));
                }
            };
        }

        for i in init_stmts {
            let Statement {
                cmd: i_cmd,
                loc,
                do_ignore,
            } = i;
            match i_cmd {
                Command::Label(l) => {
                    pushbb!();
                    if last_bb.begin_loc.is_none() {
                        last_bb.begin_loc = loc;
                    }
                    labels.insert(l, bbs.len());
                }
                Command::Na(CmdNoArg::Ret) => {
                    last_bb.next = UcJumpTarget::Return;
                    last_bb.endjmp_loc = loc;
                    bbs.push(take(&mut last_bb));
                }
                Command::Na(CmdNoArg::Hlt) => {
                    last_bb.next = UcJumpTarget::Halt;
                    last_bb.endjmp_loc = loc;
                    bbs.push(take(&mut last_bb));
                }
                Command::Wa(CmdWithArg::Jmp, arg) => {
                    last_bb.next = wraptrg(JumpTarget::new(arg).map_err(ModuleParseError)?).into();
                    last_bb.endjmp_loc = loc;
                    bbs.push(take(&mut last_bb));
                }
                Command::Wa(CmdWithArg::Jps, arg) => {
                    last_bb.condjmp = Some((
                        CondJumpCond::Sign,
                        wraptrg(JumpTarget::new(arg).map_err(ModuleParseError)?),
                    ));
                    last_bb.endjmp_loc = loc;
                    pushbb!();
                }
                Command::Wa(CmdWithArg::Jpo, arg) => {
                    last_bb.condjmp = Some((
                        CondJumpCond::Overflow,
                        wraptrg(JumpTarget::new(arg).map_err(ModuleParseError)?),
                    ));
                    last_bb.endjmp_loc = loc;
                    pushbb!();
                }
                cmd => {
                    last_bb.stmts.push(Statement {
                        cmd,
                        loc,
                        do_ignore,
                    });
                }
            }
        }

        if !last_bb.is_empty() || labels.values().any(|id| *id == bbs.len()) {
            bbs.push(last_bb);
        }

        // wrap-up
        {
            let bbs_len = bbs.len();
            if let Some(x) = bbs.last_mut() {
                if let UcJumpTarget::BasicBlock(BbRef::Id(y)) = x.next {
                    if y >= bbs_len {
                        x.next = UcJumpTarget::Halt;
                    }
                }
            }
        }

        let bbs = bbs
            .into_iter()
            .map(|i| {
                let RawBasicBlock {
                    stmts,
                    condjmp,
                    begin_loc,
                    endjmp_loc,
                    next,
                } = i;
                let mangle_jtbb = |jt2: BbRef<'static>| -> Result<usize, ModuleParseError> {
                    match jt2 {
                        BbRef::Label(label) => {
                            if let Some(real_jmptrg) = labels.get(label.as_ref()) {
                                Ok(*real_jmptrg)
                            } else {
                                Err(ModuleParseError(Argument::Label(label.into_owned())))
                            }
                        }
                        BbRef::Id(id) => Ok(id),
                    }
                };
                let condjmp = if let Some((cond, jmptrg)) = condjmp {
                    Some((cond, jmptrg.try_map(mangle_jtbb)?))
                } else {
                    None
                };
                let next = next.try_map(mangle_jtbb)?;
                Ok(BasicBlock {
                    stmts,
                    condjmp,
                    begin_loc,
                    endjmp_loc,
                    next,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Module { bbs, labels })
    }

    pub fn finish(self) -> Vec<Statement> {
        let Module { bbs, labels } = self;
        let mut ret = Vec::new();

        for (bbid, mut bb) in bbs.into_iter().enumerate() {
            let mut cur_labels = Self::labels_of_bb_(&labels, bbid);
            ret.push(Statement {
                cmd: Command::Label(
                    cur_labels
                        .next()
                        .map(|i| i.to_string())
                        .unwrap_or_else(|| format!("_L{}", bbid)),
                ),
                loc: bb.begin_loc.clone(),
                do_ignore: false,
            });
            for i in cur_labels {
                ret.push(Statement {
                    cmd: Command::Label(i.to_string()),
                    loc: None,
                    do_ignore: false,
                });
            }
            ret.append(&mut bb.stmts);
            if let Some((cjc, cjt)) = bb.condjmp {
                ret.push(Statement {
                    cmd: Command::Wa(
                        cjc.into(),
                        match cjt {
                            JumpTarget::BasicBlock(cjt_bbid) => Argument::Label(
                                Self::labels_of_bb_(&labels, cjt_bbid)
                                    .next()
                                    .map(|i| i.to_string())
                                    .unwrap_or_else(|| format!("_L{}", cjt_bbid)),
                            ),
                            JumpTarget::Absolute(a) => Argument::Absolute(a),
                        },
                    ),
                    loc: bb.endjmp_loc.clone(),
                    do_ignore: false,
                })
            }
            if bb.next == UcJumpTarget::BasicBlock(bbid + 1) {
                continue;
            }
            ret.push(Statement {
                cmd: match bb.next {
                    UcJumpTarget::Return => Command::Na(CmdNoArg::Ret),
                    UcJumpTarget::Halt => Command::Na(CmdNoArg::Hlt),
                    UcJumpTarget::Absolute(a) => {
                        Command::Wa(CmdWithArg::Jmp, Argument::Absolute(a))
                    }
                    UcJumpTarget::BasicBlock(jt_bbid) => Command::Wa(
                        CmdWithArg::Jmp,
                        Argument::Label(
                            Self::labels_of_bb_(&labels, jt_bbid)
                                .next()
                                .map(|i| i.to_string())
                                .unwrap_or_else(|| format!("_L{}", jt_bbid)),
                        ),
                    ),
                },
                loc: bb.endjmp_loc.clone(),
                do_ignore: false,
            })
        }
        ret
    }

    pub fn bbs(&self) -> &[BasicBlock] {
        &self.bbs[..]
    }

    pub fn labels(&self) -> &HashMap<String, usize> {
        &self.labels
    }

    fn labels_in_use(bbs: &[BasicBlock]) -> impl Iterator<Item = Cow<'_, str>> {
        bbs.iter()
            .flat_map(BasicBlock::linked_bb_ids)
            .filter_map(|i| match i {
                BbRef::Id(_) => None,
                BbRef::Label(l) => Some(l),
            })
    }

    fn labels_of_bb_<'a>(
        labels: &'a HashMap<String, usize>,
        bbid: usize,
    ) -> impl Iterator<Item = &'a str> {
        labels.iter().filter_map(move |x| {
            if *x.1 == bbid {
                Some(x.0.as_str())
            } else {
                None
            }
        })
    }

    pub fn labels_of_bb(&self, bbid: usize) -> impl Iterator<Item = &str> {
        Self::labels_of_bb_(&self.labels, bbid)
    }

    pub fn mangle_bbs(&mut self, trm: HashMap<usize, Option<usize>>) {
        let rem_bbids: BTreeSet<_> = trm.iter().map(|(&k, _)| k).collect();
        let mangle_bbid = |id: &mut usize| {
            let curid = if let Some(Some(x)) = trm.get(id).copied() {
                x
            } else {
                *id
            };
            *id = curid - rem_bbids.iter().take_while(|twi| **twi < curid).count();
        };
        let new_bbs: Vec<BasicBlock> = self
            .bbs
            .clone()
            .into_iter()
            .enumerate()
            .filter(|&(n, _)| !rem_bbids.contains(&n))
            .map(|(_, i)| i)
            .map(|mut i| {
                if let Some((_, JumpTarget::BasicBlock(ref mut bbid))) = i.condjmp.as_mut() {
                    mangle_bbid(bbid);
                }
                if let UcJumpTarget::BasicBlock(ref mut bbid) = i.next {
                    mangle_bbid(bbid);
                }
                i
            })
            .collect();
        let liu: BTreeSet<Cow<str>> = Self::labels_in_use(&new_bbs).collect();
        let mut l4bseen = BTreeSet::new();
        self.labels.retain(|k, v| {
            if rem_bbids.contains(v) {
                return false;
            }
            if !liu.contains(k.as_str()) && !l4bseen.insert(*v) {
                return false;
            }
            mangle_bbid(v);
            true
        });
        self.bbs = new_bbs;
    }

    pub fn unused_bbs(&self) -> BTreeSet<usize> {
        if self.bbs.is_empty() {
            return BTreeSet::new();
        }
        let mut used: BTreeSet<usize> = [0].iter().copied().collect();
        let mut old_cnt = None;
        let labels = &self.labels;
        let bbs = &self.bbs;
        while old_cnt != Some(used.len()) {
            old_cnt = Some(used.len());
            used = take(&mut used)
                .into_iter()
                .flat_map(|i| bbs.get(i).map(|bb| (i, bb)))
                .flat_map(|(i, bb)| {
                    bb.linked_bb_ids()
                        .map(|j| match j {
                            BbRef::Id(id) => id,
                            BbRef::Label(l) => {
                                labels.get(l.as_ref()).copied().expect("got dangling label")
                            }
                        })
                        .chain(std::iter::once(i))
                })
                .collect();
        }
        let allbbs: BTreeSet<usize> = (0..self.bbs.len()).collect();
        allbbs.difference(&used).copied().collect()
    }

    pub fn optimize_once(&mut self) {
        for i in self.bbs.iter_mut() {
            i.optimize_once();
            i.optimize_end(&self.labels);
        }

        let trm = self
            .bbs
            .iter()
            .enumerate()
            // merge unlabeled BBs if possible (no conditional jumps)
            .filter_map(|(n, i)| {
                if i.is_empty() {
                    Some((
                        n,
                        match &i.next {
                            UcJumpTarget::Halt => None,
                            UcJumpTarget::BasicBlock(ref b) => Some(*b),
                            _ => return None,
                        },
                    ))
                } else {
                    None
                }
            })
            // mark unused BBs
            .chain(self.unused_bbs().into_iter().map(|i| (i, None)))
            .collect();

        self.mangle_bbs(trm);
    }

    pub fn optimize(&mut self) {
        let mut old_cnt = None;
        while old_cnt != Some(self.bbs.len()) {
            old_cnt = Some(self.bbs.len());
            self.optimize_once();
        }
    }
}
