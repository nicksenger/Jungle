#[cfg(test)]
extern crate jungle_sdk as inception;
#[cfg(test)]
extern crate jungle_sdk as jungle_types;

#[cfg(test)]
mod tests {
    use std::net::{Ipv6Addr, SocketAddr, UdpSocket};

    fn reserve_local_addr() -> SocketAddr {
        let socket = UdpSocket::bind((Ipv6Addr::LOCALHOST, 0))
            .expect("should bind temporary udp socket for test port reservation");
        socket
            .local_addr()
            .expect("temporary udp socket should expose local address")
    }

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
        ($name:ident, $id:ty, state = $state:ty, journey = $journey:ty, observe = true, perturb = true) => {
            struct $name;

            impl jungle_sdk::types::Animal for $name {
                type Id = jungle_sdk::types::Id<$id>;
                type State = $state;
                type Seed = $state;
                type Journey = $journey;
            }

            impl jungle_sdk::types::AnimalObservation for $name {
                type Adapter = jungle_sdk::types::ObserveObservation;
            }

            impl jungle_sdk::types::AnimalPerturbation for $name {
                type Adapter = jungle_sdk::types::TraitPerturbation;
            }
        };

        ($name:ident, $id:ty, state = $state:ty, journey = $journey:ty, perturb = true) => {
            struct $name;

            impl jungle_sdk::types::Animal for $name {
                type Id = jungle_sdk::types::Id<$id>;
                type State = $state;
                type Seed = $state;
                type Journey = $journey;
            }

            impl jungle_sdk::types::AnimalObservation for $name {
                type Adapter = jungle_sdk::types::NoopObservation;
            }

            impl jungle_sdk::types::AnimalPerturbation for $name {
                type Adapter = jungle_sdk::types::TraitPerturbation;
            }
        };

        ($name:ident, $id:ty, state = $state:ty, journey = $journey:ty, observe = true) => {
            struct $name;

            impl jungle_sdk::types::Animal for $name {
                type Id = jungle_sdk::types::Id<$id>;
                type State = $state;
                type Seed = $state;
                type Journey = $journey;
            }

            impl jungle_sdk::types::AnimalObservation for $name {
                type Adapter = jungle_sdk::types::ObserveObservation;
            }

            impl jungle_sdk::types::AnimalPerturbation for $name {
                type Adapter = jungle_sdk::types::NoopPerturbation;
            }
        };

        ($name:ident, $id:ty, state = $state:ty, journey = $journey:ty) => {
            struct $name;

            impl jungle_sdk::types::Animal for $name {
                type Id = jungle_sdk::types::Id<$id>;
                type State = $state;
                type Seed = $state;
                type Journey = $journey;
            }

            impl jungle_sdk::types::AnimalObservation for $name {
                type Adapter = jungle_sdk::types::NoopObservation;
            }

            impl jungle_sdk::types::AnimalPerturbation for $name {
                type Adapter = jungle_sdk::types::NoopPerturbation;
            }
        };

        ($name:ident, $id:ty, journey = $journey:ty) => {
            animal!($name, $id, state = (), journey = $journey);
        };

        ($name:ident, $id:ty, $journey:ty) => {
            animal!($name, $id, SharedState, $journey);
        };

        ($name:ident, $id:ty, $state:ty, $journey:ty) => {
            struct $name;
            impl jungle_sdk::types::AnimalMember for $name {}

            impl jungle_sdk::types::Animal for $name {
                type Id = jungle_sdk::types::Id<$id>;
                type State = $state;
                type Seed = $state;
                type Journey = $journey;
            }

            impl jungle_sdk::types::AnimalObservation for $name {
                type Adapter = jungle_sdk::types::NoopObservation;
            }

            impl jungle_sdk::types::AnimalPerturbation for $name {
                type Adapter = jungle_sdk::types::NoopPerturbation;
            }

            #[jungle_sdk::inception::primitive(property = jungle_sdk::types::JungleAnimals)]
            impl jungle_sdk::types::Animals for $name {
                type List = jungle_sdk::typosaurus::collections::sp::Node<$id, $name>;
            }

            #[jungle_sdk::inception::primitive(property = jungle_sdk::types::Ident)]
            impl jungle_sdk::types::Identified for $name {
                type Id = $id;
            }
        };

        ($name:ident, $id:ty, $state:ty, $journey:ty, observe = true, perturb = true) => {
            struct $name;
            impl jungle_sdk::types::AnimalMember for $name {}

            impl jungle_sdk::types::Animal for $name {
                type Id = jungle_sdk::types::Id<$id>;
                type State = $state;
                type Seed = $state;
                type Journey = $journey;
            }

            impl jungle_sdk::types::AnimalObservation for $name {
                type Adapter = jungle_sdk::types::ObserveObservation;
            }

            impl jungle_sdk::types::AnimalPerturbation for $name {
                type Adapter = jungle_sdk::types::TraitPerturbation;
            }

            #[jungle_sdk::inception::primitive(property = jungle_sdk::types::JungleAnimals)]
            impl jungle_sdk::types::Animals for $name {
                type List = jungle_sdk::typosaurus::collections::sp::Node<$id, $name>;
            }

            #[jungle_sdk::inception::primitive(property = jungle_sdk::types::Ident)]
            impl jungle_sdk::types::Identified for $name {
                type Id = $id;
            }
        };

        ($name:ident, $id:ty, $state:ty, $journey:ty, observe = true) => {
            struct $name;
            impl jungle_sdk::types::AnimalMember for $name {}

            impl jungle_sdk::types::Animal for $name {
                type Id = jungle_sdk::types::Id<$id>;
                type State = $state;
                type Seed = $state;
                type Journey = $journey;
            }

            impl jungle_sdk::types::AnimalObservation for $name {
                type Adapter = jungle_sdk::types::ObserveObservation;
            }

            impl jungle_sdk::types::AnimalPerturbation for $name {
                type Adapter = jungle_sdk::types::NoopPerturbation;
            }

            #[jungle_sdk::inception::primitive(property = jungle_sdk::types::JungleAnimals)]
            impl jungle_sdk::types::Animals for $name {
                type List = jungle_sdk::typosaurus::collections::sp::Node<$id, $name>;
            }

            #[jungle_sdk::inception::primitive(property = jungle_sdk::types::Ident)]
            impl jungle_sdk::types::Identified for $name {
                type Id = $id;
            }
        };
    }

    mod aspect;
    mod conditional;
    mod connection;
    mod integration;
    mod migration;
    mod replay;
    mod progression;
    mod select_join;
    mod sleep;
    mod traverse_replace;
    mod while_loop;
    mod zoo;
}
