use crate::CronExpr;
use chrono::Utc;
use jungle_sdk::effect;
use std::future::Future;
use std::str::FromStr;
use std::time::Duration;

pub struct ParseNext;
#[effect(id = 0)]
impl<J> Effect<J> for ParseNext {
    type In = CronExpr;
    type Out = Duration;
    type Err = String;

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
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

pub struct RunBash;
#[effect(id = 1)]
impl<J> Effect<J> for RunBash {
    type In = String;
    type Out = ();
    type Err = String;

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
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
