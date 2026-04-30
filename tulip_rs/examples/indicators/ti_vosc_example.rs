use tulip_rs::indicators::vosc::{indicator, TIndicatorState};
fn main() {
    let volume = [
        5653100.0, 6447400.0, 7690900.0, 3831400.0, 4455100.0, 3798000.0, 3936200.0, 4732000.0,
        4841300.0, 3915300.0, 6830800.0, 6694100.0, 5293600.0, 7985800.0, 4807900.0,
    ]; //Volume
    let options = [2.0, 5.0];

    let inputs = [volume.as_slice()];

    let (outputs, _) = match indicator(&inputs, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("Full vosc Line: {:?}", outputs[0]);

    let inputs2 = [&volume[..volume.len() - 5]];

    let (outputs2, mut state) = match indicator(&inputs2, &options, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\nPartial vosc Line: {:?}", outputs2[0]);

    let inputs3 = [&volume[volume.len() - 5..]];
    let new_outputs = match state.batch_indicator(&inputs3, None) {
        Ok(r) => r,
        Err(e) => panic!("Error: {}", e),
    };

    println!("\nFinal vosc Line: {:?}", new_outputs[0]);
}
