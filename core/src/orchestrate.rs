use eyre::{Result};
use std::collections::HashMap;
use crate::ArgsOrchestrate;
use crate::scenario::Scenario;

pub fn orchestrate(args: ArgsOrchestrate) -> Result<()> {
    let mut scenarios = HashMap::new();

    // Open all scenarios
    for scenario_name in &args.scenarios {
        println!("Open scenario {}", scenario_name);
        let scenario = Scenario::open(scenario_name)?;
        scenarios.insert(scenario_name, scenario);
    }

    for (scenario_name, scenario) in &mut scenarios {
        println!("Run scenario {}", scenario_name);
        scenario.run()?;
    }

    Ok(())
}