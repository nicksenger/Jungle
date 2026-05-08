#[cfg(test)]
extern crate jungle_sdk as inception;
#[cfg(test)]
extern crate jungle_sdk as jungle_types;

#[cfg(test)]
mod tests {
    macro_rules! action {
        (
            $name:ident,
            $id:ty,
            dependency = $dependency_ty:ty
        ) => {
            struct $name;
            impl jungle_sdk::types::ActionMember for $name {}

            impl jungle_sdk::types::Action for $name {
                type Id = jungle_sdk::types::Id<$id>;
                type Dependency = $dependency_ty;
                type In = ();
                type Out = ();
                type Err = ();

                fn act(
                    _dependency: &Self::Dependency,
                    _input: Self::In,
                ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
                    std::future::ready(Ok(()))
                }
            }

            #[jungle_sdk::inception::primitive(property = jungle_sdk::types::JungleActions)]
            impl jungle_sdk::types::Actions for $name {
                type List = jungle_sdk::typosaurus::collections::sp::Node<$id, $name>;
            }

            #[jungle_sdk::inception::primitive(property = jungle_sdk::types::Ident)]
            impl jungle_sdk::types::Identified for $name {
                type Id = $id;
            }
        };

        (
            $name:ident,
            $id:ty,
            in = $in:ty,
            out = $out:ty,
            err = $err:ty,
            act = |$dependency:ident, $input:ident| $body:expr
        ) => {
            struct $name;
            impl jungle_sdk::types::ActionMember for $name {}

            impl jungle_sdk::types::Action for $name {
                type Id = jungle_sdk::types::Id<$id>;
                type Dependency = ();
                type In = $in;
                type Out = $out;
                type Err = $err;

                fn act(
                    $dependency: &Self::Dependency,
                    $input: Self::In,
                ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
                    $body
                }
            }
        };

        ($name:ident, $id:ty) => {
            struct $name;
            impl jungle_sdk::types::ActionMember for $name {}

            impl jungle_sdk::types::Action for $name {
                type Id = jungle_sdk::types::Id<$id>;
                type Dependency = ();
                type In = ();
                type Out = ();
                type Err = ();

                fn act(
                    _dependency: &Self::Dependency,
                    _input: Self::In,
                ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
                    std::future::ready(Ok(()))
                }
            }

            #[jungle_sdk::inception::primitive(property = jungle_sdk::types::JungleActions)]
            impl jungle_sdk::types::Actions for $name {
                type List = jungle_sdk::typosaurus::collections::sp::Node<$id, $name>;
            }

            #[jungle_sdk::inception::primitive(property = jungle_sdk::types::Ident)]
            impl jungle_sdk::types::Identified for $name {
                type Id = $id;
            }
        };
    }

    macro_rules! animal {
        ($name:ident, $id:ty, state = $state:ty, instinct = $instinct:ty) => {
            struct $name;

            impl jungle_sdk::types::Creature for $name {
                type Id = jungle_sdk::types::Id<$id>;
                type State = $state;
                type Seed = $state;
                type Instinct = $instinct;
            }
        };

        ($name:ident, $id:ty, instinct = $instinct:ty) => {
            animal!($name, $id, state = (), instinct = $instinct);
        };

        ($name:ident, $id:ty, $instinct:ty) => {
            animal!($name, $id, SharedState, $instinct);
        };

        ($name:ident, $id:ty, $state:ty, $instinct:ty) => {
            struct $name;
            impl jungle_sdk::types::CreatureMember for $name {}

            impl jungle_sdk::types::Creature for $name {
                type Id = jungle_sdk::types::Id<$id>;
                type State = $state;
                type Seed = $state;
                type Instinct = $instinct;
            }

            #[jungle_sdk::inception::primitive(property = jungle_sdk::types::JungleCreatures)]
            impl jungle_sdk::types::Creatures for $name {
                type List = jungle_sdk::typosaurus::collections::sp::Node<$id, $name>;
            }

            #[jungle_sdk::inception::primitive(property = jungle_sdk::types::Ident)]
            impl jungle_sdk::types::Identified for $name {
                type Id = $id;
            }
        };
    }

    mod aspect;
    mod connection;
    mod conditional;
    mod progression;
    mod while_loop;
    mod zoo;
}
