use crate::stmt::Argument;
use std::{borrow::Cow, fmt};

#[derive(Clone, Copy, Debug)]
pub enum JumpTarget<BbId> {
    BasicBlock(BbId),
    Absolute(i32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BbRef<'a> {
    Label(Cow<'a, str>),
    Id(usize),
}

pub type FlexBB = BbRef<'static>;

impl<T> JumpTarget<T> {
    #[inline]
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> JumpTarget<U> {
        self.try_map(|x| Ok::<U, ()>(f(x))).unwrap()
    }

    #[inline]
    pub fn try_map<U, E>(self, f: impl FnOnce(T) -> Result<U, E>) -> Result<JumpTarget<U>, E> {
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
    pub fn new(arg: Argument) -> Result<Self, Argument> {
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

#[inline]
pub fn jumptrg_to_bbid<BbId>(jt: Option<JumpTarget<BbId>>) -> Option<BbId> {
    match jt? {
        JumpTarget::BasicBlock(bbid) => Some(bbid),
        JumpTarget::Absolute(_) => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UcJumpTarget<BbId> {
    BasicBlock(BbId),
    Absolute(i32),
    Return,
    Halt,
}

impl<B> From<JumpTarget<B>> for UcJumpTarget<B> {
    #[inline]
    fn from(x: JumpTarget<B>) -> UcJumpTarget<B> {
        match x {
            JumpTarget::BasicBlock(b) => UcJumpTarget::BasicBlock(b),
            JumpTarget::Absolute(a) => UcJumpTarget::Absolute(a),
        }
    }
}

impl<T> UcJumpTarget<T> {
    #[inline]
    pub fn try_map<U, E>(self, f: impl FnOnce(T) -> Result<U, E>) -> Result<UcJumpTarget<U>, E> {
        type Ujt<X> = UcJumpTarget<X>;
        Ok(match self {
            Ujt::Absolute(a) => Ujt::Absolute(a),
            Ujt::BasicBlock(bb) => Ujt::BasicBlock(f(bb)?),
            Ujt::Return => Ujt::Return,
            Ujt::Halt => Ujt::Halt,
        })
    }

    #[inline]
    pub fn try_into_bbid(self) -> Result<T, Self> {
        match self {
            UcJumpTarget::BasicBlock(b) => Ok(b),
            x => Err(x),
        }
    }
}

impl<T: fmt::Display> fmt::Display for UcJumpTarget<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UcJumpTarget::Absolute(a) => write!(f, "@{}", a),
            UcJumpTarget::BasicBlock(b) => write!(f, "bb:{}", b),
            UcJumpTarget::Return => write!(f, "<RETURN>"),
            UcJumpTarget::Halt => write!(f, "<HALT>"),
        }
    }
}
