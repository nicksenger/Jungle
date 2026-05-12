macro_rules! define_action {
    (
        $name:ident,
        id = $id:expr,
        dependency = $dependency:ty,
        in = $input:ty,
        out = $output:ty,
        err = $error:ty,
        act = |$dep:ident, $in_arg:pat_param| $body:expr
    ) => {
        pub struct $name;

        impl jungle_types::ActionMember for $name {}

        impl jungle_types::Action for $name {
            type Id = u16;
            type Dependency = $dependency;
            type In = $input;
            type Out = $output;
            type Err = $error;

            fn act(
                $dep: &Self::Dependency,
                $in_arg: Self::In,
            ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
                let _ = $id;
                $body
            }
        }
    };
}

pub(crate) use define_action;
