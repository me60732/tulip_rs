use tulip_rs::indicators::roc::{indicator, TIndicatorState};
use tulip_rs::types::IndicatorError;

fn main() -> Result<(), IndicatorError> {
    // Sample input data (close prices)
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];
    let inputs = [&close[..close.len() - 5]];

    // Options (period)
    let options = [5.0];

    // Calculate the ROC indicator for the entire dataset
    let (outputs, mut state) = indicator(&inputs, &options, Some(&[true]))?;
    println!("ROC Indicator Result: {:?}", outputs[0]);
    println!("MOM Indicator Result: {:?}", outputs[1]);

    let new_inputs = [&close[close.len() - 5..]];

    // Calculate the ROC indicator from the previous state
    let new_outputs = state.batch_indicator(&new_inputs, None)?;
    println!(
        "ROC Indicator Result from Previous State: {:?}",
        new_outputs[0]
    );

    Ok(())
}
