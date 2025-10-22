/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Lock Module Tests
//!
//! This module organizes all tests for the lock module,
//! including tests for traits and their implementations.

// Trait tests
mod lock_tests;
mod read_write_lock_tests;
mod async_lock_tests;
mod async_read_write_lock_tests;

// Implementation tests
mod arc_mutex_tests;
mod arc_rw_lock_tests;
mod arc_async_mutex_tests;
mod arc_async_rw_lock_tests;

