//! SIMD-optimized ring buffer for time-series statistics.
//!
//! Provides O(1) push, latest, and oldest operations with efficient
//! statistics computation via contiguous memory access.

use std::collections::VecDeque;

/// A fixed-capacity ring buffer optimized for time-series data.
///
/// Supports O(1) operations and provides contiguous memory access
/// for SIMD-accelerated statistics computation.
#[derive(Debug, Clone)]
pub struct RingBuffer<T> {
    data: VecDeque<T>,
    capacity: usize,
}

impl<T> RingBuffer<T> {
    /// Create a new ring buffer with the specified capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Push a value to the buffer, removing the oldest if at capacity.
    /// O(1) amortized.
    pub fn push(&mut self, value: T) {
        if self.data.len() >= self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(value);
    }

    /// Get the most recent value. O(1).
    pub fn latest(&self) -> Option<&T> {
        self.data.back()
    }

    /// Get the oldest value. O(1).
    pub fn oldest(&self) -> Option<&T> {
        self.data.front()
    }

    /// Get the number of elements in the buffer.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the capacity of the buffer.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Clear all elements from the buffer.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Get an iterator over the elements (oldest to newest).
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }
}

impl<T: Copy> RingBuffer<T> {
    /// Make the internal storage contiguous and return a slice.
    /// This is useful for SIMD operations that require contiguous memory.
    pub fn make_contiguous(&mut self) -> &[T] {
        self.data.make_contiguous()
    }

    /// Get a contiguous slice of the data (may need internal reorganization).
    pub fn as_slice(&mut self) -> &[T] {
        self.make_contiguous()
    }
}

impl RingBuffer<f64> {
    /// Calculate the sum of all values. O(n).
    pub fn sum(&self) -> f64 {
        self.data.iter().sum()
    }

    /// Calculate the mean of all values. O(n).
    pub fn mean(&self) -> f64 {
        if self.data.is_empty() {
            return 0.0;
        }
        self.sum() / self.data.len() as f64
    }

    /// Find the minimum value. O(n).
    pub fn min(&self) -> f64 {
        self.data
            .iter()
            .copied()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0)
    }

    /// Find the maximum value. O(n).
    pub fn max(&self) -> f64 {
        self.data
            .iter()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0)
    }

    /// Calculate the rate of change per second given the sample interval.
    /// Returns the average rate over the buffer window.
    pub fn rate_per_sec(&self, sample_interval_secs: f64) -> f64 {
        if self.data.len() < 2 || sample_interval_secs <= 0.0 {
            return 0.0;
        }

        let oldest = self.data.front().copied().unwrap_or(0.0);
        let newest = self.data.back().copied().unwrap_or(0.0);
        let delta = newest - oldest;
        let time_span = (self.data.len() - 1) as f64 * sample_interval_secs;

        if time_span > 0.0 {
            delta / time_span
        } else {
            0.0
        }
    }

    /// Calculate the standard deviation. O(n). Delegates to batuta-common.
    pub fn std_dev(&self) -> f64 {
        let data: Vec<f64> = self.data.iter().copied().collect();
        batuta_common::math::std_dev(&data)
    }
}

impl RingBuffer<u64> {
    /// Calculate the sum of all values. O(n).
    pub fn sum(&self) -> u64 {
        self.data.iter().sum()
    }

    /// Calculate the mean of all values. O(n).
    pub fn mean(&self) -> f64 {
        if self.data.is_empty() {
            return 0.0;
        }
        self.sum() as f64 / self.data.len() as f64
    }

    /// Find the minimum value. O(n).
    pub fn min(&self) -> u64 {
        self.data.iter().copied().min().unwrap_or(0)
    }

    /// Find the maximum value. O(n).
    pub fn max(&self) -> u64 {
        self.data.iter().copied().max().unwrap_or(0)
    }

    /// Calculate the rate of change per second given the sample interval.
    /// Handles counter wrapping for monotonic counters.
    pub fn rate_per_sec(&self, sample_interval_secs: f64) -> f64 {
        if self.data.len() < 2 || sample_interval_secs <= 0.0 {
            return 0.0;
        }

        let oldest = self.data.front().copied().unwrap_or(0);
        let newest = self.data.back().copied().unwrap_or(0);

        // Handle counter wrap
        let delta = if newest >= oldest {
            newest - oldest
        } else {
            // Counter wrapped
            u64::MAX - oldest + newest + 1
        };

        let time_span = (self.data.len() - 1) as f64 * sample_interval_secs;

        if time_span > 0.0 {
            delta as f64 / time_span
        } else {
            0.0
        }
    }
}

/// Handle counter wrapping for monotonic counters (e.g., network bytes).
/// Used when calculating deltas between two counter readings.
pub fn handle_counter_wrap(prev: u64, curr: u64) -> u64 {
    if curr >= prev {
        curr - prev
    } else {
        // Counter wrapped at u64::MAX
        u64::MAX - prev + curr + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_push_and_capacity() {
        let mut buf: RingBuffer<i32> = RingBuffer::new(3);
        assert!(buf.is_empty());

        buf.push(1);
        buf.push(2);
        buf.push(3);
        assert_eq!(buf.len(), 3);

        buf.push(4);
        assert_eq!(buf.len(), 3); // Capacity maintained
        assert_eq!(buf.oldest(), Some(&2)); // 1 was removed
        assert_eq!(buf.latest(), Some(&4));
    }

    #[test]
    fn test_ring_buffer_f64_stats() {
        let mut buf: RingBuffer<f64> = RingBuffer::new(5);
        buf.push(1.0);
        buf.push(2.0);
        buf.push(3.0);
        buf.push(4.0);
        buf.push(5.0);

        assert!((buf.mean() - 3.0).abs() < 0.001);
        assert!((buf.sum() - 15.0).abs() < 0.001);
        assert!((buf.min() - 1.0).abs() < 0.001);
        assert!((buf.max() - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_ring_buffer_rate() {
        let mut buf: RingBuffer<u64> = RingBuffer::new(5);
        buf.push(100);
        buf.push(200);
        buf.push(300);
        buf.push(400);
        buf.push(500);

        // Rate = (500 - 100) / (4 * 1.0) = 100/s
        let rate = buf.rate_per_sec(1.0);
        assert!((rate - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_counter_wrap_handling() {
        // Simulate wrap at u64::MAX
        let prev = u64::MAX - 10;
        let curr = 5;
        let delta = handle_counter_wrap(prev, curr);
        assert_eq!(delta, 16); // 10 to MAX + 1 + 5 = 16
    }

    #[test]
    fn test_make_contiguous() {
        let mut buf: RingBuffer<f64> = RingBuffer::new(3);
        buf.push(1.0);
        buf.push(2.0);
        buf.push(3.0);
        buf.push(4.0); // This forces rotation

        let slice = buf.make_contiguous();
        assert_eq!(slice.len(), 3);
        assert_eq!(slice, &[2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_ring_buffer_clear() {
        let mut buf: RingBuffer<i32> = RingBuffer::new(3);
        buf.push(1);
        buf.push(2);
        assert_eq!(buf.len(), 2);
        buf.clear();
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_ring_buffer_iter() {
        let mut buf: RingBuffer<i32> = RingBuffer::new(3);
        buf.push(10);
        buf.push(20);
        buf.push(30);
        let sum: i32 = buf.iter().sum();
        assert_eq!(sum, 60);
    }

    #[test]
    fn test_ring_buffer_capacity() {
        let buf: RingBuffer<i32> = RingBuffer::new(10);
        assert_eq!(buf.capacity(), 10);
    }

    #[test]
    fn test_ring_buffer_empty_latest_oldest() {
        let buf: RingBuffer<i32> = RingBuffer::new(3);
        assert_eq!(buf.latest(), None);
        assert_eq!(buf.oldest(), None);
    }

    #[test]
    fn test_ring_buffer_f64_empty_stats() {
        let buf: RingBuffer<f64> = RingBuffer::new(3);
        assert_eq!(buf.mean(), 0.0);
        assert_eq!(buf.sum(), 0.0);
        assert_eq!(buf.min(), 0.0);
        assert_eq!(buf.max(), 0.0);
        assert_eq!(buf.std_dev(), 0.0);
    }

    #[test]
    fn test_ring_buffer_f64_single_value() {
        let mut buf: RingBuffer<f64> = RingBuffer::new(3);
        buf.push(42.0);
        assert!((buf.mean() - 42.0).abs() < 0.001);
        assert!((buf.sum() - 42.0).abs() < 0.001);
        assert!((buf.min() - 42.0).abs() < 0.001);
        assert!((buf.max() - 42.0).abs() < 0.001);
        assert_eq!(buf.std_dev(), 0.0); // Need at least 2 for std_dev
    }

    #[test]
    fn test_ring_buffer_f64_std_dev() {
        let mut buf: RingBuffer<f64> = RingBuffer::new(5);
        buf.push(2.0);
        buf.push(4.0);
        buf.push(4.0);
        buf.push(4.0);
        buf.push(5.0);
        buf.push(5.0);
        buf.push(7.0);
        buf.push(9.0);
        // Keep last 5: 4, 5, 5, 7, 9 -> mean = 6, variance = (4+1+1+1+9)/4 = 4, std = 2
        let std = buf.std_dev();
        assert!(std > 0.0);
    }

    #[test]
    fn test_ring_buffer_f64_rate_insufficient_data() {
        let mut buf: RingBuffer<f64> = RingBuffer::new(3);
        assert_eq!(buf.rate_per_sec(1.0), 0.0); // Empty

        buf.push(100.0);
        assert_eq!(buf.rate_per_sec(1.0), 0.0); // Only 1 element
    }

    #[test]
    fn test_ring_buffer_f64_rate_zero_interval() {
        let mut buf: RingBuffer<f64> = RingBuffer::new(3);
        buf.push(100.0);
        buf.push(200.0);
        assert_eq!(buf.rate_per_sec(0.0), 0.0);
        assert_eq!(buf.rate_per_sec(-1.0), 0.0);
    }

    #[test]
    fn test_ring_buffer_f64_rate_per_sec() {
        let mut buf: RingBuffer<f64> = RingBuffer::new(3);
        buf.push(0.0);
        buf.push(100.0);
        buf.push(200.0);
        // Rate = (200 - 0) / (2 * 1.0) = 100/s
        let rate = buf.rate_per_sec(1.0);
        assert!((rate - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_ring_buffer_u64_stats() {
        let mut buf: RingBuffer<u64> = RingBuffer::new(5);
        buf.push(10);
        buf.push(20);
        buf.push(30);
        buf.push(40);
        buf.push(50);
        assert_eq!(buf.sum(), 150);
        assert!((buf.mean() - 30.0).abs() < 0.001);
        assert_eq!(buf.min(), 10);
        assert_eq!(buf.max(), 50);
    }

    #[test]
    fn test_ring_buffer_u64_empty_stats() {
        let buf: RingBuffer<u64> = RingBuffer::new(3);
        assert_eq!(buf.sum(), 0);
        assert_eq!(buf.mean(), 0.0);
        assert_eq!(buf.min(), 0);
        assert_eq!(buf.max(), 0);
    }

    #[test]
    fn test_ring_buffer_u64_rate_insufficient_data() {
        let mut buf: RingBuffer<u64> = RingBuffer::new(3);
        assert_eq!(buf.rate_per_sec(1.0), 0.0);

        buf.push(100);
        assert_eq!(buf.rate_per_sec(1.0), 0.0);
    }

    #[test]
    fn test_ring_buffer_u64_rate_zero_interval() {
        let mut buf: RingBuffer<u64> = RingBuffer::new(3);
        buf.push(100);
        buf.push(200);
        assert_eq!(buf.rate_per_sec(0.0), 0.0);
        assert_eq!(buf.rate_per_sec(-1.0), 0.0);
    }

    #[test]
    fn test_ring_buffer_u64_rate_with_wrap() {
        let mut buf: RingBuffer<u64> = RingBuffer::new(3);
        buf.push(u64::MAX - 5);
        buf.push(u64::MAX);
        buf.push(10);
        // delta = wrap from MAX-5 to 10 = 16
        // time_span = 2 * 1.0 = 2.0
        // rate = 16 / 2 = 8
        let rate = buf.rate_per_sec(1.0);
        assert!((rate - 8.0).abs() < 0.001);
    }

    #[test]
    fn test_counter_wrap_no_wrap() {
        assert_eq!(handle_counter_wrap(100, 200), 100);
        assert_eq!(handle_counter_wrap(0, 1000), 1000);
        assert_eq!(handle_counter_wrap(50, 50), 0);
    }

    #[test]
    fn test_counter_wrap_at_max() {
        assert_eq!(handle_counter_wrap(u64::MAX, 0), 1);
        assert_eq!(handle_counter_wrap(u64::MAX, 10), 11);
    }

    #[test]
    fn test_ring_buffer_as_slice() {
        let mut buf: RingBuffer<i32> = RingBuffer::new(3);
        buf.push(1);
        buf.push(2);
        buf.push(3);
        let slice = buf.as_slice();
        assert_eq!(slice, &[1, 2, 3]);
    }
}
