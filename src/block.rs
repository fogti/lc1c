use crate::stmt::{Argument, CmdNoArg, CmdWithArg, Command, CondJumpCond, Statement};
use std::collections::{BTreeSet, HashMap};
use std::{borrow::Cow, convert::TryInto, fmt, mem::take};

#[derive(Clone, Debug)]
pub struct ResolveLabelError(pub CmdWithArg, pub String);

pub fn resolve_labels(
    stmts: &mut Vec<Statement>,
) -> Result<HashMap<String, u32>, ResolveLabelError> {
    // generate map of labels
    let mut labels = HashMap::new();
    let mut stcnt: u32 = 0;

    *stmts = take(stmts)
        .into_iter()
        .filter_map(|mut x| {
            match &mut x.cmd {
                Command::Label(ref mut l) => {
                    labels.insert(take(l), stcnt);
                    return None;
                }
                _ => {}
            }
            stcnt += 1;
            Some(x)
        })
        .collect();

    // resolve labels
    for x in stmts.iter_mut() {
        match &mut x.cmd {
            Command::Wa(cmd, ref mut arg) => {
                if let Argument::Label(ref mut l) = arg {
                    if let Some(y) = labels.get(&*l) {
                        *arg = Argument::Absolute(
                            (*y).try_into().expect("unable to convert label value"),
                        );
                    } else {
                        return Err(ResolveLabelError(*cmd, take(l)));
                    }
                }
            }
            _ => {}
        }
    }

    Ok(labels)
}

#[derive(Clone, Copy, Debug)]
enum JumpTarget<BbId> {
    BasicBlock(BbId),
    Absolute(i32),
}

#[derive(Clone, Debug)]
enum BbRef<'a> {
    Label(Cow<'a, str>),
    Id(usize),
}

type FlexBB = BbRef<'static>;

impl<T> JumpTarget<T> {
    fn map<U>(self, f: impl FnOnce(T) -> U) -> JumpTarget<U> {
        self.try_map(|x| Ok::<U, ()>(f(x))).unwrap()
    }

    fn try_map<U, E>(self, f: impl FnOnce(T) -> Result<U, E>) -> Result<JumpTarget<U>, E> {
        Ok(match self {
            JumpTarget::Absolute(a) => JumpTarget::Absolute(a),
            JumpTarget::BasicBlock(bb) => JumpTarget::BasicBlock(f(bb)?),
        })
    }
}

impl<T: fmt::Display> fmt::Display for JumpTarget<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JumpTarget::Absolute(a) => write!(f, "@{}", a),
            JumpTarget::BasicBlock(b) => write!(f, "bb:{}", b),
        }
    }
}

impl JumpTarget<String> {
    fn new(arg: Argument) -> Result<Self, Argument> {
        Ok(match arg {
            Argument::Label(l) => JumpTarget::BasicBlock(l),
            Argument::Absolute(a) => JumpTarget::Absolute(a),
            arg => return Err(arg),
        })
    }
}

impl From<String> for FlexBB {
    fn from(x: String) -> Self {
        FlexBB::Label(x.into())
    }
}

impl From<usize> for FlexBB {
    fn from(x: usize) -> Self {
        FlexBB::Id(x)
    }
}

#[derive(Clone, Debug)]
pub struct RawBasicBlock<BbId> {
    stmts: Vec<Statement>,
    condjmp: Option<(CondJumpCond, JumpTarget<BbId>)>,
    begin_loc: Option<crate::stmt::Location>,
    endjmp_loc: Option<crate::stmt::Location>,
    // None -> HLT
    next: Option<JumpTarget<BbId>>,
}

impl<B> Default for RawBasicBlock<B> {
    fn default() -> Self {
        RawBasicBlock {
            stmts: Vec::new(),
            condjmp: None,
            begin_loc: None,
            endjmp_loc: None,
            next: None,
        }
    }
}

impl<B> RawBasicBlock<B> {
    pub fn is_empty(&self) -> bool {
        self.stmts.is_empty() && self.condjmp.is_none()
    }
}

pub type BasicBlock = RawBasicBlock<usize>;

fn jumptrg_to_bbid<BbId>(jt: Option<JumpTarget<BbId>>) -> Option<BbId> {
    match jt? {
        JumpTarget::BasicBlock(bbid) => Some(bbid),
        JumpTarget::Absolute(_) => None,
    }
}

impl BasicBlock {
    fn linked_bb_ids(&self) -> impl Iterator<Item = BbRef<'_>> {
        fn jumptrg_to_link(jt: Option<JumpTarget<usize>>) -> Option<BbRef<'static>> {
            jumptrg_to_bbid(jt).map(BbRef::Id)
        }

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
            .chain(jumptrg_to_link(self.condjmp.map(|i| i.1)))
            .chain(jumptrg_to_link(self.next))
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
            writeln!(f, "\t{}", i)?;
        }
        if let Some(x) = self.condjmp.as_ref() {
            writeln!(f, "  condjmp: if {} -> {}", CmdWithArg::from(x.0), x.1)?;
        }
        write!(f, "  --> ")?;
        if let Some(x) = self.next.as_ref() {
            write!(f, "{}", x)
        } else {
            write!(f, "<HALT>")
        }?;
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
                    if last_bb.next.is_none() {
                        last_bb.next =
                            Some(wraptrg(JumpTarget::<usize>::BasicBlock(bbs.len() + 1)));
                    }
                    bbs.push(std::mem::take(&mut last_bb));
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
                        last_bb.begin_loc = Some(loc);
                    }
                    labels.insert(l, bbs.len());
                }
                Command::Na(CmdNoArg::Hlt) => {
                    last_bb.next = None;
                    last_bb.endjmp_loc = Some(loc);
                    bbs.push(std::mem::take(&mut last_bb));
                }
                Command::Wa(CmdWithArg::Jmp, arg) => {
                    last_bb.next = Some(wraptrg(JumpTarget::new(arg).map_err(ModuleParseError)?));
                    last_bb.endjmp_loc = Some(loc);
                    bbs.push(std::mem::take(&mut last_bb));
                }
                Command::Wa(CmdWithArg::Jps, arg) => {
                    last_bb.condjmp = Some((
                        CondJumpCond::Sign,
                        wraptrg(JumpTarget::new(arg).map_err(ModuleParseError)?),
                    ));
                    last_bb.endjmp_loc = Some(loc);
                    pushbb!();
                }
                Command::Wa(CmdWithArg::Jpo, arg) => {
                    last_bb.condjmp = Some((
                        CondJumpCond::Overflow,
                        wraptrg(JumpTarget::new(arg).map_err(ModuleParseError)?),
                    ));
                    last_bb.endjmp_loc = Some(loc);
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
                if let Some(JumpTarget::BasicBlock(BbRef::Id(y))) = x.next {
                    if y >= bbs_len {
                        x.next = None;
                    }
                }
            }
        }

        let bbs = bbs.into_iter().map(|i| {
            let RawBasicBlock { stmts, condjmp, begin_loc, endjmp_loc, next } = i;
            let mangle_jmptrg = |jmptrg: JumpTarget<BbRef<'static>>| -> Result<JumpTarget<usize>, ModuleParseError> {
                jmptrg.try_map(|jt2| {
                    match jt2 {
                        BbRef::Label(label) => {
                            if let Some(real_jmptrg) = labels.get(label.as_ref()) {
                                Ok(*real_jmptrg)
                            } else {
                                Err(ModuleParseError(Argument::Label(label.into_owned())))
                            }
                        },
                        BbRef::Id(id) => Ok(id),
                    }
                })
            };
            let condjmp = if let Some((cond, jmptrg)) = condjmp {
                Some((cond, mangle_jmptrg(jmptrg)?))
            } else {
                None
            };
            let next = if let Some(jmptrg) = next {
                Some(mangle_jmptrg(jmptrg)?)
            } else {
                None
            };
            Ok(BasicBlock { stmts, condjmp, begin_loc, endjmp_loc, next })
        }).collect::<Result<Vec<_>, _>>()?;
        Ok(Module { bbs, labels })
    }

    fn labels_in_use<'a>(bbs: &'a [BasicBlock]) -> impl Iterator<Item = Cow<'a, str>> {
        bbs.iter()
            .flat_map(BasicBlock::linked_bb_ids)
            .filter_map(|i| match i {
                BbRef::Id(_) => None,
                BbRef::Label(l) => Some(l),
            })
    }

    pub fn labels_of_bb(&self, bbid: usize) -> impl Iterator<Item = &str> {
        self.labels.iter().filter_map(move |x| {
            if *x.1 == bbid {
                Some(x.0.as_str())
            } else {
                None
            }
        })
    }

    pub fn remove_bbs(&mut self, bbids: BTreeSet<usize>) -> bool {
        let mut ret = true;
        let mangle_bbid = |id: &mut usize| {
            let curid = *id;
            *id = curid - bbids.iter().take_while(|twi| **twi < curid).count();
        };
        let mut trt_bbid = |jty: Option<&mut JumpTarget<usize>>| {
            if let Some(JumpTarget::BasicBlock(ref mut bbid)) = jty {
                if bbids.contains(bbid) {
                    ret = false;
                } else {
                    mangle_bbid(bbid);
                }
            }
        };
        let new_bbs: Vec<BasicBlock> = self
            .bbs
            .clone()
            .into_iter()
            .enumerate()
            .filter_map(|(n, mut i)| {
                if bbids.contains(&n) {
                    None
                } else {
                    trt_bbid(i.condjmp.as_mut().map(|i| &mut i.1));
                    trt_bbid(i.next.as_mut());
                    Some(i)
                }
            })
            .collect();
        std::mem::drop(trt_bbid);
        if ret {
            let liu: BTreeSet<Cow<str>> = Self::labels_in_use(&new_bbs).collect();
            self.labels.retain(|k, v| {
                if !liu.contains(k.as_str()) || bbids.contains(v) {
                    return false;
                }
                mangle_bbid(v);
                true
            });
            self.bbs = new_bbs;
            true
        } else {
            false
        }
    }

    pub fn unused_bbs(&self) -> BTreeSet<usize> {
        if self.bbs.is_empty() {
            return BTreeSet::new();
        }
        let mut used: BTreeSet<usize> = [0].iter().copied().collect();
        let mut old_cnt = used.len();
        let labels = &self.labels;
        let bbs = &self.bbs;
        loop {
            used = std::mem::take(&mut used)
                .into_iter()
                .flat_map(|i| bbs.get(i).map(|bb| (i, bb)))
                .flat_map(|(i, bb)| {
                    bb.linked_bb_ids()
                        .filter_map(|j| match j {
                            BbRef::Id(id) => Some(id),
                            BbRef::Label(l) => {
                                let x = labels.get(l.as_ref()).copied();
                                if x.is_none() {
                                    eprintln!("Module::unused_bbs: got dangling label: {}", l);
                                }
                                x
                            }
                        })
                        .chain(std::iter::once(i))
                })
                .collect();

            if used.len() == old_cnt {
                break;
            }
            old_cnt = used.len();
        }
        let allbbs: BTreeSet<usize> = (0..self.bbs.len()).into_iter().collect();
        allbbs.difference(&used).cloned().collect()
    }

    pub fn bbs(&self) -> &[BasicBlock] {
        &self.bbs[..]
    }

    pub fn labels(&self) -> &HashMap<String, usize> {
        &self.labels
    }
}
