use tulip_rs::{
    indicator_types::TIndicatorState,
    indicators::{ao_medprice::indicator, medprice},
};

fn main() {
    // Example input data: high and low prices

    let high = [
        82.15, 81.89, 83.03, 83.30, 83.85, 83.90, 83.33, 84.30, 84.84, 85.00, 85.90, 86.58, 86.98,
        88.00, 87.87,
    ]; // High prices
    let low = [
        81.29, 80.64, 81.31, 82.65, 83.07, 83.11, 82.49, 82.30, 84.15, 84.11, 84.03, 85.39, 85.76,
        87.17, 87.01,
    ]; // Low prices

    let mut new_high = high.to_vec();
    let mut new_low = low.to_vec();

    for _ in 0..4 {
        new_high.extend_from_slice(&high);
        new_low.extend_from_slice(&low);
    }

    // First calculate medprice from high and low
    let medprice_inputs = [new_high.as_slice(), new_low.as_slice()];
    let (medprice_outputs, _) = match medprice::indicator(&medprice_inputs, &[], None) {
        Ok(result) => result,
        Err(e) => panic!("Error calculating medprice: {}", e),
    };

    // Now use the medprice as input to AO medprice indicator
    let ao_inputs = [medprice_outputs[0].as_slice()];

    // Calculate the AO using the indicator function with optional outputs
    let (outputs, _) = match indicator(&ao_inputs, &[], Some(&[true, true])) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Full AO Line: {:?}", outputs[0]);
    println!("\nShort sma: {:?}", outputs[1]);
    println!("\nLong sma: {:?}", outputs[2]);

    // Demonstrate partial calculation with state
    let partial_medprice_inputs = [
        &new_high[..new_high.len() - 5],
        &new_low[..new_low.len() - 5],
    ];
    let (partial_medprice_outputs, _) =
        match medprice::indicator(&partial_medprice_inputs, &[], None) {
            Ok(result) => result,
            Err(e) => panic!("Error calculating partial medprice: {}", e),
        };

    let partial_ao_inputs = [partial_medprice_outputs[0].as_slice()];
    let (outputs, mut state) = match indicator(&partial_ao_inputs, &[], None) {
        Ok(result) => result,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\n\nPartial AO Line: {:?}", outputs[0]);

    // Calculate remaining medprice values
    let remaining_medprice_inputs = [
        &new_high[new_high.len() - 5..],
        &new_low[new_low.len() - 5..],
    ];
    let (remaining_medprice_outputs, _) =
        match medprice::indicator(&remaining_medprice_inputs, &[], None) {
            Ok(result) => result,
            Err(e) => panic!("Error calculating remaining medprice: {}", e),
        };

    let remaining_ao_inputs = [remaining_medprice_outputs[0].as_slice()];
    let outputs = state
        .batch_indicator(&remaining_ao_inputs, None)
        .expect("batch_indicator failed");
    println!("\nFinal AO Line: {:?}", outputs[0]);
}
