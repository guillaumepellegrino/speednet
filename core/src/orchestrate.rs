use eyre::{Result};
use crate::ArgsOrchestrate;
use crate::scenario::Scenario;

pub fn orchestrate(args: ArgsOrchestrate) -> Result<()> {
    let mut scenarios = Vec::new();

    // Open all scenarios
    for scenario_name in &args.scenarios {
        println!("Open scenario {}", scenario_name);
        let scenario = Scenario::open(scenario_name)?;
        scenarios.push(scenario);
    }

    // Run all scenarios
    for scenario in &mut scenarios {
        scenario.run(args.time)?;
    }

    Ok(())
}
