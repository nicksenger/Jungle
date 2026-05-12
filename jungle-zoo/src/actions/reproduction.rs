use super::support::define_action;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReproductionDependency {
    pub bask_boost: u8,
    pub egg_base_count: u8,
}

impl Default for ReproductionDependency {
    fn default() -> Self {
        Self {
            bask_boost: 5,
            egg_base_count: 8,
        }
    }
}

impl<T> From<&T> for ReproductionDependency {
    fn from(_value: &T) -> Self {
        Self::default()
    }
}

define_action!(
    Bask,
    id = 30,
    dependency = ReproductionDependency,
    in = u8,
    out = u8,
    err = String,
    act = |dependency, melanin| {
        std::future::ready(Ok(melanin.saturating_add(dependency.bask_boost)))
    }
);

define_action!(
    LayEggs,
    id = 31,
    dependency = ReproductionDependency,
    in = bool,
    out = u8,
    err = String,
    act = |dependency, has_osteoderms| {
        let clutch = if has_osteoderms {
            dependency.egg_base_count.saturating_add(2)
        } else {
            dependency.egg_base_count
        };
        std::future::ready(Ok(clutch))
    }
);
