use super::support::{define_action, maybe_delay};
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

impl<T> From<&T> for TemporalDependency {
    fn from(_value: &T) -> Self {
        Self::default()
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

define_action!(
    AdvanceAge,
    id = 50,
    dependency = TemporalDependency,
    in = u8,
    out = AgeState,
    err = String,
    act = |dependency, age_years| {
        async move {
            maybe_delay().await;
            let new_age = age_years.saturating_add(1);
            let adult_age = dependency.adult_age.max(1);
            let growth_percent =
                ((u16::from(new_age) * 100) / u16::from(adult_age)).min(100) as u8;
            let life_phase =
                classify_life_phase(new_age, dependency.adolescent_age, dependency.adult_age);
            Ok(AgeState {
                age_years: new_age,
                life_phase,
                growth_percent,
            })
        }
    }
);

define_action!(
    TickPerceivedTime,
    id = 51,
    dependency = TemporalDependency,
    in = (PerceivedTimeOfDay, u16),
    out = TimePerception,
    err = String,
    act = |dependency, (current, minutes_since_transition)| {
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
);

define_action!(
    EvaluateActivityWindow,
    id = 52,
    dependency = TemporalDependency,
    in = (DailyActivity, PerceivedTimeOfDay),
    out = bool,
    err = String,
    act = |_dependency, (activity, time_of_day)| {
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
);

define_action!(
    CelebrateBirthday,
    id = 53,
    dependency = TemporalDependency,
    in = AgeState,
    out = AgeState,
    err = String,
    act = |dependency, age| {
        async move {
            maybe_delay().await;
            let adult_age = dependency.adult_age.max(1);
            let growth_percent =
                ((u16::from(age.age_years) * 100) / u16::from(adult_age)).min(100) as u8;
            let life_phase =
                classify_life_phase(age.age_years, dependency.adolescent_age, dependency.adult_age);
            Ok(AgeState {
                age_years: age.age_years,
                life_phase,
                growth_percent,
            })
        }
    }
);

define_action!(
    Birth,
    id = 54,
    dependency = TemporalDependency,
    in = AgeState,
    out = AgeState,
    err = String,
    act = |dependency, age| {
        async move {
            maybe_delay().await;
            let adult_age = dependency.adult_age.max(1);
            let growth_percent =
                ((u16::from(age.age_years) * 100) / u16::from(adult_age)).min(100) as u8;
            let life_phase =
                classify_life_phase(age.age_years, dependency.adolescent_age, dependency.adult_age);
            Ok(AgeState {
                age_years: age.age_years,
                life_phase,
                growth_percent,
            })
        }
    }
);
