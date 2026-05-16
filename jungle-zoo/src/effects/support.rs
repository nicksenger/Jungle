macro_rules! define_effect {
    (
        $name:ident,
        id = $id:expr,
        dependency = $dependency:ty,
        in = $input:ty,
        out = $output:ty,
        err = $error:ty,
        effect = |$dep:ident, $in_arg:pat_param| $body:expr
    ) => {
        pub struct $name;
        impl<J> jungle_types::Effect<J> for $name
        where
            $dependency: Default,
        {
            type Id = u16;
            type In = $input;
            type Out = $output;
            type Err = $error;

            fn effect(
                _jungle: &J,
                $in_arg: Self::In,
            ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
                let _ = $id;
                let $dep: $dependency = <$dependency>::default();
                $body
            }
        }
    };
}

pub(crate) use define_effect;

pub(crate) async fn maybe_delay() {
    #[cfg(feature = "delay")]
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
}
