//! Virtual Memory Circular Buffer
//!
//! This module provides a circular buffer implementation using virtual memory mapping
//! to create the illusion of contiguous memory across wrap boundaries. This enables
//! efficient SIMD operations on sliding windows without copying data.

pub mod generic_buffer;
pub mod mirror_buffer;
pub mod ring_buffer;
pub mod simd_buffer;
