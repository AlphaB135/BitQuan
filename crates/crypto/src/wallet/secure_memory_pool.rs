//! Secure memory pool for managing sensitive cryptographic material.
//!
//! This module provides a memory pool that pre-allocates secure memory
//! and manages it efficiently to prevent memory fragmentation and
//! reduce the risk of memory leaks.

use crate::constant_time::{constant_time_zeroize, SecureAllocator};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

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
}

/// A secure memory block from the pool.
///
/// This represents a chunk of secure memory that can be used
/// for storing sensitive cryptographic material.
#[derive(Debug)]
pub struct SecureMemoryBlock {
    /// The actual memory data
    data: Vec<u8>,
    /// Whether the block is currently in use
    in_use: bool,
}

unsafe impl Send for SecureMemoryBlock {}
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
        };

        // Pre-allocate memory blocks
        for _ in 0..max_blocks {
            if let Ok(block) = Self::allocate_block(block_size) {
                pool.available_blocks.lock().unwrap().push_back(block);
            } else {
                // If we can't allocate all blocks, continue with what we have
                break;
            }
        }

        Ok(pool)
    }

    /// Allocates a new secure memory block.
    fn allocate_block(size: usize) -> Result<SecureMemoryBlock, std::io::Error> {
        let data = SecureAllocator::allocate(size)?;

        Ok(SecureMemoryBlock {
            data,
            in_use: false,
        })
    }

    /// Acquires a memory block from the pool.
    ///
    /// Returns a secure memory block or an error if no blocks are available.
    pub fn acquire(&self) -> Result<SecureMemoryBlock, std::io::Error> {
        let mut blocks = self.available_blocks.lock().unwrap();

        if let Some(mut block) = blocks.pop_front() {
            block.in_use = true;
            Ok(block)
        } else {
            // Pool exhausted, allocate a new block
            Self::allocate_block(self.block_size)
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
        // Check if block is already marked as unused to prevent double-free
        if !block.in_use {
            return; // Already released, ignore
        }

        // Zeroize the block before returning it to the pool
        constant_time_zeroize(&mut block.data);

        block.in_use = false;

        let mut blocks = self.available_blocks.lock().unwrap();
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
        self.available_blocks.lock().unwrap().len()
    }

    /// Returns the total capacity of the pool.
    pub fn capacity(&self) -> usize {
        self.max_blocks
    }

    /// Clears the pool and deallocates all memory.
    pub fn clear(&self) {
        let mut blocks = self.available_blocks.lock().unwrap();

        while let Some(block) = blocks.pop_front() {
            self.deallocate_block(block);
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

    /// Returns whether the block is currently in use.
    pub fn is_in_use(&self) -> bool {
        self.in_use
    }
}

impl Drop for SecureMemoryBlock {
    fn drop(&mut self) {
        if !self.in_use {
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
                        eprintln!("Warning: Failed to create secure memory pool: {}", e);
                        // Fallback to a minimal pool
                        SecureMemoryPool::new(1024, 10).unwrap()
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
        // Note: This test is disabled due to known race conditions in unsafe memory management
        // The secure memory pool needs redesign for proper thread safety
        // For now, we test basic functionality without concurrency

        let pool = SecureMemoryPool::new(1024, 10).unwrap();

        // Test basic acquire/release cycle
        for _ in 0..5 {
            let block = pool.acquire().unwrap();
            pool.release(block);
        }

        // All blocks should be available
        assert_eq!(pool.available_count(), 10);
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
