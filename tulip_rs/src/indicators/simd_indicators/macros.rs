#[macro_export]
macro_rules! create_simd_vec_from_inputs {
    ($input_ptr:ident, $width:expr, $length:expr) => {
        {
            let mut simd_vec: Vec<Simd<f64, $width>> = crate::uninit_vec!(Simd<f64, $width>, $length);
            for i in 0..$length {
                let mut values = [0.0; $width];
                for j in 0..$width {
                    unsafe {
                        values[j] = *$input_ptr[j].add(i);
                    }
                }
                unsafe {
                    *simd_vec.get_unchecked_mut(i) = Simd::from_array(values);
                }
            }
            simd_vec
        }
    };
}

#[macro_export]
macro_rules! extract_input_ptrs {
    ($inputs:expr, $width:expr, $($field_name:ident),*) => {
        {
            let mut _index = 0;
            (
                $(
                    {
                        let $field_name: [*const f64; $width] = std::array::from_fn(|j| unsafe {
                            $inputs.get_unchecked(j).get_unchecked(_index).as_ptr()
                        });
                        _index += 1;
                        $field_name
                    }
                ),*
            )
        }
    };
}

#[macro_export]
macro_rules! extract_output_ptrs {
    ($outputs:expr, $width:expr, $($field_name:ident),*) => {
        {
            let mut _index = 0;
            (
                $(
                    {
                        let $field_name: [*mut f64; $width] = std::array::from_fn(|j| unsafe {
                            $outputs.get_unchecked_mut(j).get_unchecked_mut(_index).as_mut_ptr()
                        });
                        _index += 1;
                        $field_name
                    }
                ),*
            )
        }
    };
}
/*#[macro_export]
macro_rules! extract_simd_inputs_at_index {
    ($index:expr, $width:expr, $($ptr_array:ident),*) => {
        {
            (
                $(
                    Simd::from_array(std::array::from_fn(|j| unsafe {
                        *$ptr_array[j].add($index)
                    }))
                ),*
            )
        }
    };
}*/

#[macro_export]
macro_rules! extract_simd_inputs_at_index_array {
    ($indices:expr, $width:expr, $($var_name:ident @ $ptr_array:ident),*) => {
        {
            // Declare arrays for each named variable
            $(let mut $var_name = [0.0; $width];)*

            // Loop through all lanes, extracting from different input arrays at different indices
            for j in 0..$width {
                unsafe {
                    let index = $indices[j];
                    $($var_name[j] = *$ptr_array[j].add(index);)*
                }
            }

            // Convert to SIMD vectors and return as tuple
            ($(Simd::from_array($var_name)),*)
        }
    };
}

#[macro_export]
macro_rules! extract_array_inputs_at_index_array {
    ($indices:expr, $width:expr, $($var_name:ident @ $ptr_array:ident),*) => {
        {
            // Declare arrays for each named variable
            $(let mut $var_name = [0.0; $width];)*

            // Loop through all lanes, extracting from different input arrays at different indices
            for j in 0..$width {
                unsafe {
                    let index = $indices[j];
                    $($var_name[j] = *$ptr_array[j].add(index);)*
                }
            }

            // Return raw arrays
            ($($var_name),*)
        }
    };
}

#[macro_export]
macro_rules! extract_simd_inputs_at_index {
    ($index:expr, $width:expr, $($var_name:ident @ $ptr_array:ident),*) => {
        {
            // Declare arrays for each named variable
            $(let mut $var_name = [0.0; $width];)*

            // Single loop through all assets, extracting from different input arrays
            for j in 0..$width {
                unsafe {
                    $($var_name[j] = *$ptr_array[j].add($index);)*
                }
            }

            // Convert to SIMD vectors and return as tuple
            ($(Simd::from_array($var_name)),*)
        }
    };
}

#[macro_export]
macro_rules! extract_simd_inputs_at_index_splat {
    ($index:expr, $width:expr, $($var_name:ident @ $ptr_array:ident),*) => {
        {
            (
                $(
                    unsafe {
                        Simd::splat(*$ptr_array[0].add($index))
                    }
                ),*
            )
        }
    };
}
#[macro_export]
macro_rules! extract_array_inputs_at_index {
    ($index:expr, $width:expr, $($var_name:ident @ $ptr_array:ident),*) => {
        {
            // Declare arrays for each named variable
            $(let mut $var_name = [0.0; $width];)*

            // Single loop through all assets, extracting from different input arrays
            for j in 0..$width {
                unsafe {
                    $($var_name[j] = *$ptr_array[j].add($index);)*
                }
            }

            // Return raw arrays
            ($($var_name),*)
        }
    };
}
#[macro_export]
macro_rules! store_simd_optional_outputs {
    ($index:expr, $width:expr, $($want_flag:expr, $ptr_array:ident => $simd_value:expr),*) => {
        $(
            if $want_flag {
                for j in 0..$width {
                    unsafe {
                        *$ptr_array[j].add($index) = $simd_value[j];
                    }
                }
            }
        )*
    };
}
#[macro_export]
macro_rules! store_simd_optional_outputs_corrected {
    ($index:expr, $width:expr, $($want_flag:expr, $ptr_array:ident => corrected($simd_value:expr, $multiplier:expr)),*) => {
        $(
            if $want_flag {
                let values = ($simd_value * $multiplier);//.to_array();
                for j in 0..$width {
                    unsafe {
                        *$ptr_array[j].add($index) = values[j];
                    }
                }
            }
        )*
    };
}

/// Extract multiple values from the same input array at different indices
/// Usage: extract_simd_multiple_at_indices!(N, ptr_array, idx1, idx2, idx3, ...) -> (simd1, simd2, simd3, ...)
#[macro_export]
macro_rules! extract_simd_at_indices {
    ($width:expr, $ptr_array:ident, $($var_name:ident @ $index:expr),*) => {
        {
            // Declare arrays for each named variable
            $(let mut $var_name = [0.0; $width];)*

            // Single loop through all assets, extracting all needed values
            for j in 0..$width {
                unsafe {
                    let ptr = $ptr_array[j];
                    $($var_name[j] = *ptr.add($index);)*
                }
            }

            // Convert to SIMD vectors and return as tuple
            ($(Simd::from_array($var_name)),*)
        }
    };
}

#[macro_export]
macro_rules! extract_array_at_indices {
    ($width:expr, $ptr_array:ident, $($var_name:ident @ $index:expr),*) => {
        {
            // Declare arrays for each named variable
            $(let mut $var_name = [0.0; $width];)*

            // Single loop through all assets, extracting all needed values
            for j in 0..$width {
                unsafe {
                    let ptr = $ptr_array[j];
                    $($var_name[j] = *ptr.add($index);)*
                }
            }

            // Return raw arrays
            ($($var_name),*)
        }
    };
}

#[macro_export]
macro_rules! extract_simd_at_indices_array {
    ($width:expr, $ptr_array:ident, $($var_name:ident @ $indices_simd:expr),*) => {
        {
            // Declare arrays for each named variable
            $(let mut $var_name = [0.0; $width];)*

            // Loop through each lane (asset) first for better cache locality
            for j in 0..$width {
                unsafe {
                    let ptr = $ptr_array[j];
                    // Extract all needed values for this lane/asset at once
                    $(
                        //let indices_array = $indices_simd.to_array();
                        $var_name[j] = *ptr.add($indices_simd[j]);
                    )*
                }
            }

            // Convert to SIMD vectors and return as tuple
            ($(Simd::from_array($var_name)),*)
        }
    };
}
#[macro_export]
macro_rules! write_simd_at_indices {
    ($width:expr, $index:expr, $($ptr_array:ident => $simd_value:expr),*) => {
        unsafe {
            $(
                for j in 0..$width {
                    *$ptr_array[j].add($index) = $simd_value[j];
                }
            )*
        }
    };
}


#[macro_export]
macro_rules! extract_simd_from_ptrs {
    // Usage:
    // let (a, b, c, d) = crate::extract_simd_from_ptrs!(N, a_ptrs, b_ptrs, c_ptrs, d_ptrs);
    ($width:expr, $($var_name:ident @ $ptr_array:ident),+) => {
        {
            $(
                let mut $var_name = [0.0; $width];
            )+

            for j in 0..$width {
                unsafe {
                    $(
                        $var_name[j] = *$ptr_array[j];
                    )+
                }
            }

            ($(Simd::from_array($var_name)),+)
        }
    };
}