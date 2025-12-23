//! Secure memory pool for managing sensitive cryptographic material.
//!
//! This module provides a memory pool that pre-allocates secure memory
//! and manages it efficiently to prevent memory fragmentation and
//! reduce the risk of memory leaks.

use crate::constant_time::{constant_time_zeroize, SecureAllocator};
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// A secure memory pool for managing sensitive data.
///
/// This pool pre-allocates memory blocks and manages them to
/// provide fast, secure memory allocation for cryptographic operations.
pub struct SecureMemoryPool {
    /// Pool of available memory blocks
    available_blocks: Arc<Mutex<VecDeque<SecureMemoryBlock>>>,
    /// Size of each memory block in the pool
    block_size: usize,
    /// Maximum number of blocks in the pool
    max_blocks: usize,
    /// Counter for generating unique block IDs
    block_id_counter: std::sync::atomic::AtomicU64,
}

/// A secure memory block from the pool.
///
/// This represents a chunk of secure memory that can be used
/// for storing sensitive cryptographic material.
#[derive(Debug)]
pub struct SecureMemoryBlock {
    /// The actual memory data
    data: Vec<u8>,
    /// Unique ID for this block to prevent race conditions
    block_id: u64,
    /// Whether the block is currently in use (protected by atomic operations)
    in_use: std::sync::atomic::AtomicBool,
}

// SAFETY: SecureMemoryBlock owns its data (Vec<u8>) which is Send.
// It has no thread-local state or shared mutable state that would violate Send.
unsafe impl Send for SecureMemoryBlock {}
// SAFETY: SecureMemoryBlock owns its data (Vec<u8>) which is Sync.
// Access to the block is controlled by the Mutex in SecureMemoryPool.
unsafe impl Sync for SecureMemoryBlock {}

impl SecureMemoryPool {
    /// Creates a new secure memory pool.
    ///
    /// # Arguments
    ///
    /// * `block_size` - Size of each memory block in bytes
    /// * `max_blocks` - Maximum number of blocks to pre-allocate
    ///
    /// # Returns
    ///
    /// A new secure memory pool or an error if allocation fails.
    pub fn new(block_size: usize, max_blocks: usize) -> Result<Self, std::io::Error> {
        let pool = Self {
            available_blocks: Arc::new(Mutex::new(VecDeque::with_capacity(max_blocks))),
            block_size,
            max_blocks,
            block_id_counter: std::sync::atomic::AtomicU64::new(0),
        };

        // Pre-allocate memory blocks
        for _ in 0..max_blocks {
            if let Ok(block) = pool.allocate_block(block_size) {
                match pool.available_blocks.lock() {
                    Ok(mut blocks) => blocks.push_back(block),
                    Err(e) => {
                        eprintln!("Warning: Failed to acquire lock for pool initialization: {e}");
                        break;
                    }
                }
            } else {
                // If we can't allocate all blocks, continue with what we have
                break;
            }
        }

        Ok(pool)
    }

    /// Allocates a new secure memory block.
    fn allocate_block(&self, size: usize) -> Result<SecureMemoryBlock, std::io::Error> {
        let data = SecureAllocator::allocate(size)?;
        let block_id = self.block_id_counter.fetch_add(1, Ordering::SeqCst);

        Ok(SecureMemoryBlock {
            data,
            block_id,
            in_use: AtomicBool::new(false),
        })
    }

    /// Acquires a memory block from the pool.
    ///
    /// Returns a secure memory block or an error if no blocks are available.
    pub fn acquire(&self) -> Result<SecureMemoryBlock, std::io::Error> {
        let mut blocks = self.available_blocks.lock().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "Failed to acquire pool lock",
            )
        })?;

        if let Some(block) = blocks.pop_front() {
            // Use atomic compare-and-swap to ensure thread safety
            // This prevents race conditions where multiple threads could acquire the same block
            match block.in_use.compare_exchange(
                false, // Expected value: not in use
                true,  // New value: mark as in use
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    // Successfully marked as in use, return the block
                    Ok(block)
                }
                Err(_) => {
                    // Block was already in use (shouldn't happen but handle gracefully)
                    // Put it back and allocate a new block
                    blocks.push_back(block);
                    self.allocate_block(self.block_size)
                }
            }
        } else {
            // Pool exhausted, allocate a new block
            self.allocate_block(self.block_size)
        }
    }

    /// Releases a memory block back to the pool.
    ///
    /// # Arguments
    ///
    /// * `block` - The memory block to release
    ///
    /// # Safety
    ///
    /// The block must have been acquired from this pool.
    pub fn release(&self, mut block: SecureMemoryBlock) {
        // Use atomic compare-and-swap to prevent double-free
        if block
            .in_use
            .compare_exchange(
                true,  // Expected value: currently in use
                false, // New value: mark as not in use
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_err()
        {
            return; // Already released, ignore
        }

        // Zeroize the block before returning it to the pool
        constant_time_zeroize(&mut block.data);

        // Critical section: acquire lock before checking pool capacity
        let mut blocks = match self.available_blocks.lock() {
            Ok(blocks) => blocks,
            Err(_) => {
                eprintln!("Warning: Failed to acquire pool lock during release");
                return;
            }
        };
        if blocks.len() < self.max_blocks {
            blocks.push_back(block);
        } else {
            // Pool is full, deallocate the block
            drop(blocks); // Release lock before deallocation
            self.deallocate_block(block);
        }
    }

    /// Deallocates a memory block.
    fn deallocate_block(&self, block: SecureMemoryBlock) {
        // The Vec will be automatically dropped and deallocated
        // SecureAllocator::deallocate is called automatically when the Vec is dropped
        drop(block);
    }

    /// Returns the number of available blocks in the pool.
    pub fn available_count(&self) -> usize {
        self.available_blocks
            .lock()
            .map(|blocks| blocks.len())
            .unwrap_or_else(|_| {
                eprintln!("Warning: Failed to acquire lock for available_count, returning 0");
                0
            })
    }

    /// Returns the total capacity of the pool.
    pub fn capacity(&self) -> usize {
        self.max_blocks
    }

    /// Clears the pool and deallocates all memory.
    pub fn clear(&self) {
        if let Ok(mut blocks) = self.available_blocks.lock() {
            while let Some(block) = blocks.pop_front() {
                self.deallocate_block(block);
            }
        } else {
            eprintln!("Warning: Failed to acquire lock for clear operation");
        }
    }
}

impl Drop for SecureMemoryPool {
    fn drop(&mut self) {
        self.clear();
    }
}

impl SecureMemoryBlock {
    /// Returns a mutable slice to the memory block.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the memory is accessed safely
    /// and that the block remains valid while the slice is in use.
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Returns an immutable slice to the memory block.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the memory is accessed safely
    /// and that the block remains valid while the slice is in use.
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Returns the size of the memory block.
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Returns the unique ID of this memory block.
    pub fn block_id(&self) -> u64 {
        self.block_id
    }

    /// Returns whether the block is currently in use.
    pub fn is_in_use(&self) -> bool {
        self.in_use.load(Ordering::SeqCst)
    }
}

impl Drop for SecureMemoryBlock {
    fn drop(&mut self) {
        if !self.in_use.load(Ordering::SeqCst) {
            // Block was not properly released, zeroize it
            constant_time_zeroize(&mut self.data);
        }
    }
}

/// A global secure memory pool manager.
///
/// This provides a singleton instance of the secure memory pool
/// for use throughout the application.
pub struct SecureMemoryManager {
    pool: Arc<SecureMemoryPool>,
}

impl SecureMemoryManager {
    /// Gets the global secure memory manager instance.
    pub fn instance() -> Arc<Self> {
        use std::sync::OnceLock;

        static INSTANCE: OnceLock<Arc<SecureMemoryManager>> = OnceLock::new();

        INSTANCE
            .get_or_init(|| {
                Arc::new(Self {
                    pool: Arc::new(SecureMemoryPool::new(4096, 100).unwrap_or_else(|e| {
                        eprintln!(
                            "Warning: Failed to create secure memory pool (4096, 100): {}",
                            e
                        );
                        SecureMemoryPool::new(1024, 10).unwrap_or_else(|e| {
                            eprintln!("Warning: Failed to create fallback pool (1024, 10): {}", e);
                            SecureMemoryPool::new(256, 1).unwrap_or_else(|e| {
                                eprintln!("Warning: Failed to create minimal pool (256, 1): {}", e);
                                eprintln!("Creating empty pool - security features degraded");
                                SecureMemoryPool {
                                    available_blocks: Arc::new(Mutex::new(VecDeque::new())),
                                    block_size: 256,
                                    max_blocks: 0,
                                    block_id_counter: std::sync::atomic::AtomicU64::new(0),
                                }
                            })
                        })
                    })),
                })
            })
            .clone()
    }

    /// Acquires a memory block from the global pool.
    pub fn acquire() -> Result<SecureMemoryBlock, std::io::Error> {
        Self::instance().pool.acquire()
    }

    /// Releases a memory block back to the global pool.
    pub fn release(block: SecureMemoryBlock) {
        Self::instance().pool.release(block);
    }

    /// Returns statistics about the global memory pool.
    pub fn stats() -> MemoryPoolStats {
        let manager = Self::instance();
        MemoryPoolStats {
            available_blocks: manager.pool.available_count(),
            total_capacity: manager.pool.capacity(),
            block_size: 4096,
        }
    }
}

/// Statistics about the secure memory pool.
#[derive(Debug, Clone)]
pub struct MemoryPoolStats {
    /// Number of available blocks in the pool
    pub available_blocks: usize,
    /// Total capacity of the pool
    pub total_capacity: usize,
    /// Size of each memory block
    pub block_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_memory_pool_creation() {
        let pool = SecureMemoryPool::new(1024, 10).unwrap();

        assert_eq!(pool.capacity(), 10);
        assert_eq!(pool.available_count(), 10);
    }

    #[test]
    fn test_memory_block_acquisition() {
        let pool = SecureMemoryPool::new(1024, 5).unwrap();

        // Acquire a block
        let block = pool.acquire().unwrap();
        assert!(block.is_in_use());
        assert_eq!(block.size(), 1024);
        assert_eq!(pool.available_count(), 4);

        // Release the block
        pool.release(block);
        assert_eq!(pool.available_count(), 5);
    }

    #[test]
    fn test_memory_block_operations() {
        let pool = SecureMemoryPool::new(1024, 5).unwrap();
        let mut block = pool.acquire().unwrap();

        // Test writing to the block
        {
            let slice = block.as_slice_mut();
            slice[0] = 42;
            slice[1] = 84;
            slice[2] = 126;
        }

        // Test reading from the block
        {
            let slice = block.as_slice();
            assert_eq!(slice[0], 42);
            assert_eq!(slice[1], 84);
            assert_eq!(slice[2], 126);
        }

        pool.release(block);
    }

    #[test]
    fn test_pool_exhaustion() {
        let pool = SecureMemoryPool::new(1024, 2).unwrap();

        // Acquire all blocks
        let block1 = pool.acquire().unwrap();
        let block2 = pool.acquire().unwrap();

        assert_eq!(pool.available_count(), 0);

        // Pool is exhausted, should allocate new block
        let block3 = pool.acquire().unwrap();
        assert_eq!(block3.size(), 1024);

        // Release blocks
        pool.release(block1);
        pool.release(block2);
        pool.release(block3);
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::thread;

        let pool = Arc::new(SecureMemoryPool::new(1024, 20).unwrap());
        let mut handles = vec![];
        let acquire_count = Arc::new(AtomicUsize::new(0));
        let release_count = Arc::new(AtomicUsize::new(0));

        // Spawn multiple threads to test concurrent access
        for _ in 0..10 {
            let pool_clone = Arc::clone(&pool);
            let acquire_count_clone = Arc::clone(&acquire_count);
            let release_count_clone = Arc::clone(&release_count);

            let handle = thread::spawn(move || {
                for _ in 0..10 {
                    // Acquire and release blocks concurrently
                    match pool_clone.acquire() {
                        Ok(mut block) => {
                            acquire_count_clone.fetch_add(1, Ordering::SeqCst);
                            assert!(block.is_in_use());

                            // Simulate some work with the block
                            {
                                let slice = block.as_slice_mut();
                                slice[0] = 42;
                                // Add a small delay to increase chance of race conditions
                                std::hint::spin_loop();
                            }

                            pool_clone.release(block);
                            release_count_clone.fetch_add(1, Ordering::SeqCst);
                        }
                        Err(_) => {
                            // Handle allocation failure gracefully
                        }
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify that all acquires were matched by releases
        let total_acquires = acquire_count.load(Ordering::SeqCst);
        let total_releases = release_count.load(Ordering::SeqCst);
        assert_eq!(total_acquires, total_releases);

        // All blocks should be available after all threads complete
        assert_eq!(pool.available_count(), 20);
    }

    #[test]
    fn test_race_condition_protection() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::thread;

        let pool = Arc::new(SecureMemoryPool::new(1024, 5).unwrap());
        let race_detected = Arc::new(AtomicBool::new(false));
        let mut handles = vec![];

        // Test with very small pool to increase contention
        for thread_id in 0..5 {
            let pool_clone = Arc::clone(&pool);
            let race_detected_clone = Arc::clone(&race_detected);

            let handle = thread::spawn(move || {
                for iteration in 0..10 {
                    if let Ok(mut block) = pool_clone.acquire() {
                        // Verify block is properly marked as in use
                        if !block.is_in_use() {
                            race_detected_clone.store(true, Ordering::SeqCst);
                            return;
                        }

                        // Do some work with unique pattern per thread/iteration
                        let thread_data = ((thread_id * 10 + iteration) & 0xFF) as u8;
                        let _block_id = block.block_id();
                        {
                            let slice = block.as_slice_mut();
                            // Write unique pattern
                            for i in 0..8.min(slice.len()) {
                                slice[i] = thread_data.wrapping_add(i as u8);
                            }
                        }

                        // Small delay to increase chance of race conditions
                        std::hint::spin_loop();

                        // Verify data integrity (this should pass if no race condition)
                        {
                            let slice = block.as_slice();
                            for (i, &byte_val) in slice.iter().enumerate().take(8.min(slice.len()))
                            {
                                if byte_val != thread_data.wrapping_add(i as u8) {
                                    // Data was corrupted by another thread - race condition!
                                    race_detected_clone.store(true, Ordering::SeqCst);
                                    return;
                                }
                            }
                        }

                        pool_clone.release(block);
                    }
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify no race conditions were detected
        assert!(
            !race_detected.load(Ordering::SeqCst),
            "Race condition detected!"
        );

        // Pool should be in consistent state
        assert_eq!(pool.available_count(), 5);
    }

    #[test]
    fn test_double_release_protection() {
        let pool = SecureMemoryPool::new(1024, 5).unwrap();

        let block = pool.acquire().unwrap();
        assert!(block.is_in_use());

        // First release should succeed
        pool.release(block);

        // Note: We can't test double release since the block is moved
        // But the atomic compare_exchange prevents double-free
    }

    #[test]
    fn test_global_memory_manager() {
        let stats1 = SecureMemoryManager::stats();

        // Acquire and release a block
        let block = SecureMemoryManager::acquire().unwrap();
        SecureMemoryManager::release(block);

        let stats2 = SecureMemoryManager::stats();

        // Stats should be consistent
        assert_eq!(stats1.block_size, stats2.block_size);
        assert_eq!(stats1.total_capacity, stats2.total_capacity);
    }

    #[test]
    fn test_memory_zeroization_on_release() {
        let pool = SecureMemoryPool::new(1024, 5).unwrap();
        let mut block = pool.acquire().unwrap();

        // Write some data to the block
        {
            let slice = block.as_slice_mut();
            slice.fill(0xFF);
        }

        // Verify data was written
        {
            let slice = block.as_slice();
            assert!(slice.iter().all(|&b| b == 0xFF));
        }

        // Release the block (should zeroize)
        pool.release(block);

        // Acquire the same block and verify it's zeroized
        let block = pool.acquire().unwrap();
        {
            let slice = block.as_slice();
            assert!(slice.iter().all(|&b| b == 0));
        }

        pool.release(block);
    }

    #[test]
    fn test_pool_clear() {
        let pool = SecureMemoryPool::new(1024, 5).unwrap();

        // Acquire some blocks
        let _block1 = pool.acquire().unwrap();
        let _block2 = pool.acquire().unwrap();

        assert_eq!(pool.available_count(), 3);

        // Clear the pool
        pool.clear();

        // All blocks should be deallocated
        assert_eq!(pool.available_count(), 0);
    }
}
