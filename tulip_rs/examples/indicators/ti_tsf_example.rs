use tulip_rs::indicators::tsf::{indicator, TIndicatorState};

fn main() {
    let close = [
        81.59, 81.06, 82.87, 83.00, 83.61, 83.15, 82.84, 83.99, 84.55, 84.36, 85.53, 86.54, 86.89,
        87.77, 87.29,
    ];
    let options = [5.0]; // Period

    let inputs = [close.as_slice()];

    // Example with recent_only parameter set to false
    let (result, _) = match indicator(&inputs, &options, Some(&[true, true, true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("TSF Line: {:?}", result[0]);
    println!("Linreg Line: {:?}", result[1]);
    println!("Slope Line: {:?}", result[2]);
    println!("Intercept Line: {:?}", result[3]);

    let inputs2 = [&close[0..close.len() - 5]];
    let (result2, mut state2) = match indicator(&inputs2, &options, Some(&[true, true, true])) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };
    println!("\nPartial TSF Line: {:?}", result2[0]);

    let inputs3 = [&close[close.len() - 5..]];

    let result = match state2.batch_indicator(&inputs3, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\nRemaining TSF Line: {:?}", result[0]);
}
