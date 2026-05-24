//use crate::common::validate_inputs;
use crate::indicators::mfi::{
    min_data, output_length, IndicatorState as State, INPUTS_WIDTH, OPTIONS_WIDTH,
};
use crate::indicators::simd_indicators::mfi_simd::assets::SimdState;
use crate::indicators::simd_indicators::road_train::{Asset, Driver, PrimeMover};
use crate::types::IndicatorError;
use crate::{common::validate_options, common_simd::assets::validate_inputs};
use std::simd::Simd;

/// SIMD driver that advances the Money Flow Index (MFI) across `N` asset lanes per
/// scheduling epoch.
struct MfiDriver {
    want_optional_outputs: bool,
}

impl Driver<State> for MfiDriver {
    /// Processes one epoch of bars for `N` assets simultaneously using SIMD.
    ///
    /// Reads from `inputs[asset][field]` (high, low, close, volume), writes the MFI to
    /// `outputs[asset][0]`, optional typical price to `outputs[asset][1]`, and updates
    /// `states[asset]` in place.
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        mut outputs: Vec<Vec<&mut [f64]>>,
        mut states: Vec<&mut State>,
        _options: Vec<Option<&()>>,
    ) {
        let mut state = SimdState::<N>::new(&mut states);
        let len = inputs[0][0].len();

        let want_typprice = self.want_optional_outputs;

        //collect outputs
        let (mfi_line_ptr, typprice_line_ptr) =
            crate::extract_output_ptrs!(outputs, N, mfi_line_ptr, typprice_line_ptr);

        let (high_ptrs, low_ptrs, close_ptrs, volume_ptrs) =
            crate::extract_input_ptrs!(inputs, N, high_ptrs, low_ptrs, close_ptrs, volume_ptrs);

        // Optimization 3: Simplified main loop with pre-computed offsets
        for i in 0..len {
            // Get inputs arrays for stocks
            let (high, low, close, volume) = crate::extract_simd_inputs_at_index!(
                i,
                N,
                high @ high_ptrs,
                low @ low_ptrs,
                close @ close_ptrs,
                volume @ volume_ptrs
            );

            let mfi = unsafe { state.calc_unchecked_simd(high, low, close, volume) };
            //unsafe { calc_simd(&mut state, high, low, close, multiplier) };
            // Store results using pre-computed pointers
            crate::write_simd_at_indices!(N, i,
                mfi_line_ptr => mfi
            );
            crate::store_simd_optional_outputs!(i, N,
                want_typprice, typprice_line_ptr => state.typprice
            );
        }

        // Update states efficiently
        state.write_states(&mut states);
    }
}

/// Calculates the Money Flow Index (MFI) for `N` assets simultaneously using SIMD
/// parallelism.
///
/// Uses the [`PrimeMover`] scheduler to batch assets into SIMD-width groups.
///
/// # Arguments
/// * `inputs` - An array of `N` asset input sets; `inputs[i]` is `[&[f64]; INPUTS_WIDTH]`
///   containing `[high, low, close, volume]` for asset `i`.
/// * `options` - Shared options slice; `options[0]` is the period.
/// * `optional_outputs` - Optional slice selecting extra outputs: index `0` = `typprice`.
///
/// # Returns
/// `Ok((outputs, states))` where `outputs[i][0]` is the MFI for asset `i`,
/// `outputs[i][1]` is the optional typical price, and `states[i]` is the final
/// [`IndicatorState`] for asset `i`.
/// Returns `Err(IndicatorError)` if any input slice is too short or options are invalid.
pub fn indicator_by_assets<const N: usize>(
    inputs: &[&[&[f64]; INPUTS_WIDTH]; N], //stock[ fields [ field [f64] ] ]
    options: &[f64; OPTIONS_WIDTH],
    optional_outputs: Option<&[bool]>,
) -> Result<(Vec<Vec<Vec<f64>>>, Vec<State>), IndicatorError> {
    validate_inputs::<INPUTS_WIDTH>(inputs, min_data(options))?;
    validate_options(options)?;
    let period = options[0] as usize;

    let mut road_train = PrimeMover::<N, State>::new();
    let mut output_buffers = Vec::with_capacity(N);
    let mut want_optional_outputs = false;
    for i in 0..N {
        let asset_inputs = vec![
            inputs[i][0], // high
            inputs[i][1], // low
            inputs[i][2], // close
            inputs[i][3], // volume
        ];

        let (mfi_line, mut typprice_line) = {
            let len = inputs[i][0].len();
            let capacity = output_length(len, options);
            (
                crate::uninit_vec!(f64, capacity),
                crate::init_optional_outputs_eff!(
                    optional_outputs, &[false],
                    typprice_line: len
                ),
            )
        };

        let state = State::init_state(
            (inputs[i][0], inputs[i][1], inputs[i][2], inputs[i][3]),
            period,
            &mut typprice_line,
        );

        if i == 0 {
            (_, want_optional_outputs) = crate::calc_want_flags!(typprice_line);
        }
        let mut starts = [0; 2];
        starts[1] = crate::slice_outputs_start!(mfi_line.len(), typprice_line);

        let mut output_buffer = vec![mfi_line, typprice_line];

        //let adosc_len = output_buffer[0].len();
        let mut asset_outputs = Vec::with_capacity(output_buffer.len());

        for j in 0..output_buffer.len() {
            unsafe {
                //let slice_len = output_buffer.len() - starts[j];
                // Get a mutable reference to the output buffer for this asset
                let output_buffer = &mut output_buffer[j];
                asset_outputs.push(std::slice::from_raw_parts_mut(
                    output_buffer.as_mut_ptr().add(starts[j]), //slice from
                    output_buffer.len(),                       // slice to
                ));
            }
        }

        road_train.add_asset(Asset::new(
            asset_inputs,
            asset_outputs,
            i,
            period,
            0,
            state,
            None,
        ));
        output_buffers.push(output_buffer);
    }

    let mut driver = MfiDriver {
        want_optional_outputs,
    };
    let states = road_train.drive(&mut driver);

    Ok((output_buffers, states))
}
