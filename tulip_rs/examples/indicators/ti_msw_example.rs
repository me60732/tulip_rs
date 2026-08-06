use tulip_rs::indicators::msw::{Indicator, Msw, TIndicatorState};
use tulip_rs::types::IndicatorError;

fn main() -> Result<(), IndicatorError> {
    // Sample input data (close prices)
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];
    // Options (period)
    let options = [5.0];

    let inputs = [close.as_slice()];

    let (outputs, _) = Msw::indicator(&inputs, &options, None)?;
    println!("FULL MSW Sine: {:?}", outputs[0]);
    println!("FULL MSW Lead: {:?}", outputs[1]);

    let inputs2 = [&close[..close.len() - 5]];
    // Calculate the MSW indicator for the partial dataset
    let (outputs2, mut state) = Msw::indicator(&inputs2, &options, None)?;
    println!("\nMSW Sine: {:?}", outputs2[0]);
    println!("MSW Lead: {:?}", outputs2[1]);

    // New input data for updating the indicator
    let new_inputs = [&close[close.len() - 5..]];

    // Calculate the MSW indicator from the previous state
    let new_outputs = state.batch_indicator(&new_inputs, None)?;
    println!("\nNew MSW Sine: {:?}", new_outputs[0]);
    println!("New MSW Lead: {:?}", new_outputs[1]);

    Ok(())
}
