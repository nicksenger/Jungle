#[cfg(test)]
extern crate jungle as inception;
#[cfg(test)]
extern crate jungle as jungle_types;

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
            impl jungle::types::ActionMember for $name {}

            impl jungle::types::Action for $name {
                type Id = jungle::types::Id<$id>;
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

            #[jungle::inception::primitive(property = jungle::types::JungleActions)]
            impl jungle::types::Actions for $name {
                type List = jungle::typosaurus::collections::sp::Node<$id, $name>;
            }

            #[jungle::inception::primitive(property = jungle::types::Ident)]
            impl jungle::types::Identified for $name {
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
            impl jungle::types::ActionMember for $name {}

            impl jungle::types::Action for $name {
                type Id = jungle::types::Id<$id>;
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
            impl jungle::types::ActionMember for $name {}

            impl jungle::types::Action for $name {
                type Id = jungle::types::Id<$id>;
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

            #[jungle::inception::primitive(property = jungle::types::JungleActions)]
            impl jungle::types::Actions for $name {
                type List = jungle::typosaurus::collections::sp::Node<$id, $name>;
            }

            #[jungle::inception::primitive(property = jungle::types::Ident)]
            impl jungle::types::Identified for $name {
                type Id = $id;
            }
        };
    }

    macro_rules! animal {
        ($name:ident, $id:ty, state = $state:ty, journey = $journey:ty, observe = true, perturb = true) => {
            struct $name;

            impl jungle::types::Animal for $name {
                type Id = jungle::types::Id<$id>;
                type Generation = jungle::typosaurus::num::consts::U0;
                type State = $state;
                type Seed = $state;
                type Journey = $journey;
            }

            impl jungle::types::AnimalObservation for $name {
                type Adapter = jungle::types::ObserveObservation;
            }

            impl jungle::types::AnimalPerturbation for $name {
                type Adapter = jungle::types::TraitPerturbation;
            }
        };

        ($name:ident, $id:ty, state = $state:ty, journey = $journey:ty, perturb = true) => {
            struct $name;

            impl jungle::types::Animal for $name {
                type Id = jungle::types::Id<$id>;
                type Generation = jungle::typosaurus::num::consts::U0;
                type State = $state;
                type Seed = $state;
                type Journey = $journey;
            }

            impl jungle::types::AnimalObservation for $name {
                type Adapter = jungle::types::NoopObservation;
            }

            impl jungle::types::AnimalPerturbation for $name {
                type Adapter = jungle::types::TraitPerturbation;
            }
        };

        ($name:ident, $id:ty, state = $state:ty, journey = $journey:ty, observe = true) => {
            struct $name;

            impl jungle::types::Animal for $name {
                type Id = jungle::types::Id<$id>;
                type Generation = jungle::typosaurus::num::consts::U0;
                type State = $state;
                type Seed = $state;
                type Journey = $journey;
            }

            impl jungle::types::AnimalObservation for $name {
                type Adapter = jungle::types::ObserveObservation;
            }

            impl jungle::types::AnimalPerturbation for $name {
                type Adapter = jungle::types::NoopPerturbation;
            }
        };

        ($name:ident, $id:ty, state = $state:ty, journey = $journey:ty) => {
            struct $name;

            impl jungle::types::Animal for $name {
                type Id = jungle::types::Id<$id>;
                type Generation = jungle::typosaurus::num::consts::U0;
                type State = $state;
                type Seed = $state;
                type Journey = $journey;
            }

            impl jungle::types::AnimalObservation for $name {
                type Adapter = jungle::types::NoopObservation;
            }

            impl jungle::types::AnimalPerturbation for $name {
                type Adapter = jungle::types::NoopPerturbation;
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
            impl jungle::types::AnimalMember for $name {}

            impl jungle::types::Animal for $name {
                type Id = jungle::types::Id<$id>;
                type Generation = jungle::typosaurus::num::consts::U0;
                type State = $state;
                type Seed = $state;
                type Journey = $journey;
            }

            impl jungle::types::AnimalObservation for $name {
                type Adapter = jungle::types::NoopObservation;
            }

            impl jungle::types::AnimalPerturbation for $name {
                type Adapter = jungle::types::NoopPerturbation;
            }

            #[jungle::inception::primitive(property = jungle::types::JungleAnimals)]
            impl jungle::types::Animals for $name {
                type List = jungle::typosaurus::collections::sp::Node<$id, $name>;
            }

            #[jungle::inception::primitive(property = jungle::types::Ident)]
            impl jungle::types::Identified for $name {
                type Id = $id;
            }
        };

        ($name:ident, $id:ty, $state:ty, $journey:ty, observe = true, perturb = true) => {
            struct $name;
            impl jungle::types::AnimalMember for $name {}

            impl jungle::types::Animal for $name {
                type Id = jungle::types::Id<$id>;
                type Generation = jungle::typosaurus::num::consts::U0;
                type State = $state;
                type Seed = $state;
                type Journey = $journey;
            }

            impl jungle::types::AnimalObservation for $name {
                type Adapter = jungle::types::ObserveObservation;
            }

            impl jungle::types::AnimalPerturbation for $name {
                type Adapter = jungle::types::TraitPerturbation;
            }

            #[jungle::inception::primitive(property = jungle::types::JungleAnimals)]
            impl jungle::types::Animals for $name {
                type List = jungle::typosaurus::collections::sp::Node<$id, $name>;
            }

            #[jungle::inception::primitive(property = jungle::types::Ident)]
            impl jungle::types::Identified for $name {
                type Id = $id;
            }
        };

        ($name:ident, $id:ty, $state:ty, $journey:ty, observe = true) => {
            struct $name;
            impl jungle::types::AnimalMember for $name {}

            impl jungle::types::Animal for $name {
                type Id = jungle::types::Id<$id>;
                type Generation = jungle::typosaurus::num::consts::U0;
                type State = $state;
                type Seed = $state;
                type Journey = $journey;
            }

            impl jungle::types::AnimalObservation for $name {
                type Adapter = jungle::types::ObserveObservation;
            }

            impl jungle::types::AnimalPerturbation for $name {
                type Adapter = jungle::types::NoopPerturbation;
            }

            #[jungle::inception::primitive(property = jungle::types::JungleAnimals)]
            impl jungle::types::Animals for $name {
                type List = jungle::typosaurus::collections::sp::Node<$id, $name>;
            }

            #[jungle::inception::primitive(property = jungle::types::Ident)]
            impl jungle::types::Identified for $name {
                type Id = $id;
            }
        };
    }

    mod adapt_helpers;
    mod aspect;
    mod conditional;
    mod connection;
    mod integration;
    mod migration;
    mod progression;
    mod replay;
    mod select_join;
    mod sleep;
    mod transparent_metadata;
    mod traverse_replace;
    mod versioning;
    mod while_loop;
    mod zoo;
}
