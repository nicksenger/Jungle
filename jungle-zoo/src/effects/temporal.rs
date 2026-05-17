use super::support::maybe_delay;
use crate::state::{AgeState, DailyActivity, LifePhase, PerceivedTimeOfDay, TimePerception};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemporalDependency {
    pub adolescent_age: u8,
    pub adult_age: u8,
    pub minutes_per_segment: u16,
}

impl Default for TemporalDependency {
    fn default() -> Self {
        Self {
            adolescent_age: 3,
            adult_age: 8,
            minutes_per_segment: 360,
        }
    }
}

fn classify_life_phase(age_years: u8, adolescent_age: u8, adult_age: u8) -> LifePhase {
    if age_years < adolescent_age {
        LifePhase::Child
    } else if age_years < adult_age {
        LifePhase::Adolescent
    } else {
        LifePhase::Adult
    }
}

fn next_time_of_day(time_of_day: PerceivedTimeOfDay) -> PerceivedTimeOfDay {
    match time_of_day {
        PerceivedTimeOfDay::Morning => PerceivedTimeOfDay::Afternoon,
        PerceivedTimeOfDay::Afternoon => PerceivedTimeOfDay::Evening,
        PerceivedTimeOfDay::Evening => PerceivedTimeOfDay::Night,
        PerceivedTimeOfDay::Night => PerceivedTimeOfDay::Morning,
    }
}

pub struct AdvanceAge;

#[jungle_sdk::effect(id = 50)]
impl<J> jungle_sdk::types::Effect<J> for AdvanceAge {
    type In = u8;
    type Out = AgeState;
    type Err = String;

    fn effect(
        _jungle: &J,
        age_years: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        let dependency = TemporalDependency::default();
        async move {
            maybe_delay().await;
            let new_age = age_years.saturating_add(1);
            let adult_age = dependency.adult_age.max(1);
            let growth_percent = ((u16::from(new_age) * 100) / u16::from(adult_age)).min(100) as u8;
            let life_phase =
                classify_life_phase(new_age, dependency.adolescent_age, dependency.adult_age);
            Ok(AgeState {
                age_years: new_age,
                life_phase,
                growth_percent,
            })
        }
    }
}

pub struct TickPerceivedTime;

#[jungle_sdk::effect(id = 51)]
impl<J> jungle_sdk::types::Effect<J> for TickPerceivedTime {
    type In = (PerceivedTimeOfDay, u16);
    type Out = TimePerception;
    type Err = String;

    fn effect(
        _jungle: &J,
        (current, minutes_since_transition): Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        let dependency = TemporalDependency::default();
        async move {
            maybe_delay().await;
            let total = minutes_since_transition.saturating_add(dependency.minutes_per_segment);
            let next = if total < 720 {
                TimePerception {
                    current,
                    minutes_since_transition: total,
                }
            } else {
                TimePerception {
                    current: next_time_of_day(current),
                    minutes_since_transition: total - 720,
                }
            };
            Ok(next)
        }
    }
}

pub struct EvaluateActivityWindow;

#[jungle_sdk::effect(id = 52)]
impl<J> jungle_sdk::types::Effect<J> for EvaluateActivityWindow {
    type In = (DailyActivity, PerceivedTimeOfDay);
    type Out = bool;
    type Err = String;

    fn effect(
        _jungle: &J,
        (activity, time_of_day): Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            maybe_delay().await;
            let is_active = match activity {
                DailyActivity::Diurnal => {
                    matches!(
                        time_of_day,
                        PerceivedTimeOfDay::Morning | PerceivedTimeOfDay::Afternoon
                    )
                }
                DailyActivity::Nocturnal => {
                    matches!(
                        time_of_day,
                        PerceivedTimeOfDay::Evening | PerceivedTimeOfDay::Night
                    )
                }
            };
            Ok(is_active)
        }
    }
}

pub struct CelebrateBirthday;

#[jungle_sdk::effect(id = 53)]
impl<J> jungle_sdk::types::Effect<J> for CelebrateBirthday {
    type In = AgeState;
    type Out = AgeState;
    type Err = String;

    fn effect(
        _jungle: &J,
        age: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        let dependency = TemporalDependency::default();
        async move {
            maybe_delay().await;
            let adult_age = dependency.adult_age.max(1);
            let growth_percent =
                ((u16::from(age.age_years) * 100) / u16::from(adult_age)).min(100) as u8;
            let life_phase = classify_life_phase(
                age.age_years,
                dependency.adolescent_age,
                dependency.adult_age,
            );
            Ok(AgeState {
                age_years: age.age_years,
                life_phase,
                growth_percent,
            })
        }
    }
}

pub struct Birth;

#[jungle_sdk::effect(id = 54)]
impl<J> jungle_sdk::types::Effect<J> for Birth {
    type In = AgeState;
    type Out = AgeState;
    type Err = String;

    fn effect(
        _jungle: &J,
        age: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        let dependency = TemporalDependency::default();
        async move {
            maybe_delay().await;
            let adult_age = dependency.adult_age.max(1);
            let growth_percent =
                ((u16::from(age.age_years) * 100) / u16::from(adult_age)).min(100) as u8;
            let life_phase = classify_life_phase(
                age.age_years,
                dependency.adolescent_age,
                dependency.adult_age,
            );
            Ok(AgeState {
                age_years: age.age_years,
                life_phase,
                growth_percent,
            })
        }
    }
}
