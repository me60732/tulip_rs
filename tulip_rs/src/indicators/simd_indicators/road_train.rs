//use std::simd::{};

macro_rules! generate_simd_match {
    ($asset_count:expr, $driver:expr, $inputs:expr, $outputs:expr, $states:expr, $options:expr, $max_width:expr) => {
        generate_simd_match!(@internal $asset_count, $driver, $inputs, $outputs, $states, $options, $max_width, [1, 2, 4, 8, 16, 32, 64])
    };

    (@internal $asset_count:expr, $driver:expr, $inputs:expr, $outputs:expr, $states:expr, $options:expr, $max_width:expr, [$($width:expr),*]) => {
        match $asset_count {
            $(
                $width if $width <= $max_width => {
                    $driver.next_run::<$width>($inputs, $outputs, $states, $options)
                },
            )*
            _ => {}
        }
    };
}

/// Generic SIMD dispatch trait. Each indicator implements this to process `N` asset lanes
/// per scheduling epoch.
pub trait Driver<S, O = ()> {
    /// Advances N asset lanes by one epoch. `inputs[asset][field]` are the input slices for
    /// each asset/field; `outputs[asset][output]` are the corresponding output slices;
    /// `states[asset]` are mutable per-asset state references; `options[asset]` are per-asset
    /// option references (`None` in shared-option mode).
    fn next_run<const N: usize>(
        &mut self,
        inputs: Vec<Vec<&[f64]>>,
        outputs: Vec<Vec<&mut [f64]>>,
        states: Vec<&mut S>,
        options: Vec<Option<&O>>,
    );
}

/// Represents a single asset being managed by the [`PrimeMover`] scheduler, holding borrowed
/// input/output slices, the per-asset state, and scheduling metadata (pickup mile, offsets, etc.).
pub struct Asset<'a, S, O = ()> {
    /// Per-asset indicator state.
    pub state: S,
    /// Borrowed input price slices for this asset.
    pub inputs: Vec<&'a [f64]>,
    /// Borrowed mutable output slices for this asset.
    pub outputs: Vec<&'a mut [f64]>,
    /// Original insertion index, used to restore result order.
    pub original_idx: usize,
    /// The odometer value at which this asset first becomes ready for processing.
    pub pickup_mile: usize,
    /// Warm-up bars to prepend before the first output-producing bar.
    start_offset: usize,
    /// Current read position in the input slices.
    pub inputs_idx: usize,
    /// Current write position in the output slices.
    pub outputs_idx: usize,
    /// Optional per-asset options; `None` when all assets share the same options.
    pub options: Option<&'a O>,
}

impl<'a, S, O> Asset<'a, S, O> {
    /// Constructs an `Asset` with the given inputs, outputs, original index, warm-up offset,
    /// state, and optional options.
    pub fn new(
        inputs: Vec<&'a [f64]>,
        outputs: Vec<&'a mut [f64]>,
        idx: usize,
        inputs_idx: usize,
        start_offset: usize,
        state: S,
        options: Option<&'a O>,
    ) -> Self {
        Self {
            state,
            original_idx: idx,
            pickup_mile: 0,
            inputs,
            inputs_idx,
            start_offset,
            outputs_idx: 0,
            outputs,
            options,
        }
    }
}

/// A batch of `asset_count` assets ready to be dispatched together in a single SIMD
/// [`Driver::next_run`] call.
pub struct Trailer<'a, S, O = ()> {
    /// Mutable references to per-asset indicator states for this batch.
    pub states: Vec<&'a mut S>,
    /// Input slices for each asset and field in this batch: `inputs[asset][field]`.
    pub inputs: Vec<Vec<&'a [f64]>>,
    /// Mutable output slices for each asset and output in this batch: `outputs[asset][output]`.
    pub outputs: Vec<Vec<&'a mut [f64]>>,
    /// Number of assets in this trailer batch (always a power of two, ≤ N).
    pub asset_count: usize,
    /// Per-asset option references; `None` in shared-option mode.
    pub options: Vec<Option<&'a O>>,
}

/// Parallel SIMD scheduler. Accumulates up to `N` assets, then dispatches them through a
/// [`Driver`] in batches until all input data is consumed.
#[derive(Default)]
pub struct PrimeMover<'a, const N: usize, S, O = ()> {
    assets: Vec<Asset<'a, S, O>>,
    odometer: usize,

    // Optimization 1: Reusable buffers to avoid allocations
    to_pickup_buffer: Vec<usize>,
    pickup_miles_buffer: Vec<usize>,
}

impl<'a, const N: usize, S: 'a, O: 'a> PrimeMover<'a, N, S, O> {
    /// Creates an empty `PrimeMover` with pre-allocated internal buffers.
    pub fn new() -> Self {
        Self {
            odometer: 0,
            assets: Vec::with_capacity(N),
            // Pre-allocate buffers with reasonable capacity
            to_pickup_buffer: Vec::with_capacity(N),
            pickup_miles_buffer: Vec::with_capacity(N),
        }
    }

    /// Runs the full scheduling loop: repeatedly calls [`next`] to get [`Trailer`] batches,
    /// dispatches each to `driver.next_run`, and returns the final states in original insertion order.
    pub fn drive<D: Driver<S, O>>(mut self, driver: &mut D) -> Vec<S> {
        while let Some(trailers) = self.next() {
            for trailer in trailers {
                generate_simd_match!(
                    trailer.asset_count,
                    driver,
                    trailer.inputs,
                    trailer.outputs,
                    trailer.states,
                    trailer.options,
                    N
                );
            }
        }

        self.into_results()
    }

    /// Advances the internal odometer and returns the next set of [`Trailer`]s to process,
    /// or `None` when all assets are exhausted.
    pub fn next(&mut self) -> Option<Vec<Trailer<'a, S, O>>> {
        // Clear reusable buffers
        self.to_pickup_buffer.clear();
        self.pickup_miles_buffer.clear();

        // Single pass through assets to collect all needed information
        let mut max_input_len = 0;
        let mut min_future_pickup = None;

        for (i, asset) in self.assets.iter().enumerate() {
            let pickup_mile = asset.pickup_mile;
            let input_len = asset.inputs[0].len();

            // Track max input length for fallback calculation
            max_input_len = max_input_len.max(input_len);

            // Check if asset is ready for pickup
            if pickup_mile <= self.odometer {
                self.to_pickup_buffer.push(i);
            } else {
                // Track future pickup miles for bar_count calculation
                min_future_pickup = match min_future_pickup {
                    None => Some(pickup_mile),
                    Some(current_min) => Some(current_min.min(pickup_mile)),
                };
            }
        }

        if self.to_pickup_buffer.is_empty() {
            return None;
        }

        // Calculate bar_count using data from single iteration
        let bar_count = match min_future_pickup {
            Some(next_mile) => next_mile - self.odometer,
            None => {
                // No more pickups, process to the end of inputs
                max_input_len.saturating_sub(self.odometer)
            }
        };

        if bar_count == 0 {
            return None;
        }

        let mut trailers = Vec::new();
        let mut idx = 0;
        let total = self.to_pickup_buffer.len();

        while idx < total {
            let mut width = N;
            while width > (total - idx) {
                width /= 2;
            }

            let mut inputs = Vec::with_capacity(width);
            let mut outputs = Vec::with_capacity(width);
            let mut states = Vec::with_capacity(width);
            let mut options = Vec::with_capacity(width);

            /*let mut max_start_offset = 0;
            for i in 0..width {
                let asset_idx = self.to_pickup_buffer[idx + i];
                unsafe {
                    let asset_ptr = self.assets.as_ptr().add(asset_idx);
                    let asset = &*asset_ptr;
                    max_start_offset = max_start_offset.max(asset.start_offset);
                }
            }*/

            for i in 0..width {
                let asset_idx = self.to_pickup_buffer[idx + i];

                unsafe {
                    let asset_ptr = self.assets.as_mut_ptr().add(asset_idx);
                    let asset = &mut *asset_ptr;

                    states.push(&mut asset.state);
                    options.push(asset.options);
                    // Handle input slicing (unchanged)
                    let input_start = if asset.inputs_idx >= asset.start_offset {
                        asset.inputs_idx - asset.start_offset
                    } else {
                        0
                    };
                    //let input_start = asset.inputs_idx - start_offset;
                    let input_end = input_start + asset.start_offset + bar_count;

                    let input_len = asset.inputs[0].len();
                    let actual_start = input_start.min(input_len);
                    let actual_end = input_end.min(input_len);

                    let asset_inputs: Vec<&[f64]> = asset
                        .inputs
                        .iter()
                        .map(|input| &input[actual_start..actual_end])
                        .collect();
                    inputs.push(asset_inputs);

                    asset.inputs_idx = actual_end;

                    // Handle output slicing for mutable slices
                    let output_len = asset.outputs[0].len();
                    let output_start = asset.outputs_idx.min(output_len);
                    let output_end = (output_start + bar_count).min(output_len);

                    let mut asset_outputs: Vec<&mut [f64]> =
                        Vec::with_capacity(asset.outputs.len());

                    // We need to use raw pointers to create multiple mutable slices from the same source
                    for output_slice in asset.outputs.iter_mut() {
                        let ptr = output_slice.as_mut_ptr();
                        let len = output_slice.len();

                        // Create a sub-slice using raw pointers
                        let sub_slice = std::slice::from_raw_parts_mut(
                            ptr.add(output_start.min(len)),
                            (output_end - output_start.min(len)).min(len - output_start.min(len)),
                        );
                        asset_outputs.push(sub_slice);
                    }

                    outputs.push(asset_outputs);
                    asset.outputs_idx = output_end;
                }
            }

            trailers.push(Trailer {
                asset_count: states.len(),
                states,
                inputs,
                outputs,
                options,
            });

            idx += width;
        }

        self.odometer += bar_count;
        Some(trailers)
    }

    // Keep add_asset method - need to update for new output type
    /*pub fn add_asset(&mut self, asset: Asset<'a, S, O>) {
        self.assets.push(asset);
        if self.assets.len() == self.assets.capacity() {
            self.assets
                .sort_by_key(|asset| std::cmp::Reverse(asset.inputs[0].len()));

            let len = self.assets[0].inputs[0].len();
            for asset in self.assets.iter_mut().skip(1) {
                asset.pickup_mile = len - asset.inputs[0].len();
            }
        }
    }*/
    /// Adds an asset. Once the full `N` assets have been added, computes each asset's pickup
    /// mile (data offset + warm-up bars) and sorts by earliest availability.
    pub fn add_asset(&mut self, asset: Asset<'a, S, O>) {
        self.assets.push(asset);
        if self.assets.len() == self.assets.capacity() {
            // First, find the maximum input length
            let max_len = self
                .assets
                .iter()
                .map(|a| a.inputs[0].len())
                .max()
                .unwrap_or(0);

            // Calculate pickup_mile for each asset
            for asset in self.assets.iter_mut() {
                let length_diff = max_len - asset.inputs[0].len();
                let init_requirement = asset.inputs_idx; // period - 1 for most indicators

                // pickup_mile accounts for both data availability and initialization
                asset.pickup_mile = length_diff + init_requirement;
                //println!("pickup mile: {:?}", asset.pickup_mile);
            }

            // Sort by pickup_mile (lowest first = earliest availability)
            self.assets.sort_by_key(|asset| asset.pickup_mile);
        }
        self.odometer = self.assets[0].pickup_mile;
    }
    /// Consumes the scheduler and returns the final per-asset states in the original insertion order.
    pub fn into_results(self) -> Vec<S> {
        //let asset_count = self.assets.len();

        // Collect assets with their original indices
        let mut indexed_data: Vec<(usize, S)> = self
            .assets
            .into_iter()
            .map(|asset| (asset.original_idx, asset.state))
            .collect();

        // Sort by original index to restore original order
        indexed_data.sort_by_key(|(original_idx, _)| *original_idx);

        // Extract states in original order
        indexed_data.into_iter().map(|(_, state)| state).collect()
    }
}
