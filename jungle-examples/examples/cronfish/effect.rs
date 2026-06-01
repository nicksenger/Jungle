use crate::CronExpr;
use chrono::Utc;
use jungle_sdk::effect;
use std::str::FromStr;
use std::time::Duration;

pub struct CronfishUntilNextFireEffect;
#[effect(id = 0)]
impl<J> Effect<J> for CronfishUntilNextFireEffect {
    type In = CronExpr;
    type Out = Duration;
    type Err = String;

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        let result = (|| {
            let schedule =
                cron::Schedule::from_str(&input).map_err(|err| format!("cron parse failed: {err}"))?;
            let now = Utc::now();
            let next = schedule
                .after(&now)
                .next()
                .ok_or_else(|| "cron schedule has no future fire time".to_string())?;
            let remaining = next.signed_duration_since(now);
            remaining
                .to_std()
                .map_err(|err| format!("invalid fire duration: {err}"))
        })();
        std::future::ready(result)
    }
}

pub struct CronfishFiredEffect;
#[effect(id = 1)]
impl<J> Effect<J> for CronfishFiredEffect {
    type In = String;
    type Out = ();
    type Err = String;

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        let result = (|| {
            let status = std::process::Command::new("bash")
                .arg("-lc")
                .arg(&input)
                .status()
                .map_err(|err| format!("failed to run fired script: {err}"))?;
            if status.success() {
                Ok(())
            } else {
                Err(format!("fired script failed with status: {status}"))
            }
        })();
        std::future::ready(result)
    }
}
