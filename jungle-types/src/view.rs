use crate::{Conditional, Join, NodeMetadata, Select, Step, Transparent, While};
use inception::*;

/// Structural AST for a journey/flow graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JourneyAst {
    Empty,
    Sequence(Vec<JourneyAst>),
    Step {
        label: &'static str,
    },
    Conditional {
        label: &'static str,
        left: Box<JourneyAst>,
        right: Box<JourneyAst>,
    },
    While {
        label: &'static str,
        body: Box<JourneyAst>,
    },
    Transparent {
        label: &'static str,
        metadata: &'static str,
        body: Box<JourneyAst>,
    },
    Select {
        label: &'static str,
        left: Box<JourneyAst>,
        right: Box<JourneyAst>,
    },
    Join {
        label: &'static str,
        left: Box<JourneyAst>,
        right: Box<JourneyAst>,
    },
}

impl JourneyAst {
    pub fn sequence(nodes: Vec<JourneyAst>) -> JourneyAst {
        let mut flat = Vec::new();
        for node in nodes {
            match node {
                JourneyAst::Empty => {}
                JourneyAst::Sequence(children) => {
                    for child in children {
                        if !matches!(child, JourneyAst::Empty) {
                            flat.push(child);
                        }
                    }
                }
                other => flat.push(other),
            }
        }
        match flat.len() {
            0 => JourneyAst::Empty,
            1 => flat.into_iter().next().expect("single sequence item"),
            _ => JourneyAst::Sequence(flat),
        }
    }
}

#[inception(property = JungleJourneyAst, signature(input = Input, output = Output))]
pub trait BuildJourneyAst<Input> {
    type Output;

    fn push_ast(input: Input) -> Self::Output;

    fn nothing(input: Input) -> Input {
        input
    }

    fn merge<H, R>(
        _left: H,
        _right: R,
        input: Input,
    ) -> <R as BuildJourneyAst<<H as BuildJourneyAst<Input>>::Output>>::Output
    where
        H: BuildJourneyAst<Input>,
        R: BuildJourneyAst<<H as BuildJourneyAst<Input>>::Output>,
    {
        let output = <H as BuildJourneyAst<_>>::push_ast(input);
        <R as BuildJourneyAst<_>>::push_ast(output)
    }

    fn merge_variant_field<H, R>(_left: H, _right: R, input: Input) -> Input {
        let _ = (_left, _right);
        let _ = core::marker::PhantomData::<(H, R)>;
        input
    }

    fn join<F>(_fields: F, input: Input) -> <F as BuildJourneyAst<Input>>::Output
    where
        F: BuildJourneyAst<Input>,
    {
        <F as BuildJourneyAst<_>>::push_ast(input)
    }
}

#[inception::primitive(property = JungleJourneyAst)]
impl<T, A> BuildJourneyAst<Vec<JourneyAst>> for Step<T, A>
where
    T: crate::Animal + 'static,
    A: crate::Pulse<T> + 'static,
    <A as crate::Pulse<T>>::Action: 'static,
{
    type Output = Vec<JourneyAst>;

    fn push_ast(mut nodes: Vec<JourneyAst>) -> Self::Output {
        nodes.push(JourneyAst::Step {
            label: core::any::type_name::<<A as crate::Pulse<T>>::Action>(),
        });
        nodes
    }
}

#[inception::primitive(property = JungleJourneyAst)]
impl<P, L, R> BuildJourneyAst<Vec<JourneyAst>> for Conditional<P, L, R>
where
    P: 'static,
    L: BuildJourneyAst<Vec<JourneyAst>, Output = Vec<JourneyAst>>,
    R: BuildJourneyAst<Vec<JourneyAst>, Output = Vec<JourneyAst>>,
{
    type Output = Vec<JourneyAst>;

    fn push_ast(mut nodes: Vec<JourneyAst>) -> Self::Output {
        let left =
            JourneyAst::sequence(<L as BuildJourneyAst<Vec<JourneyAst>>>::push_ast(Vec::new()));
        let right =
            JourneyAst::sequence(<R as BuildJourneyAst<Vec<JourneyAst>>>::push_ast(Vec::new()));
        nodes.push(JourneyAst::Conditional {
            label: core::any::type_name::<P>(),
            left: Box::new(left),
            right: Box::new(right),
        });
        nodes
    }
}

#[inception::primitive(property = JungleJourneyAst)]
impl<C, F> BuildJourneyAst<Vec<JourneyAst>> for While<C, F>
where
    C: 'static,
    F: BuildJourneyAst<Vec<JourneyAst>, Output = Vec<JourneyAst>>,
{
    type Output = Vec<JourneyAst>;

    fn push_ast(mut nodes: Vec<JourneyAst>) -> Self::Output {
        let body =
            JourneyAst::sequence(<F as BuildJourneyAst<Vec<JourneyAst>>>::push_ast(Vec::new()));
        nodes.push(JourneyAst::While {
            label: core::any::type_name::<C>(),
            body: Box::new(body),
        });
        nodes
    }
}

#[inception::primitive(property = JungleJourneyAst)]
impl<M, F> BuildJourneyAst<Vec<JourneyAst>> for Transparent<M, F>
where
    M: NodeMetadata + 'static,
    F: BuildJourneyAst<Vec<JourneyAst>, Output = Vec<JourneyAst>>,
{
    type Output = Vec<JourneyAst>;

    fn push_ast(mut nodes: Vec<JourneyAst>) -> Self::Output {
        let body =
            JourneyAst::sequence(<F as BuildJourneyAst<Vec<JourneyAst>>>::push_ast(Vec::new()));
        nodes.push(JourneyAst::Transparent {
            label: core::any::type_name::<M>(),
            metadata: M::METADATA,
            body: Box::new(body),
        });
        nodes
    }
}

#[inception::primitive(property = JungleJourneyAst)]
impl<L, R> BuildJourneyAst<Vec<JourneyAst>> for Select<L, R>
where
    L: BuildJourneyAst<Vec<JourneyAst>, Output = Vec<JourneyAst>>,
    R: BuildJourneyAst<Vec<JourneyAst>, Output = Vec<JourneyAst>>,
{
    type Output = Vec<JourneyAst>;

    fn push_ast(mut nodes: Vec<JourneyAst>) -> Self::Output {
        let left =
            JourneyAst::sequence(<L as BuildJourneyAst<Vec<JourneyAst>>>::push_ast(Vec::new()));
        let right =
            JourneyAst::sequence(<R as BuildJourneyAst<Vec<JourneyAst>>>::push_ast(Vec::new()));
        nodes.push(JourneyAst::Select {
            label: "Select",
            left: Box::new(left),
            right: Box::new(right),
        });
        nodes
    }
}

#[inception::primitive(property = JungleJourneyAst)]
impl<L, R> BuildJourneyAst<Vec<JourneyAst>> for Join<L, R>
where
    L: BuildJourneyAst<Vec<JourneyAst>, Output = Vec<JourneyAst>>,
    R: BuildJourneyAst<Vec<JourneyAst>, Output = Vec<JourneyAst>>,
{
    type Output = Vec<JourneyAst>;

    fn push_ast(mut nodes: Vec<JourneyAst>) -> Self::Output {
        let left =
            JourneyAst::sequence(<L as BuildJourneyAst<Vec<JourneyAst>>>::push_ast(Vec::new()));
        let right =
            JourneyAst::sequence(<R as BuildJourneyAst<Vec<JourneyAst>>>::push_ast(Vec::new()));
        nodes.push(JourneyAst::Join {
            label: "Join",
            left: Box::new(left),
            right: Box::new(right),
        });
        nodes
    }
}

pub trait JourneyAstSource {
    fn journey_ast() -> JourneyAst;
}

impl<T> JourneyAstSource for T
where
    T: BuildJourneyAst<Vec<JourneyAst>, Output = Vec<JourneyAst>>,
{
    fn journey_ast() -> JourneyAst {
        JourneyAst::sequence(<T as BuildJourneyAst<Vec<JourneyAst>>>::push_ast(Vec::new()))
    }
}
